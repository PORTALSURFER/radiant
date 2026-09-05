# Qualified historical performance evidence

`scripts/qualified_baseline.py` seals existing `radiant_perf` JSONL into a
self-contained evidence pack. It leaves the benchmark CLI and its legacy
average-only comparison unchanged. CI's same-run comparison remains a harness
smoke test. Historical native acceptance uses a separately named earlier pack.

The metadata JSON must contain `commit` (full Git object ID), `hardware`, `os`,
`backend`, `build_mode` (`release`), `dataset`, `scale`, `refresh_hz`,
`power_state`, `features`, `measurement_kind` (`native` or
`deterministic_harness`), and `sampling`. Record precise hardware and OS
versions, compiler and feature settings, dataset generation/revision and bytes,
display scale/refresh, power source/mode, and sample counts and aggregation in
these fields. Structured values are supported; both records must match exactly
in every qualification field. Commit is recorded but may differ.

```sh
python3 scripts/qualified_baseline.py seal metadata.json metrics.jsonl before.json
python3 scripts/qualified_baseline.py compare before.json after.json
python3 scripts/qualified_baseline.py compare before.json after.json \
  --field p99_us --field layout_count --tolerance 0
```

Sealing exclusively creates the destination, refusing overwrite. The pack
contains original metric bytes and their SHA-256 digest, checked on load. This
detects accidental changes; it is not an authenticity signature. Retain packs
in version control or an immutable artifact store alongside the native logs.

Default gates compare average, p95 and p99 independently with 5% tolerance.
Explicit fields are **lower-is-better** metrics: do not select cache-hit counts
as regression gates. Counter totals are divided by each record's iteration
count; `_us` timings already represent per-sample observations. A zero baseline
regresses on any positive result. Exit 0 means every requested comparison
passed, 1 means a regression, and 2 means invalid, incompatible or incomplete
evidence. Missing scenarios, unavailable GPU timings and legacy records without
requested percentiles cannot pass. Average-only legacy records remain usable
with `--field avg_us`. The report identifies same-revision comparisons.

Keep CPU preparation, CPU encode/submit, successful-present cadence and true
GPU timestamp intervals in separate named scenarios. Do not substitute CPU
submission for GPU work or successful presentation for scanout. Use absent
metrics for unavailable observations, never invented zeros. Existing harness
percentiles describe timed batches, not individual native frames; record that
distinction in `sampling` and `measurement_kind`.

For native before-change capture, retain repeated cold launches and separate
warm distributions for fixed-source gain/fade and nearby pan, a resident-range
crossing control, a 10-minute stereo 48 kHz source with explicit sample bytes,
16 equivalent shader surfaces, a local component edit, two windows and idle.
Record each workload's fixture and cold/warm state in `dataset`; use separate
packs when qualifications differ. Native platform/backend packs cannot be
interchanged. The 60 Hz acceptance target remains authoritative; 120 Hz is a
stretch target. Record variance across repeated runs before choosing timing
tolerances; deterministic work counters may use zero tolerance.

The comparison tooling alone does not satisfy native workload acceptance.
OPT-1452 remains open until release-mode baseline artifacts are captured;
OPT-1418 and OPT-1376 own final platform acceptance.
