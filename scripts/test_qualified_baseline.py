import json
from pathlib import Path
import tempfile
import unittest

import qualified_baseline as baseline


class QualifiedBaselineTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.metadata = dict.fromkeys(baseline.QUALIFIERS, "fixture")
        self.metadata.update(commit="a" * 40, measurement_kind="deterministic_harness",
                             build_mode="release", scale=1, refresh_hz=60)
        self.row = dict(type="radiant_perf", scenario="edit", iterations=10,
                        avg_us=2, p50_us=1, p95_us=3, p99_us=4, layout_count=10)

    def pack(self, name, meta=None, row=None):
        meta_path, rows_path, output = [self.root / (name + suffix)
                                       for suffix in (".meta", ".jsonl", ".pack")]
        meta_path.write_text(json.dumps(self.metadata if meta is None else meta))
        rows_path.write_text(json.dumps(self.row if row is None else row) + "\n")
        baseline.seal(meta_path, rows_path, output)
        return output

    def test_historical_tail_regression_cannot_hide_behind_average(self):
        old = self.pack("old")
        new = self.pack("new", {**self.metadata, "commit": "b" * 40},
                        {**self.row, "avg_us": 1, "p99_us": 9})
        result, code = baseline.compare(old, new, ["avg_us", "p99_us"], 0.05)
        self.assertEqual(code, 1)
        self.assertFalse(result["same_revision"])

    def test_counters_normalize_iterations_and_zero_regresses(self):
        old = self.pack("old")
        new = self.pack("new", row={**self.row, "iterations": 20, "layout_count": 20})
        self.assertEqual(baseline.compare(old, new, ["layout_count"], 0)[1], 0)
        zero = self.pack("zero", row={**self.row, "layout_count": 0})
        self.assertEqual(baseline.compare(zero, old, ["layout_count"], 0)[1], 1)

    def test_legacy_missing_timing_is_incomplete_not_pass(self):
        old = self.pack("old", row={k: v for k, v in self.row.items()
                                     if k not in ("p50_us", "p95_us", "p99_us")})
        new = self.pack("new")
        self.assertEqual(baseline.compare(old, new, ["avg_us"], 0)[1], 0)
        self.assertEqual(baseline.compare(old, new, ["p99_us"], 0)[1], 2)
        self.assertEqual(baseline.compare(old, new, ["gpu_us"], 0)[1], 2)

    def test_native_and_harness_evidence_cannot_mix(self):
        old = self.pack("old")
        new = self.pack("new", {**self.metadata, "measurement_kind": "native"})
        result, code = baseline.compare(old, new, ["avg_us"], 0)
        self.assertEqual(code, 2)
        self.assertEqual(result["status"], "incompatible")

    def test_missing_scenario_and_changed_hardware_cannot_pass(self):
        old = self.pack("old")
        new = self.pack("new", row={**self.row, "scenario": "different"})
        self.assertEqual(baseline.compare(old, new, ["avg_us"], 0)[1], 2)
        different = self.pack("hardware", {**self.metadata, "hardware": "other"})
        self.assertEqual(baseline.compare(old, different, ["avg_us"], 0)[1], 2)

    def test_digest_and_exclusive_capture(self):
        old = self.pack("old")
        with self.assertRaises(FileExistsError):
            self.pack("old")
        pack = json.loads(old.read_text())
        pack["metrics_jsonl"] += " "
        old.write_text(json.dumps(pack))
        with self.assertRaisesRegex(ValueError, "digest"):
            baseline.unpack(old)

    def test_empty_feature_set_is_explicit_and_malformed_envelopes_fail(self):
        old = self.pack("old", {**self.metadata, "features": []})
        self.assertEqual(baseline.unpack(old)[0]["features"], [])
        for envelope in ([], {"schema": True}, {"schema": 1,
                          "metadata": self.metadata, "metrics_jsonl": []}):
            old.write_text(json.dumps(envelope))
            with self.subTest(envelope=envelope), self.assertRaises(ValueError):
                baseline.unpack(old)

    def test_invalid_metadata_does_not_reserve_destination(self):
        with self.assertRaises(ValueError):
            self.pack("invalid", {**self.metadata, "hardware": {"value": float("inf")}})
        self.assertFalse((self.root / "invalid.pack").exists())

    def test_malformed_missing_and_duplicate_records(self):
        for rows in ("", json.dumps(self.row) + "\n" + json.dumps(self.row),
                     json.dumps({**self.row, "avg_us": -1}),
                     json.dumps({**self.row, "p99_us": None}),
                     json.dumps({**self.row, "iterations": 0})):
            with self.subTest(rows=rows), self.assertRaises(ValueError):
                baseline.records(rows)
        with self.assertRaisesRegex(ValueError, "qualification"):
            baseline.metadata({})


if __name__ == "__main__":
    unittest.main()
