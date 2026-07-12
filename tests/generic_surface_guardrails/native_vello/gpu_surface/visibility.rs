use std::{fs, path::PathBuf};

#[test]
fn native_gpu_surface_visibility_occlusion_stays_focused() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let visibility = fs::read_to_string(
        manifest_dir.join("src/gui_runtime/native_vello/generic_runtime/gpu_surface/visibility.rs"),
    )
    .expect("GPU surface visibility module should be readable");
    let occlusion =
        fs::read_to_string(manifest_dir.join(
            "src/gui_runtime/native_vello/generic_runtime/runtime_helpers/surface_occlusion.rs",
        ))
        .expect("shared surface occlusion module should be readable");
    let interaction = fs::read_to_string(manifest_dir.join(
        "src/gui_runtime/native_vello/generic_runtime/runtime_helpers/gpu_surface_regions.rs",
    ))
    .expect("GPU surface interaction-region module should be readable");

    assert!(
        visibility.contains("visible_rects_after_occlusion")
            && visibility.contains("surface_occlusion_regions_into(")
            && visibility.contains("&mut scratch.occlusion_regions")
            && !visibility.contains("const OPAQUE_SUFFIX_OCCLUSION_ALPHA")
            && !visibility.contains("PaintPrimitive::FillRect(fill)"),
        "GPU surface visibility should delegate shared surface occlusion collection"
    );
    assert!(
        occlusion.contains("const OPAQUE_SUFFIX_OCCLUSION_ALPHA")
            && occlusion.contains("fn surface_occlusion_regions")
            && occlusion.contains("Self::Exact => u8::MAX")
            && occlusion.contains("Self::GpuCompositor => OPAQUE_SUFFIX_OCCLUSION_ALPHA")
            && occlusion.contains("PaintPrimitive::FillRect(fill)")
            && occlusion.contains("PaintPrimitive::OverlayPanel(panel)")
            && occlusion.contains("PaintPrimitive::ClipStart(clip)")
            && occlusion.contains("append_rect_outside_clip(surface_rect, clip, regions)")
            && occlusion.contains("update_clip_stack(primitive, clip_stack)")
            && occlusion.contains("clipped_occlusion_region(surface_rect, fill.rect, clip_stack)"),
        "surface clip and opaque suffix occlusion should live in runtime_helpers/surface_occlusion.rs"
    );
    assert!(
        interaction.contains("surface_occlusion_regions_into(")
            && interaction.contains("primitives.get(..index)")
            && interaction.contains("&mut scratch.clip_stack")
            && !interaction.contains("const OPAQUE_SUFFIX_OCCLUSION_ALPHA")
            && !interaction.contains("fn push_opaque_fill_rects"),
        "GPU interaction regions should share render-time surface occlusion"
    );
}
