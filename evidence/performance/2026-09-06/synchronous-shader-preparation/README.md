# Pinned native synchronous shader-preparation baseline

Observed 6 September 2026 on Apple M5 Pro / Metal, WGPU 29.0.4. All three fresh-process ignored native tests passed at fixture revision 1f9928e941ec753e95ad1e74685c916db60de4d0, based on production main 319e857f7d0add3af441607fe4c61cc5fa9ce55e. The fixture lives in gpu_surface/custom_shader/pipeline/measurement_native_tests.rs and directly measures the production module/layout/pipeline helper phases; it does not call the combined `prepare_custom_shader_pipeline` entry point. Its captured baseline remains phase-level synchronous evidence. The separate worker fixture measures that complete entry point. No production implementation changed.

## Bounded observations

Each process creates a fresh native device, measures one cold builder, three repeated identical builders on the warm device, four bounded distinct-key variants, and intentional invalid WGSL. Warm samples execute the builder again: they are not Radiant cache-hit measurements. The test also confirms the actual production invalid-module diagnostics counter.

Cold builder totals were 2.364, 0.884 and 1.137 ms. Warm identical-builder totals ranged 0.490–1.000 ms. Device setup took 35.929–46.406 ms and is reported separately. The JSON files retain every module, layout, pipeline, validation-pop and total duration; units are nanoseconds. Invalid WGSL reports a typed failure instead of a pipeline.

These are debug-build CPU observations of a small triangle shader, not a frame benchmark, worst-case compiler bound, GPU duration, shader-cache latency, or foreground input-responsiveness result. Driver work can synchronously execute in the builder; moving it to a worker does not make compilation disappear or permit interrupting an in-progress driver call. There were no foreground windows or GPU timestamp queries.

## Reproduction

Run sequentially in three fresh test processes using CARGO_INCREMENTAL=0 and a shared CARGO_TARGET_DIR. Set RADIANT_SHADER_PREPARATION_OUTPUT_DIR to a fresh directory, RADIANT_SHADER_PREPARATION_LABEL=baseline, and RADIANT_SHADER_PREPARATION_SOURCE_REVISION to the verified full fixture head. Then:

```sh
cargo test --locked --all-features --lib records_sync_custom_shader_preparation_phases -- --ignored --test-threads=1 --nocapture
```

Require exactly one executed test and an exclusively created shader-preparation-samples.json. Do not use a short test name with --exact, which would match zero tests. Initial fixture compilation used an unavailable InstanceDescriptor::default constructor; it was corrected to the pinned new_without_display_handle API before these successful runs.

## Next acceptance

OPT-1457 still needs bounded asynchronous preparation implementation, exact device/request fencing, failure/retry and lifecycle tests, candidate behavior/timing comparison, and foreground responsiveness evidence. This baseline is a prerequisite, not ticket completion.
