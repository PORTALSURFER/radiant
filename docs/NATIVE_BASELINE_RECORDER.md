# Native rendering workload recorder

`rendering_baseline` records bounded raw observations from the public frame and
GPU callbacks. Build in release mode at a named revision and run on an unlocked,
visible desktop. Use a new output filename for each independent process:

```sh
cargo build --release --example rendering_baseline
target/release/examples/rendering_baseline local /tmp/radiant-local-1.jsonl
python3 scripts/native_baseline_metrics.py /tmp/radiant-local-1.jsonl /tmp/radiant-local-1
```

The recorder refuses to overwrite a file. The aggregator refuses to overwrite a
directory and rejects runs with no successful present, invalid timings, duplicate
identities, missing window roles, or unmatched GPU records. Keep its `raw.jsonl`,
`metrics.jsonl` and `availability.json` together. Seal `metrics.jsonl` with the
qualification procedure in `QUALIFIED_BASELINES.md`; use `measurement_kind:
"native"`, the actual display/backend settings, and the recorder revision.
Record power state, machine, OS, feature set, source fixture and scale/refresh
alongside each run. A script test is not a native capture.

For a Finder launch on macOS, prepare a real executable bundle:

```sh
python3 scripts/native_baseline_bundle.py target/release/examples/rendering_baseline \
  local /absolute/output/local-1.jsonl /absolute/output/Local-1.app
```

Open that app in Finder. The helper copies the executable directly into the
bundle and supplies `RADIANT_BASELINE_MODE` and `RADIANT_BASELINE_OUTPUT` through
its launch environment. Both variables are required when no command-line
arguments are supplied; explicit CLI arguments take precedence. Use a new
bundle and output for each run. The helper refuses existing bundles and output
files and does not launch, sign or install anything. A shell wrapper as the
bundle executable can prevent native activation from reaching the recorder.

The final `native_run` row includes the complete startup timing artifact, when
available, and `run_error`. An unconfirmed activation can therefore be
distinguished from a later rendering failure. Recoverable Rust unwinds at the
executable's outer runtime boundary preserve preceding observations as failed
diagnostics; the runtime is never reused after an unwind. Process aborts and
forced termination may still leave an empty file. The aggregator rejects a
failed run even if it presented frames before failing. Never use partial
failure records as a performance baseline.

The workload argument is one of:

| Mode | Work |
| --- | --- |
| `cold` | First presentation of a ten-minute, 48 kHz, stereo raw signal; static viewport afterward |
| `pan` | Repeated 32-frame viewport shifts over the same long source |
| `crossing` | Repeated 48,000-frame viewport shifts over the same long source |
| `gain` | Gain/fade preview using one precomputed immutable signal summary |
| `shaders` | Sixteen distinct surface keys with identical shader content |
| `local` | Edit the first label while 99 sibling labels remain unchanged |
| `two_windows` | The local-edit fixture in a primary and one auxiliary window |
| `idle` | Static labels with animation disabled for a 20-second observation |

Animated captures stop after 240 ticks plus 250 ms to drain GPU observations,
with a 20-second timeout. Source preparation is measured before application
launch and reported separately from startup and frame work. The source is
synthetic; this fixture measures rendering rather than audio or decoding.

The aggregator splits the first successful frame of each window from subsequent
warm frames. CPU preparation, render encoding, blit encoding, composition,
overlay preparation, and submission/presentation remain separate. Successful
present intervals exclude each window's first frame. GPU observations correlate
by window identity and frame sequence, independent of delivery order. A cohort
gets a GPU timing metric only when every presented frame has a measured GPU
result; otherwise the availability report says `gpu_unavailable`. Missing warm
frames say `no_samples`. Neither condition is a passing complete native pack.
Idle captures use frame count and elapsed duration as observed activity evidence;
they do not infer event-loop wakeups or energy use from frame count.

Capture at least three independent processes per mode. Retain each run rather
than selecting the fastest. Compare cold samples across processes and report
warm average/p50/p95/p99 ranges across runs. Treat isolated tolerance failures or
overlapping noisy ranges as inconclusive until repeated under equivalent load.
Inspect work counters as well as timings. A CPU-only comparison does not close a
GPU evidence requirement. Native acceptance remains at 60 Hz; 120 Hz is stretch.
No result represents scanout or input-to-display latency.

Use the same fixtures and qualification on future native Windows and Wayland
hosts, recording unsupported timing explicitly. Compilation on those targets
alone does not satisfy their native acceptance gates.
