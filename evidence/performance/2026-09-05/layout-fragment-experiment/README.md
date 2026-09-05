# Rejected geometry-fragment experiment

The exact-input fragment cache is **not selected for main**. All three paired
comparisons fail the existing 5% regression gate. Its lower layout-call count
does not translate into a useful total-refresh reduction, and the paint control
is consistently slower in these runs. This evidence closes no part of OPT-1388's
remaining runtime-projection or bounded-geometry acceptance requirement.

## Revisions and fixture

- Baseline: `cc395ab583f1fe6bebc1bfd684f5e798948e7f9c` on
  `codex/opt-1388-layout-fragment-control`.
- Candidate: `aa3e2db1cf641349073e84b39e99e10d99234b2a` on the unmerged
  `codex/opt-1388-layout-fragments` experiment branch.
- Both include the same guarded fixture and invalid-constraint clamp correction.
  The candidate also integrates current-main native recorder tooling, which is
  not executed by these scenarios.

The local-edit fixture has 32 explicit pure components with 100 text leaves each.
All leaves have a fixed parent slot width of 100 pixels. Only the first leaf of
the first component alternates between 20 and 21 pixels of parent slot height.
The 3400 x 2700 viewport accommodates the normal container gaps. Construction
and one warm-up call occur before 100 measured calls, sampled in batches of 8.
Every measured edit asserts exactly one new runtime layout pass and no overflow;
the reported node visits therefore belong to an actual pass for that edit.

The other three scenarios are existing large-tree paint and refresh controls.
Their unchanged refresh cases intentionally report zero layout passes. All six
raw JSONL files and sealed metadata packs are retained, along with all three
comparison results. The seals contain the SHA-256 of the original JSONL bytes.
Runs were serial: three baseline packs followed by three final candidate packs,
with the same scenario order each time. No local build or test overlapped a
measured run. Hardware, OS, compiler, features and AC power are recorded in each
pack; scale and refresh are synthetic harness settings, not display evidence.

## Observations

| Metric | Baseline | Candidate |
| --- | ---: | ---: |
| Local edit layout calls per operation | 3,233 | 102 |
| Local edit average, microseconds | 1193.370–1277.080 | 1184.800–1279.040 |
| Local edit p99 batch average, microseconds | 1255.031–1437.792 | 1267.781–1489.135 |
| Paint control average, microseconds | 60.390–61.990 | 65.460–66.780 |

Layout calls fall by 96.8%, but total-refresh ranges overlap. The paired paint
control averages are 7.7%, 5.6% and 9.3% slower. The other refresh controls show no
regression at the configured threshold. These deterministic observations do not
establish native frame, GPU, startup or cross-platform performance.

The prototype retains exact plain-container inputs and completed rectangles,
replays measurement calls and compatibility diagnostics, and bounds cache size
and ID admission. It still scans the whole layout tree for unique IDs, compares
all candidate child inputs, copies all output rectangles and performs full
runtime projection. The next implementation must address that remaining work
rather than treating fewer layout-function calls as sufficient evidence.

Earlier exploratory fixtures used widget sizing without the explicit parent-slot
and per-pass assertions. Their local scratch timings are not qualification or
acceptance evidence and are not mixed into this comparison.

## Reproduce

At either named revision:

```sh
CARGO_INCREMENTAL=0 cargo bench --locked --bench perf_harness -- \
  runtime_component_local_geometry_3200 runtime_surface_large_tree \
  runtime_refresh_large_tree runtime_projection_refresh_large_tree --jsonl
```

Re-run any retained comparison with:

```sh
python3 scripts/qualified_baseline.py compare \
  evidence/performance/2026-09-05/layout-fragment-experiment/before-1.json \
  evidence/performance/2026-09-05/layout-fragment-experiment/after-1.json
```

The comparison exits 1 for the retained regression. Missing measurements must
remain missing; no native capture is represented by this harness evidence.
