# Exact-input component projection controls

Three independent release harness runs at
`318ec8b5ffdc1056ebe90acee28c56dc444d6606` compare cached and forced-fresh
projection of 32 components with 300 text leaves each. The fixture changes the
first component every operation. Packs retain raw JSONL, digests and complete
machine/build/fixture qualification. Power was verified as AC, battery 100%.

| Metric per operation | Cached | Forced fresh |
| --- | --- | --- |
| Enclosing application projection calls | 1 | 1 |
| Component function calls | 1 | 32 |
| Component cache hits | 31 | 0 |
| Average projection time, range across runs | 155.450–174.470 µs | 4340.760–4402.180 µs |
| p99 batch average, range across runs | 163.693–199.974 µs | 4390.974–4795.057 µs |

Counters are totals divided by each row's 100 operations. Percentiles describe
batches of eight operations, not individual native frames. Cached results use
the ordinary function-item path. The fresh control uses a capturing forwarding
closure, which deliberately does not qualify for memoization and returns the
same component output. Both execute the same enclosing view and lowering entry.
Each fixture populates components before timing; no build or test overlapped
measurements. The cached scenario runs before the fresh scenario in each run.

These are same-revision controlled application-projection measurements, not a
historical native regression gate or an application-frame speedup. Runtime
projection, layout, painting, GPU execution and presentation are outside the
fixture. Separate production differential tests cover retained pointer capture,
focus, IME, paint and semantic parity, including actual component/widget-lowering
callback non-visitation. Native local-edit acceptance remains outstanding.
