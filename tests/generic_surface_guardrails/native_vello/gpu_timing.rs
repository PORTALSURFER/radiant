use super::surface_backend::read_runtime_source;

#[test]
fn native_gpu_timing_is_fixed_capacity_and_encoder_only() {
    let timing = read_runtime_source("src/gui_runtime/native_vello/generic_runtime/gpu_timing.rs");
    let present = read_runtime_source("src/gui_runtime/native_vello/generic_runtime/present.rs");

    assert!(timing.contains("const TIMING_SLOT_COUNT: usize = 4"));
    assert!(timing.contains("[GpuTimingSlotState; TIMING_SLOT_COUNT]"));
    assert!(!timing.contains("Vec<"));
    assert!(!timing.contains("VecDeque<"));
    assert!(!timing.contains("HashMap<"));
    assert_eq!(timing.matches("write_timestamp(").count(), 2);
    for forbidden in [
        "TIMESTAMP_QUERY_INSIDE_PASSES",
        "begin_render_pass",
        "begin_compute_pass",
        "timestamp_writes",
        "RenderPass",
        "ComputePass",
    ] {
        assert!(
            !timing.contains(forbidden),
            "GPU timing must not use pass timestamp paths: {forbidden}"
        );
    }
    assert!(!timing.contains("pub(super) struct NativeGpuTimingPool"));
    assert!(!timing.contains("pub(super) struct NativeGpuTimingSlot"));
    assert!(!timing.contains("pub(super) wgpu::"));
    assert!(!timing.contains("host_observe_frame_gpu_timing"));

    let start = present
        .find("self.start_native_gpu_timing(")
        .expect("present should submit the standalone timing start");
    let scene = present
        .find("render_scene_texture_if_needed(")
        .expect("present should retain the composited scene path");
    let overlay = present
        .find("render_post_gpu_overlay(")
        .expect("present should retain post-GPU overlay composition");
    let end = present
        .find("encode_gpu_timing_end(")
        .expect("present should encode the final timing timestamp");
    let submit = present
        .find("dev_handle.queue.submit(")
        .expect("present should submit the frame command buffer");
    assert!(start < scene && overlay < end && end < submit);
    assert!(present.contains("if render_resize_frame_directly || !self.frame_gpu_timing_enabled"));
}

#[test]
fn native_gpu_timing_callback_is_wake_only_and_delivery_is_event_loop_owned() {
    let timing = read_runtime_source("src/gui_runtime/native_vello/generic_runtime/gpu_timing.rs");
    let event = read_runtime_source("src/gui_runtime/native_vello/runtime_event.rs");
    let lifecycle =
        read_runtime_source("src/gui_runtime/native_vello/generic_runtime/lifecycle.rs");
    let runner = read_runtime_source("src/gui_runtime/native_vello/generic_runtime/runner.rs");

    assert!(event.contains("NativeGpuTimingReady"));
    assert!(timing.contains("send_event(RuntimeUserEvent::NativeGpuTimingReady"));
    assert!(!timing.contains("observe_frame_gpu_timing"));
    assert!(lifecycle.contains("RuntimeUserEvent::NativeGpuTimingReady"));
    assert!(runner.contains("host_observe_frame_gpu_timing(delivery.sample)"));
    assert!(runner.contains("finish_native_gpu_timing_delivery(delivery)"));
}
