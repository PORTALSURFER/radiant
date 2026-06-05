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
            "src/gui_runtime/native_vello/generic_runtime/gpu_surface/visibility/occlusion.rs",
        ))
        .expect("GPU surface visibility occlusion module should be readable");

    assert!(
        visibility.contains("mod occlusion;")
            && visibility.contains("visible_rects_after_occlusion")
            && visibility.contains("gpu_surface_opaque_suffix_regions_into(")
            && visibility.contains("&mut scratch.occlusion_regions")
            && !visibility.contains("const OPAQUE_SUFFIX_OCCLUSION_ALPHA")
            && !visibility.contains("PaintPrimitive::FillRect(fill)"),
        "GPU surface visibility should delegate opaque suffix collection"
    );
    assert!(
        occlusion.contains("const OPAQUE_SUFFIX_OCCLUSION_ALPHA")
            && occlusion.contains("fn gpu_surface_opaque_suffix_regions")
            && occlusion.contains("PaintPrimitive::FillRect(fill)")
            && occlusion.contains("intersect_rect(surface_rect, fill.rect)"),
        "opaque suffix occlusion filtering should live in visibility/occlusion.rs"
    );
}
