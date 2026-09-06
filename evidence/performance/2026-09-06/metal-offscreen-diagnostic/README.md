# Metal offscreen diagnosis — not a native baseline

On the same Apple M5 Pro host used for the failed native capture, all four
modes of this standalone diagnostic verified all 4,096 pixels after 32 draws
to a 64×64 RGBA8Unorm offscreen texture. Basic clearing and Vello GPU rendering
passed with timestamp queries disabled and enabled. Both timestamp modes
returned start=0, end=0, and period=1, despite advertised timestamp support.
`PASS` refers only to correct rendering/readback; it does not qualify timing.

The retained logs were captured consecutively using one executable, whose
SHA256 is in `manifest.json`. The manifest also hashes the source, dependency
lockfile, and raw outputs. No native window, swapchain presentation, refresh
rate, power state, or larger workload is qualified by this probe. These small
workloads did not reproduce the earlier Metal memory error. They do not prove
that timestamp queries caused it or that the native runtime failure is fixed.

The aggregator now withholds GPU metrics for entirely zero-valued cohorts and
reports `gpu_unqualified_zero`, while preserving CPU evidence and raw samples.
Mixed cohorts retain individual zero values. This avoids presenting the
observed zero-only query results as measured zero-cost GPU rendering without
changing the public runtime timing outcome API.

## Reproduction on a Metal host

Copy this directory outside the Radiant workspace (it is an independent Cargo
package), then run each command in that directory:

```sh
cargo run --locked
cargo run --locked -- --timestamps
cargo run --locked -- --vello
cargo run --locked -- --vello --timestamps
```

Each mode returns a failure for unsupported requested features, readback
mismatch, or a device/readback error. Timestamp values are printed as evidence;
zero timestamps do not fail the rendering check. The probe uses bounded device
poll/readback waits. It does not modify system GPU settings.

## Native retry

`native-retry.jsonl` is a separate native-window attempt from the recorder at
source commit `90b8b02c022b1ad76ccd49eae520b48c74cd6d51`, executable SHA256
`39e4544f8f462b5e3020098b81bd760e83e326d54cc8c0dfba74ebfaa7a8ab5a`.
It exited before first present after approximately 21 seconds, with incomplete
startup and no frame observations. Its recorded renderer build succeeded, but
that does not establish successful presentation. This attempt neither accepts
a native baseline nor reproduces the earlier out-of-memory error. OPT-1452 and
its dependent native performance evidence remain open.
