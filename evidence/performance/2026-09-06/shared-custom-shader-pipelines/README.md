# Shared custom shader pipelines: functional GPU evidence

6 September 2026. Functional offscreen validation on the local macOS development host. These are not cold/warm timing baselines, GPU-duration measurements, or foreground acceptance results.

Tested source head: `8473bf831cb86b6811a854b99e5d86f457b9a1d7`.

The explicit native command was:

```sh
CARGO_INCREMENTAL=0 CARGO_TARGET_DIR=/Users/portalsurfer/.codex/radiant-validation/target cargo test --locked --all-features --lib gpu_surface::native_tests -- --ignored --test-threads=1 --nocapture
```

All five tests passed using an actual WGPU offscreen device and 64x64 render target, without window activation or timestamp queries. The tests require an adapter selected from WGPU PRIMARY backends and fail if none is available; the log does not record an exact adapter identity.

- Sixteen disjoint tiles create one physical pipeline and sixteen independent bindings. Two-frame pixel readback and write-state assertions show a targeted payload/presentation update leaves the other tiles unchanged and creates no new pipeline or binding.
- A full 1,024-association cache admits and renders a fresh static surface in that frame, then retires stale associations while retaining one shared physical pipeline.
- An extra upload action after terminal cleanup vetoes commit and restores prior resource ownership/write state, repeatedly without accumulation.
- Invalid planned shader replacement preserves prior resources when execution vetoes.
- Invalid shader creation during a legacy saturated-cache transition restores the predecessor.

The retained broader suite passed 214 GPU-surface tests (four opt-in native tests ignored at that earlier source stage) and all 315 guardrails. Strict all-target/all-feature Clippy and formatting passed at the tested source head after test-only iterator fixes. The fifth native test was added after the broader suite; the final Clippy command compiled all targets and the explicit native command ran all five.

The linker emitted its existing large `__eh_frame` warning; all five tests completed successfully. The elapsed test-harness duration is not a rendering performance comparison. OPT-1455 remains open for its required native many-surface before/after workload evidence and remaining qualification.
