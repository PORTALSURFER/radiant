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

The linker emitted its existing large `__eh_frame` warning; all five tests completed successfully. The elapsed test-harness duration is not a rendering performance comparison. The controlled many-surface comparison below supplies the native before/after pipeline-count evidence; broader frame-latency qualification remains tracked by OPT-1452 and OPT-1460.

## Controlled native before/after comparison

The identical ignored fixture retained here ran successfully against:

- Baseline production `a889af816a7f24c4cc442b60578027994c7da6bd`; test-only harness commit `233d5b6816e46b4496705e77df3849e36f1949b9`.
- Candidate production `9fd0559d594faeac920a5534a804766004ce56b3`; test-only harness commit `ac933cb193cdfc8ce89c363109135dadb8606afd`.

Each harness commit adds only this fixture and a `#[cfg(test)]` module declaration. SHA-256 of the identical fixture is `fb6dcf434a73c0b3efd2d033a5dd7566c3e009481742e8e59e67b6779e46067e`.

Both observed adapters were Apple M5 Pro, Metal, IntegratedGpu. Sixteen simultaneously active disjoint tiles created 16 pipelines on the baseline and one on the candidate; both retained 16 bindings. The warm pass rebuilt no pipeline or binding and wrote no payload. The targeted update made one 16-byte static write and two presentation writes totaling 32 bytes on both. All 16 recorded tile-center RGBA values match exactly between baseline and candidate, including the updated green tile and unaffected neighbors. Each fixture also asserts its expected cold/warm counters, residency and reference tile colors.

Command on each clean harness worktree (touch `src/lib.rs` when switching the shared target):

```sh
CARGO_INCREMENTAL=0 CARGO_TARGET_DIR=/Users/portalsurfer/.codex/radiant-validation/target RADIANT_EXPECTED_SHADER_PIPELINES=16 RADIANT_COMPARISON_LABEL=baseline cargo test --locked --all-features --lib offscreen_tile_comparison -- --ignored --test-threads=1 --nocapture
```

The candidate uses `RADIANT_EXPECTED_SHADER_PIPELINES=1` and label `candidate`; all other workload/test inputs are identical. Raw logs retain JSON observations and adapter details. This proves reduced pipeline creation and preserved visual/binding behavior for this native workload. It does not measure latency, timestamp durations, frame tails or foreground window behavior.
