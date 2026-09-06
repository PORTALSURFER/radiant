# Failed native baseline capture — 6 September 2026

This is retained failure evidence, not an accepted performance baseline.
A release build of commit `90b8b02c022b1ad76ccd49eae520b48c74cd6d51`
was packaged with `scripts/native_baseline_bundle.py` and opened through Finder.
The real executable bundle reveals the window; earlier CLI and shell-wrapper
launches did not receive application activation confirmation.

The updated recorder retained 238 frame and 237 GPU callback rows before a
native runtime unwind. All GPU callbacks reported zero duration; these values
are not treated as measured rendering performance. The final `native_run`
records a failure, and `native_baseline_metrics.py` rejects the raw recording
with exit 2. The pack intentionally has no sealed performance metrics.

Earlier foreground runs on the same source base reproduced Metal
`kIOGPUCommandBufferCallbackErrorOutOfMemory` errors and this WGPU panic:

```text
We timed out while waiting on the last successful submission to complete!
```

The exact cause of the native GPU failure remains unresolved. No renderer
optimization, native acceptance pass, or successful physical GPU completion is
inferred. OPT-1452 remains open. Active user applications were preserved.
