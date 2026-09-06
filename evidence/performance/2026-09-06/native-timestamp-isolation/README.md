# Native timestamp query isolation

Diagnostic evidence on macOS 26.6.2 / Apple M5 Pro. These are not accepted
native rendering performance packs. Each app retains the same release
`local` workload, 760×520 window, 240 animation ticks and shutdown drain.
Actual successful-present counts can differ from animation ticks.

| Variant | Observed processes | Result |
| --- | --- | --- |
| Main `905ec913` | 2 | 238/237 frame observations; terminal WGPU timeout; first run system log has 981 Metal OOM messages |
| Timestamp features disabled | 1 | 240 frames, Unsupported timing, process completes |
| Timing observer absent | 3 | first two never present; third completes with 239 frames |
| Query pool absent, features retained | 1 | 240 frames, Unsupported timing, completes; captured log has no OOM messages |
| Invalid-clock quarantine `76ca1cc6` | 3 | completes with 238/241/240 frames and ConversionFailed timing, but actual process logs still have 8/42 OOM messages in runs 1/3 |

The query-pool-disabled control isolates query work from feature enablement.
Post-readback quarantine prevents the repeated terminal timeout in these runs,
but cannot make the initial timestamp submissions safe. Completion alone is
therefore insufficient for rendering acceptance. No valid GPU durations were
observed. The first two observer-off attempts are retained as incomplete, not
counted as successful controls. System-capture coverage differs across runs;
absence of a separate captured log is not proof of absence of driver errors.

Build manifests retain exact source revisions, binary SHA-256 hashes and
commands. The diagnostic variants are reproducible from main `905ec913` with
the included patches. Raw recorder JSONL remains unchanged. System logs are
losslessly gzip-compressed; `raw-manifest.json` records both original and
retained hashes. The GPU quarantine is a parent source commit in this branch.

The proposed production response is to decline the current standalone-encoder
timestamp strategy before submission on the pinned WGPU 29 Metal backend.
This is an unavailable diagnostic capability, not a claim that every Metal
device has defective timestamps or that rendering itself requires timestamps.

## Pre-submission Metal policy result

Production candidate `d01814d85c417b296e9ac106876b4c24a7e8974f` declines the
standalone timestamp feature pair before Metal device creation. Its exact
release binary is retained by SHA-256 in `metal-policy-build-manifest.json`.

- Run 1 completed in 4.093 seconds with 239 successful-present and matching
  Unsupported timing observations. Captured system logs contain no OOM event.
- Runs 2 and 3 reached a prepared scene but never revealed/presented, exiting
  with explicit incomplete startup artifacts. Both are retained; neither is a
  passing native capture. Captured logs contain no OOM event.
- Run 4 received a second explicit Finder open while already running. It then
  revealed at 19.783 seconds and first presented at 19.870 seconds, retaining
  130 presented frames before the existing timeout. No OOM event was captured.
  This assisted activation diagnostic is not an unassisted cold baseline.

System logs identify all four actual executable paths. The first successful
capture validates the query-path mitigation on this machine, with explicit
unavailable timing. Intermittent activation and the full repeated native pack
remain outstanding. No GPU duration or 60 Hz acceptance is claimed; the main
built-in display was configured at 120 Hz and recorder scale was 2.
