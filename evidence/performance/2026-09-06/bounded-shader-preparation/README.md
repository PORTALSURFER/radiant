# Bounded custom shader preparation validation

OPT-1457 implementation is staged, not accepted as a foreground performance improvement.

Final source revision: `312a771cf9916363e62b9271a5f3719002dc0901`.
Guardrail expectations were updated at `cf575af7` and formatted at `cef645ea`.
These later commits change tests only.

| Retained log | Revision | Result |
| --- | --- | --- |
| clippy.log | 312a771c | Strict all-target/all-feature Clippy passed |
| guardrails.log | cf575af7 | 315 passed |
| fmt.log | cef645ea | Formatting passed |
| shader.log | cef645ea | 64 passed, 6 opt-in tests ignored |
| native.log | cef645ea | 8 real native GPU renderer tests passed |
| broker-native.log | cba51d86 | 3 real native GPU broker tests passed |

The broker native run preceded the final boxed staging representation and removal
of an always-true reconciliation argument. The focused and renderer native suites
were rerun after those changes. Earlier broad GPU coverage at 57b39d6d passed
222 tests with 10 opt-in tests ignored; it is not represented as a final-head run.

The native tests exercise prepared pipeline installation, ordered repeated surface
keys, pending replacement and cache preservation, saturated transaction rollback,
and invalid-pipeline fallback. Broker tests cover bounded admission, coalescing,
per-device dispatch, cancellation/stale completion, host rejection retry, and
retirement. The production worker performs WGPU creation and validation scopes
on one worker thread. Demand redraw only consumes prepared handles; binding
validation polls once and takes an incomplete fallback if not ready.

Installed GPU cache ownership is bounded separately from transient preparation
leases. Committed ordered target receipts release all matching broker interests.
There are at most two active workers globally, one per captured device, eight
queued jobs, 256 transient identities, 1,024 interests, and 1 MiB retained key text.
These are logical bounds, not total process/GPU memory measurements.

The sibling synchronous-shader-preparation directory retains the pre-change phase
measurements. Neither those microfixtures nor these offscreen tests measure real
input-to-present latency. Worker CPU comparison and foreground before/after
acceptance remain outstanding. OPT-1457 must remain open pending that evidence.
