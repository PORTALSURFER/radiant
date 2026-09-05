#!/usr/bin/env python3
"""Seal and compare qualified historical performance records (standard library only)."""

import argparse
import hashlib
import json
import math
from pathlib import Path
import re
import sys


QUALIFIERS = (
    "hardware", "os", "backend", "build_mode", "dataset", "scale",
    "refresh_hz", "power_state", "features", "measurement_kind", "sampling",
)


def read_json(path):
    return json.loads(Path(path).read_text(), parse_constant=lambda value: fail(value))


def fail(message):
    raise ValueError(message)


def metadata(value):
    if not isinstance(value, dict):
        fail("metadata must be an object")
    for key in (*QUALIFIERS, "commit"):
        if key not in value or value[key] is None or value[key] in ("", {}, []):
            fail(f"missing qualification: {key}")
    if not isinstance(value["commit"], str) or not re.fullmatch(r"[0-9a-f]{40}", value["commit"]):
        fail("commit must be a full lowercase git object id")
    if value["measurement_kind"] not in ("deterministic_harness", "native"):
        fail("measurement_kind must be deterministic_harness or native")
    if value["build_mode"] != "release":
        fail("historical evidence requires release mode")
    for key in ("scale", "refresh_hz"):
        number(value[key], key, positive=True)
    return value


def number(value, name, positive=False):
    if isinstance(value, bool) or not isinstance(value, (int, float)):
        fail(f"{name} must be numeric")
    if not math.isfinite(value) or value < 0 or (positive and value == 0):
        fail(f"{name} must be finite and {'positive' if positive else 'nonnegative'}")
    return value


def records(source):
    result = {}
    for line in source.splitlines():
        if not line.strip():
            continue
        row = json.loads(line, parse_constant=lambda value: fail(value))
        if not isinstance(row, dict):
            fail("metric must be an object")
        if row.get("type") != "radiant_perf":
            fail("expected radiant_perf metric records only")
        name = row.get("scenario")
        if not isinstance(name, str) or not name or name in result:
            fail("missing or duplicate scenario")
        number(row.get("avg_us"), "avg_us")
        tails = [row.get(key) for key in ("p50_us", "p95_us", "p99_us")]
        if any(value is not None for value in tails):
            for value in tails:
                number(value, "percentile")
            if tails != sorted(tails):
                fail("percentiles must be ordered")
        number(row.get("iterations"), "iterations", positive=True)
        if not isinstance(row["iterations"], int):
            fail("iterations must be an integer")
        result[name] = row
    if not result:
        fail("no metric records")
    return result


def seal(meta_path, metrics_path, output):
    meta = metadata(read_json(meta_path))
    raw = Path(metrics_path).read_bytes()
    records(raw.decode())
    pack = {"schema": 1, "metadata": meta,
            "metrics_sha256": hashlib.sha256(raw).hexdigest(), "metrics_jsonl": raw.decode()}
    # Exclusive creation preserves named before-change evidence.
    with Path(output).open("x") as stream:
        json.dump(pack, stream, indent=2, sort_keys=True, allow_nan=False)
        stream.write("\n")


def unpack(path):
    pack = read_json(path)
    if pack.get("schema") != 1:
        fail("unsupported evidence schema")
    meta = metadata(pack["metadata"])
    raw = pack["metrics_jsonl"].encode()
    if hashlib.sha256(raw).hexdigest() != pack["metrics_sha256"]:
        fail("evidence digest mismatch")
    return meta, records(raw.decode())


def compare(baseline, candidate, fields, tolerance):
    number(tolerance, "tolerance")
    if not fields or any(not field or field in ("iterations", "total_us") for field in fields):
        fail("select per-sample timings or operation counters")
    old_meta, old = unpack(baseline)
    new_meta, new = unpack(candidate)
    mismatches = [key for key in QUALIFIERS if old_meta[key] != new_meta[key]]
    if mismatches:
        return {"status": "incompatible", "qualifiers": mismatches}, 2
    rows = []
    for scenario in sorted(old.keys() | new.keys()):
        left, right = old.get(scenario), new.get(scenario)
        if left is None or right is None:
            rows.append({"scenario": scenario, "status": "missing_scenario"})
            continue
        for field in fields:
            a, b = left.get(field), right.get(field)
            if a is None or b is None:
                rows.append({"scenario": scenario, "field": field, "status": "unavailable"})
                continue
            number(a, field)
            number(b, field)
            # Counter totals are normalized to an operation; timings already are per sample.
            if not field.endswith("_us"):
                a /= left["iterations"]
                b /= right["iterations"]
            rows.append({"scenario": scenario, "field": field, "baseline": a,
                         "candidate": b, "status": "regression" if b > a * (1 + tolerance)
                         else "pass"})
    incomplete = any(row["status"] in ("unavailable", "missing_scenario") for row in rows)
    regression = any(row["status"] == "regression" for row in rows)
    return {"status": "incomplete" if incomplete else "regression" if regression else "pass",
            "baseline_commit": old_meta["commit"], "candidate_commit": new_meta["commit"],
            "same_revision": old_meta["commit"] == new_meta["commit"], "comparisons": rows}, (
                2 if incomplete else 1 if regression else 0)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest="command", required=True)
    capture = commands.add_parser("seal")
    capture.add_argument("metadata")
    capture.add_argument("metrics")
    capture.add_argument("output")
    check = commands.add_parser("compare")
    check.add_argument("baseline")
    check.add_argument("candidate")
    check.add_argument("--field", action="append", help="lower-is-better metric; repeatable")
    check.add_argument("--tolerance", type=float, default=0.05)
    args = parser.parse_args()
    try:
        if args.command == "seal":
            seal(args.metadata, args.metrics, args.output)
            return 0
        result, code = compare(args.baseline, args.candidate,
                               args.field or ["avg_us", "p95_us", "p99_us"], args.tolerance)
        print(json.dumps(result, sort_keys=True, allow_nan=False))
        return code
    except (ValueError, OSError, KeyError, TypeError) as error:
        print(json.dumps({"status": "invalid", "error": str(error)}), file=sys.stderr)
        return 2


if __name__ == "__main__":
    sys.exit(main())
