# Component receipt comparison evidence

This candidate is not merged. It substantially reduces the measured local
interaction refresh, but the retained comparisons do not support an unconditional
5% no-regression claim. OPT-1388 remains open, including bounded geometry.

The production change retains a private immutable identity for Clone-qualified
component snapshots. An interaction-only leaf beside unchanged snapshots can use
the existing request-fenced atomic partial refresh. The enclosing view still runs;
changed component results and geometry still take the full path in these revisions.

## Revisions and runs

The initial baseline is `cbb2baba48e4ece4dbf3e934120f508e85a09014`; the initial
candidate is `91b31ca1d571e44262eda5dca2f111b5a02f2db4`. Root-level files retain
three interleaved pairs of 100-iteration runs. All three comparisons fail at
least one control check. The `repeatability` directory repeats that procedure
using the **same baseline executable for both sides**; all three also fail,
showing that these short runs cannot reliably attribute the control differences.

The `long-6000` directory uses identical bounded iteration support on both sides:
baseline `e349c1b6bd938f413e7c9d7145a0b76f3be94078`, candidate
`e0c8f5b144c6f1dbb0934fb3b42b09f164f7e503`. Each scenario performs 6,000 measured
calls after construction and one untimed warm-up, sampled in batches of eight.
All three long comparisons pass average timings; pair three passes every check.
Pairs one and two retain tail regressions and are not reclassified as passes.

## Observations

The local interaction fixture has 32 cached components with 100 text leaves each,
a fixed 3400 x 2700 viewport, and one tooltip that alternates each operation.
Its longer baseline averages are 856.279–927.562 microseconds and candidate
averages are 32.717–34.062 microseconds. Application projection remains one per
operation. Runtime projection drops from one to zero; layout is zero on both
sides, so this is not evidence of incremental geometry.

The other scenarios measure a changed component's geometry, cached application
projection, and existing full-refresh/paint controls. Every cohort contains the
same six scenarios. Sealed JSON records retain compiler, hardware, OS, power,
synthetic scale/refresh and sampling settings; manifests retain executable and
raw-data hashes. Cargo/rustc processes were checked before and after every
accepted measurement. These are backend-neutral timings, not native frame, GPU,
startup or cross-platform acceptance.

The candidate passes 5,251 library/integration tests, 270 example tests, 19
doctests and strict all-feature/all-target Clippy. The iteration follow-up passes
18 harness tests and 315 source guardrails. Runtime capture, focus, layout,
paint and semantic output are compared with the full-refresh path.

## Reproduce

Build the benchmark at either named long-run revision, then run:

```sh
CARGO_INCREMENTAL=0 cargo bench --locked --bench perf_harness -- \
  app_component_projection_cached_9600 runtime_component_local_geometry_3200 \
  runtime_component_local_interaction_3200 runtime_surface_large_tree \
  runtime_refresh_large_tree runtime_projection_refresh_large_tree \
  --jsonl --iterations 6000
```

The iteration override accepts one integer from 1 through 100,000 and preserves
all existing default iteration counts when omitted. Recheck a retained pair:

```sh
python3 scripts/qualified_baseline.py compare \
  evidence/performance/2026-09-06/component-receipt-reuse/long-6000/before-1.json \
  evidence/performance/2026-09-06/component-receipt-reuse/long-6000/after-1.json
```

That retained pair exits 1 for its control-tail regressions. Pair three exits 0.
