use std::{fs, path::PathBuf};

#[test]
fn native_gpu_surface_overlay_uniforms_stay_in_focused_module() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let renderer = fs::read_to_string(
        manifest_dir.join("src/gui_runtime/native_vello/generic_runtime/gpu_surface.rs"),
    )
    .expect("GPU surface renderer module should be readable");
    let passes = fs::read_to_string(
        manifest_dir.join("src/gui_runtime/native_vello/generic_runtime/gpu_surface/passes.rs"),
    )
    .expect("GPU surface pass module should be readable");
    let overlays = fs::read_to_string(
        manifest_dir.join("src/gui_runtime/native_vello/generic_runtime/gpu_surface/overlays.rs"),
    )
    .expect("GPU surface overlay uniform module should be readable");

    assert!(
        renderer.contains("mod overlays;") && renderer.contains("use overlays::vertical_overlays;"),
        "GPU surface renderer should route overlay uniform packing through a focused module"
    );
    let production_renderer = renderer
        .split("#[cfg(test)]")
        .next()
        .expect("renderer production source should precede tests");
    for wildcard in [
        "use super::*;",
        "use encoding::*;",
        "use gpu_surface_types::*;",
        "use overlays::*;",
        "use passes::*;",
    ] {
        assert!(
            !production_renderer.contains(wildcard),
            "GPU surface renderer root should use explicit imports instead of `{wildcard}`"
        );
    }
    assert!(
        !passes.contains("fn vertical_overlays")
            && !passes.contains("fn normalized_ratio")
            && overlays.contains("fn vertical_overlays")
            && overlays.contains("fn normalized_ratio")
            && overlays.contains("fn rgba_to_float"),
        "overlay uniform packing should not live with WGPU render-pass and scissor setup"
    );
}
