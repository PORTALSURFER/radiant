import json
import unittest

from native_baseline_metrics import CPU_FIELDS, COUNTERS, aggregate


class NativeMetricsTests(unittest.TestCase):
    def rows(self, workload="local", windows=((1, "primary"),)):
        rows = [dict(type="native_fixture", workload=workload, source_prepare_us=4),
                dict(type="native_run", first_present_ms=5, startup_failure=None, elapsed_us=100)]
        for window, role in windows:
            for sequence in (1, 2, 3):
                rows.append(dict(type="native_frame", workload=workload, window=window,
                                 sequence=sequence, role=role, present_interval_us=sequence * 10,
                                 **{key: sequence for key in (*CPU_FIELDS, *COUNTERS)}))
                rows.append(dict(type="native_gpu", window=window, sequence=sequence,
                                 gpu_us=sequence * 2, outcome="measured"))
        return rows

    def run_rows(self, rows):
        return aggregate("".join(json.dumps(row) + "\n" for row in rows))

    def test_cold_warm_and_counters_are_separate(self):
        metrics, report = self.run_rows(self.rows())
        metrics = {row["scenario"]: row for row in metrics}
        self.assertEqual(metrics["local/primary/cold/cpu_total_us"]["iterations"], 1)
        warm = metrics["local/primary/warm/cpu_total_us"]
        self.assertEqual((warm["iterations"], warm["avg_us"], warm["p99_us"]), (2, 2.5, 3))
        self.assertEqual(warm["scene_rebuild_count"], 5)
        self.assertNotIn("local/primary/cold/present_interval_us", metrics)
        self.assertEqual(report["availability"][1]["status"], "complete")

    def test_missing_gpu_is_unavailable_not_zero_or_partial_mean(self):
        rows = self.rows()
        rows[-1]["gpu_us"] = None
        rows[-1]["outcome"] = "unsupported"
        metrics, report = self.run_rows(rows)
        self.assertFalse(any(row["scenario"].endswith("warm/gpu_us") for row in metrics))
        self.assertEqual(report["availability"][1]["gpu_samples"], 1)
        self.assertEqual(report["availability"][1]["status"], "gpu_unavailable")

    def test_all_zero_gpu_cohorts_are_unqualified_without_losing_cpu_evidence(self):
        rows = self.rows()
        for row in rows:
            if row["type"] == "native_gpu":
                row["gpu_us"] = 0
        metrics, report = self.run_rows(rows)
        self.assertFalse(any(row["scenario"].endswith("/gpu_us") for row in metrics))
        self.assertTrue(any(row["scenario"].endswith("/cpu_total_us") for row in metrics))
        self.assertEqual([row["status"] for row in report["availability"]],
                         ["gpu_unqualified_zero", "gpu_unqualified_zero"])
        self.assertEqual([row["gpu_samples"] for row in report["availability"]], [1, 2])

    def test_zero_qualification_is_per_window_and_phase_and_preserves_mixed_samples(self):
        rows = self.rows("two_windows", ((1, "primary"), (7, "auxiliary")))
        for row in rows:
            if row["type"] == "native_gpu" and (row["window"] == 7 or row["sequence"] == 2):
                row["gpu_us"] = 0
        metrics, report = self.run_rows(rows)
        warm = next(row for row in metrics if row["scenario"] == "two_windows/primary/warm/gpu_us")
        self.assertEqual(warm["avg_us"], 3)
        self.assertEqual(warm["p50_us"], 0)
        self.assertFalse(any(row["scenario"].startswith("two_windows/auxiliary/")
                             and row["scenario"].endswith("/gpu_us") for row in metrics))
        self.assertEqual([row["status"] for row in report["availability"]],
                         ["complete", "complete", "gpu_unqualified_zero", "gpu_unqualified_zero"])

    def test_gpu_correlation_does_not_depend_on_delivery_order(self):
        rows = self.rows()
        metrics, _ = self.run_rows(list(reversed(rows)))
        gpu = next(row for row in metrics if row["scenario"].endswith("warm/gpu_us"))
        self.assertEqual(gpu["avg_us"], 5)

    def test_missing_window_and_duplicate_identity_fail(self):
        with self.assertRaisesRegex(ValueError, "window role"):
            self.run_rows(self.rows("two_windows"))
        rows = self.rows()
        with self.assertRaisesRegex(ValueError, "duplicate"):
            self.run_rows(rows + [rows[-1]])
        rows[-1]["sequence"] = 99
        with self.assertRaisesRegex(ValueError, "no matching"):
            self.run_rows(rows)

    def test_two_window_timings_remain_separate(self):
        metrics, report = self.run_rows(self.rows("two_windows", ((1, "primary"), (7, "auxiliary"))))
        self.assertEqual(report["frame_count"], 6)
        self.assertTrue(any("/auxiliary/warm/" in row["scenario"] for row in metrics))
        self.assertTrue(any("/primary/warm/" in row["scenario"] for row in metrics))

    def test_runtime_failure_after_present_is_not_an_accepted_capture(self):
        rows = self.rows()
        rows[1]["run_error"] = "device lost after first present"
        with self.assertRaisesRegex(ValueError, "diagnostic only"):
            self.run_rows(rows)
        rows[1]["run_error"] = None
        self.assertEqual(self.run_rows(rows)[1]["frame_count"], 3)

    def test_empty_failed_and_nonfinite_runs_fail(self):
        with self.assertRaisesRegex(ValueError, "no native frames"):
            self.run_rows(self.rows()[:2])
        rows = self.rows()
        rows[1]["first_present_ms"] = None
        with self.assertRaisesRegex(ValueError, "first successful"):
            self.run_rows(rows)
        rows = self.rows()
        rows[2]["cpu_total_us"] = float("nan")
        with self.assertRaisesRegex(ValueError, "finite"):
            self.run_rows(rows)


if __name__ == "__main__":
    unittest.main()
