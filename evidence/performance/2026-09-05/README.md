# Initial historical harness comparison

`eabc6dff-runtime-hover.json` retains the release benchmark from canonical
revision eabc6dff122b01cb9b7238ba473e89177883a152. The capture checkout had only
the new Python tooling, its tests, CI wiring and documentation; no library or
benchmark code differed from that revision. The candidate capture is named
after c41b9c1942dc954c047922dc2b7e8bfc67a9af90, which adds that tooling without
changing measured runtime code.

`runtime-hover-comparison.json` was emitted by the qualified comparator, with
average/p95/p99 and normalized relayout counts selected. All comparisons pass
the default 5% tolerance. This establishes historical comparison plumbing,
not a speedup claim. Both runs report exactly one relayout per operation.
Batch timings vary substantially within each workload: p50 is approximately
434 microseconds while p99 is approximately 85 milliseconds. Do not use these
two runs to establish a stable native timing threshold.

Commands used:

```sh
cargo bench --locked --bench perf_harness runtime_virtualized_list_hover -- \
  --jsonl --write-baseline-jsonl /tmp/metrics.jsonl
python3 scripts/qualified_baseline.py seal /tmp/metadata.json /tmp/metrics.jsonl pack.json
python3 scripts/qualified_baseline.py compare eabc6dff-runtime-hover.json \
  c41b9c19-runtime-hover.json --field avg_us --field p95_us --field p99_us \
  --field relayout_count
```

The hardware/OS/compiler/power qualification is embedded in each pack. The
scale and refresh values are synthetic harness settings, not measurements of
the connected displays. This pack contains no GPU or native presentation
measurements. Injected tail and zero-counter regressions are independently
covered by `scripts/test_qualified_baseline.py` and must return failure.
