use std::{fs, path::PathBuf};

#[test]
fn frame_cache_avoids_post_mutation_expect() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let module = fs::read_to_string(
        manifest_dir.join("src/gui_runtime/native_vello/generic_runtime/composited_base.rs"),
    )
    .expect("composited base presenter should be readable");
    let source = fs::read_to_string(
        manifest_dir.join("src/gui_runtime/native_vello/generic_runtime/composited_base/frame.rs"),
    )
    .expect("composited base frame cache should be readable");
    let ensure_body = source
        .split("pub(super) fn ensure")
        .nth(1)
        .and_then(|tail| tail.split("fn new").next())
        .expect("CompositedBaseFrame::ensure should be present");

    assert!(
        module.contains("mod frame;")
            && module.contains("pub(super) use frame::{")
            && module.contains("CompositedBaseFrameRetirement"),
        "composited base presentation should delegate cached texture ownership to the frame module"
    );
    assert!(
        module.contains("use super::{GpuSurfaceRenderer, RenderFrameProfile, RenderSurfacePixelSize, gpu_surface};")
            && module.contains(
                "use crate::runtime::{GpuShaderPresentationUniformUpdate, PaintPrimitive, SurfacePaintPlan};",
            )
            && module.contains("profile.measure(||")
            && module.contains("use vello::{util::RenderSurface, wgpu};")
            && !module.starts_with("use super::*;"),
        "composited base presenter should name GPU renderer, profile, surface size, runtime, profiled timing, Vello, and WGPU dependencies"
    );
    assert!(
        !module.contains("struct CompositedBaseFrame")
            && source.contains("struct CompositedBaseFrame"),
        "cached composited base texture state should stay out of the presenter module"
    );
    assert!(
        source.contains("use super::super::device::wgpu_device_id;")
            && source.contains("use vello::wgpu;")
            && !source.starts_with("use super::*;"),
        "composited base frame cache should name its WGPU and device-id dependencies"
    );
    assert!(
        ensure_body.contains("composited_base_frame_ensure_decision")
            && ensure_body.contains("retired.is_some()")
            && ensure_body.contains("CompositedBaseFrameEnsureOutcome::Vetoed")
            && source.contains("target_generation")
            && source.contains("completion_identity"),
        "CompositedBaseFrame::ensure should make exact allow/reuse/veto decisions before ownership mutation"
    );
    assert!(
        source.contains("block_dimensions()")
            && source.contains("block_copy_size(None)")
            && source.contains("checked_mul")
            && !source.contains("PollType::Wait")
            && !ensure_body.contains(".expect(")
            && !ensure_body.contains(".unwrap("),
        "CompositedBaseFrame should use checked requested footprint accounting without waits or panic shortcuts"
    );
}
