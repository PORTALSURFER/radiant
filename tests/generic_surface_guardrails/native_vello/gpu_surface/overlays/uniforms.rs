use super::*;

#[test]
fn native_gpu_surface_overlay_uniforms_stay_in_focused_module() {
    let renderer =
        gpu_surface_source("src/gui_runtime/native_vello/generic_runtime/gpu_surface.rs");
    let passes =
        gpu_surface_source("src/gui_runtime/native_vello/generic_runtime/gpu_surface/passes.rs");
    let overlays =
        gpu_surface_source("src/gui_runtime/native_vello/generic_runtime/gpu_surface/overlays.rs");
    let atlas =
        gpu_surface_source("src/gui_runtime/native_vello/generic_runtime/gpu_surface/atlas.rs");

    assert!(
        renderer.contains("mod overlays;")
            && atlas.contains("use super::overlays::vertical_overlays;"),
        "GPU surface atlas rendering should route overlay uniform packing through a focused module"
    );
    assert!(
        !passes.contains("fn vertical_overlays")
            && !passes.contains("fn normalized_ratio")
            && overlays.contains("fn vertical_overlays")
            && overlays.contains("struct VerticalOverlayUniforms")
            && overlays.contains("struct VerticalOverlayParts")
            && overlays.contains("fn vertical_overlay_parts")
            && !overlays.contains("type VerticalOverlayUniforms = (")
            && !atlas.contains("let (overlay_ratios, overlay_widths, overlay_colors)")
            && overlays.contains("fn normalized_ratio")
            && overlays.contains("fn rgba_to_float"),
        "overlay uniform packing should use named uniforms and parts in its focused module instead of positional tuples or WGPU render-pass setup"
    );
}
