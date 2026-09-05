#!/usr/bin/env python3
"""Aggregate recorder observations without substituting CPU timings for GPU work."""

import argparse
from collections import defaultdict
import hashlib
import json
import math
from pathlib import Path
import sys

CPU_FIELDS = (
    "cpu_prepare_us", "cpu_render_encode_us", "cpu_blit_encode_us",
    "cpu_composite_us", "cpu_overlay_us", "cpu_submit_present_us", "cpu_total_us",
)
COUNTERS = (
    "scene_rebuild_count", "signal_summary_build_count", "signal_body_render_count",
    "shader_pipeline_rebuild_count", "shader_static_write_bytes",
)


def number(value):
    if isinstance(value, bool) or not isinstance(value, (int, float)) or not math.isfinite(value) or value < 0:
        raise ValueError("expected finite nonnegative observation")
    return value


def identity(row):
    values = row.get("window"), row.get("sequence")
    if any(type(value) is not int or value <= 0 for value in values):
        raise ValueError("missing successful-present correlation identity")
    return values


def metric(name, values):
    ordered = sorted(number(value) for value in values)
    if not ordered:
        raise ValueError("empty metric cohort")
    return {
        "type": "radiant_perf", "scenario": name, "category": "native",
        "group": "rendering", "iterations": len(ordered),
        "avg_us": sum(ordered) / len(ordered),
        **{f"p{percent}_us": ordered[math.ceil(len(ordered) * percent / 100) - 1]
           for percent in (50, 95, 99)},
    }


def aggregate(raw):
    rows = [json.loads(line) for line in raw.splitlines() if line.strip()]
    if any(not isinstance(row, dict) for row in rows):
        raise ValueError("expected observation objects")
    fixtures = [row for row in rows if row.get("type") == "native_fixture"]
    runs = [row for row in rows if row.get("type") == "native_run"]
    if len(fixtures) != 1 or len(runs) != 1:
        raise ValueError("expected one fixture and one completed run")
    fixture, run = fixtures[0], runs[0]
    if run.get("startup_failure") or run.get("first_present_ms") is None:
        raise ValueError("run did not reach its first successful present")
    workload = fixture.get("workload")
    if workload not in ("cold", "pan", "crossing", "gain", "shaders", "local", "two_windows", "idle"):
        raise ValueError("unknown workload")
    frames, gpu = {}, {}
    windows = defaultdict(list)
    for row in rows:
        kind = row.get("type")
        if kind not in ("native_frame", "native_gpu"):
            continue
        key = identity(row)
        target = frames if kind == "native_frame" else gpu
        if key in target:
            raise ValueError("duplicate frame correlation identity")
        target[key] = row
        if kind == "native_frame":
            if row.get("workload") != workload or row.get("role") not in ("primary", "auxiliary"):
                raise ValueError("incompatible frame cohort")
            for field in (*CPU_FIELDS, *COUNTERS, "present_interval_us"):
                number(row.get(field))
            windows[key[0]].append(row)
        elif row.get("gpu_us") is not None:
            number(row["gpu_us"])
    if not frames:
        raise ValueError("no native frames")
    if gpu.keys() - frames.keys():
        raise ValueError("GPU observation has no matching presented frame")
    roles = {row["role"] for row in frames.values()}
    if roles != ({"primary", "auxiliary"} if workload == "two_windows" else {"primary"}):
        raise ValueError("missing or unexpected window role")
    output = [metric(f"{workload}/startup/first_present", [number(run["first_present_ms"]) * 1000]),
              metric(f"{workload}/source_prepare", [number(fixture.get("source_prepare_us"))])]
    availability = []
    for window, cohort in sorted(windows.items()):
        cohort.sort(key=lambda row: row["sequence"])
        role = cohort[0]["role"]
        if any(row["role"] != role for row in cohort):
            raise ValueError("window role changed during capture")
        if sum(items[0]["role"] == role for items in windows.values()) != 1:
            raise ValueError("ambiguous window role")
        for phase, selected in (("cold", cohort[:1]), ("warm", cohort[1:])):
            if not selected:
                availability.append({"role": role, "phase": phase, "status": "no_samples"})
                continue
            prefix = f"{workload}/{role}/{phase}"
            for field in CPU_FIELDS:
                record = metric(f"{prefix}/{field}", [row[field] for row in selected])
                if field == "cpu_total_us":
                    record.update({field: sum(row[field] for row in selected) for field in COUNTERS})
                output.append(record)
            # The first profile has no prior successful presentation in this run.
            if phase == "warm":
                output.append(metric(f"{prefix}/present_interval_us", [row["present_interval_us"] for row in selected]))
            samples = [gpu.get(identity(row)) for row in selected]
            complete = all(sample is not None and sample.get("gpu_us") is not None for sample in samples)
            availability.append({"role": role, "phase": phase, "frames": len(selected),
                                 "gpu_samples": sum(sample is not None and sample.get("gpu_us") is not None for sample in samples),
                                 "status": "complete" if complete else "gpu_unavailable",
                                 "outcomes": sorted({str(sample.get("outcome")) if sample else "missing" for sample in samples})})
            if complete:
                output.append(metric(f"{prefix}/gpu_us", [sample["gpu_us"] for sample in samples]))
    return output, {"schema": 1, "raw_sha256": hashlib.sha256(raw.encode()).hexdigest(),
                    "workload": workload, "frame_count": len(frames),
                    "run_elapsed_us": number(run.get("elapsed_us")), "availability": availability}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("raw")
    parser.add_argument("output_directory", help="new directory; existing captures are never overwritten")
    args = parser.parse_args()
    try:
        raw = Path(args.raw).read_text()
        metrics, report = aggregate(raw)
        serialized = "".join(json.dumps(row, allow_nan=False) + "\n" for row in metrics)
        destination = Path(args.output_directory)
        destination.mkdir()
        (destination / "raw.jsonl").write_text(raw)
        (destination / "metrics.jsonl").write_text(serialized)
        (destination / "availability.json").write_text(json.dumps(report, indent=2, allow_nan=False) + "\n")
    except (ValueError, OSError, KeyError, TypeError) as error:
        print(json.dumps({"status": "invalid", "error": str(error)}), file=sys.stderr)
        return 2
    return 0


if __name__ == "__main__":
    sys.exit(main())
