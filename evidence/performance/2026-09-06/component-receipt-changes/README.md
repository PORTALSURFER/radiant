# Bounded component interaction evidence

This follow-up measures the component snapshot implementation at
`671e5fa427884bef1148e0c557ec6ce5d2beb653` against the identical seven-scenario
benchmark fixture at `8aec403c8c1d117416b0953215d621d804af810c`. It adds verified
interaction changes inside one component to the unchanged-component receipt
work documented in `../component-receipt-reuse/`.

## Results and limits

Three interleaved release pairs retain 6,000 measured calls per scenario,
with one untimed warm-up and batch size eight. All average and p95 checks pass
the existing 5% comparison threshold. Pair three passes every check. Pairs one
and two fail p99 checks and remain classified as regressions:

| Pair | Scenario | p99 change |
| --- | --- | --- |
| 1 | cached application projection | +13.49% |
| 1 | full runtime refresh | +29.92% |
| 2 | component geometry | +13.13% |
| 2 | full runtime refresh | +31.32% |

The inside-component interaction averages fall from 884.924–926.535 us to
89.177–90.146 us. Each edit invokes one component callback with 31 cache hits;
runtime projection and layout are both zero. The external interaction control
falls from 850.864–861.125 us to 32.925–33.185 us. Geometry still performs a full
3,233-node layout per edit. These results support the bounded interaction path;
they do not establish an unconditional no-regression claim, bounded geometry,
native frame timing, or GPU performance. OPT-1388 remains open.

Every raw row, sealed qualification pack, comparison and executable hash is
retained here. The earlier short, same-binary repeatability and long cohorts
are retained in `../component-receipt-reuse/`; this cohort does not replace them.
Cargo/rustc were checked before and after every run. Sampling order is baseline,
candidate; candidate, baseline; baseline, candidate. The fixture has 32 components
with 100 text leaves, explicit fixed slots and a 3400 x 2700 viewport. The new
scenario alternates the first leaf's tooltip inside component zero and checks
its actual published value and callback counts. Scale 1 and refresh 60 are
synthetic harness settings. Hardware, OS, power and compiler are in each seal.

Functional validation passed 5,255 all-feature library/integration tests. The
subsequent test-only IME expansion passed all 17 focused component tests; 270
examples, 19 doctests, strict all-feature/all-target Clippy, formatting and diff
checks also passed. Differential tests cover active capture, focus, composition,
paint and semantic parity. A changed component visits only its own 101 nodes;
unchanged components perform no descendant comparison. Skipped intermediate
projections cannot reuse a delta against the wrong committed predecessor.

## Reproduce

Build at either named revision, retaining separate executables:

```sh
CARGO_INCREMENTAL=0 cargo bench --locked --bench perf_harness --no-run
```

Run the resulting executable with:

```sh
perf_harness runtime_component_local_interaction_3200 \
  runtime_component_changed_interaction_3200 runtime_component_local_geometry_3200 \
  runtime_surface_large_tree runtime_refresh_large_tree \
  runtime_projection_refresh_large_tree app_component_projection_cached_9600 \
  --jsonl --iterations=6000
```

Recheck the retained third comparison (exit 0; pairs one and two exit 1):

```sh
python3 scripts/qualified_baseline.py compare \
  evidence/performance/2026-09-06/component-receipt-changes/before-3.json \
  evidence/performance/2026-09-06/component-receipt-changes/after-3.json
```
