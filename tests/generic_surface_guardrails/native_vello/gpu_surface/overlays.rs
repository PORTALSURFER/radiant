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
    let encoding = fs::read_to_string(
        manifest_dir.join("src/gui_runtime/native_vello/generic_runtime/gpu_surface/encoding.rs"),
    )
    .expect("GPU surface encoding module should be readable");
    let atlas = fs::read_to_string(
        manifest_dir.join("src/gui_runtime/native_vello/generic_runtime/gpu_surface/atlas.rs"),
    )
    .expect("GPU surface atlas renderer module should be readable");
    let pipeline = fs::read_to_string(
        manifest_dir.join("src/gui_runtime/native_vello/generic_runtime/gpu_surface/pipeline.rs"),
    )
    .expect("GPU surface pipeline module should be readable");
    let signal_pipeline = fs::read_to_string(
        manifest_dir
            .join("src/gui_runtime/native_vello/generic_runtime/gpu_surface/signal_pipeline.rs"),
    )
    .expect("GPU surface signal pipeline module should be readable");
    let signal = fs::read_to_string(
        manifest_dir.join("src/gui_runtime/native_vello/generic_runtime/gpu_surface/signal.rs"),
    )
    .expect("GPU surface signal renderer module should be readable");
    let stats = fs::read_to_string(
        manifest_dir.join("src/gui_runtime/native_vello/generic_runtime/gpu_surface/stats.rs"),
    )
    .expect("GPU surface stats module should be readable");
    let custom_shader = fs::read_to_string(
        manifest_dir
            .join("src/gui_runtime/native_vello/generic_runtime/gpu_surface/custom_shader.rs"),
    )
    .expect("GPU surface custom shader renderer module should be readable");
    let custom_shader_binding =
        fs::read_to_string(manifest_dir.join(
            "src/gui_runtime/native_vello/generic_runtime/gpu_surface/custom_shader/binding.rs",
        ))
        .expect("GPU surface custom shader binding module should be readable");
    let custom_shader_diagnostics = fs::read_to_string(manifest_dir.join(
        "src/gui_runtime/native_vello/generic_runtime/gpu_surface/custom_shader/diagnostics.rs",
    ))
    .expect("GPU surface custom shader diagnostics module should be readable");
    let custom_shader_pipeline = fs::read_to_string(manifest_dir.join(
        "src/gui_runtime/native_vello/generic_runtime/gpu_surface/custom_shader/pipeline.rs",
    ))
    .expect("GPU surface custom shader pipeline module should be readable");
    let gpu_surface_types = fs::read_to_string(
        manifest_dir
            .join("src/gui_runtime/native_vello/generic_runtime/gpu_surface/gpu_surface_types.rs"),
    )
    .expect("GPU surface type module should be readable");
    let type_pipeline = fs::read_to_string(manifest_dir.join(
        "src/gui_runtime/native_vello/generic_runtime/gpu_surface/gpu_surface_types/pipeline.rs",
    ))
    .expect("GPU surface pipeline type module should be readable");
    let type_composite = fs::read_to_string(manifest_dir.join(
        "src/gui_runtime/native_vello/generic_runtime/gpu_surface/gpu_surface_types/composite.rs",
    ))
    .expect("GPU surface composite type module should be readable");
    let type_texture = fs::read_to_string(manifest_dir.join(
        "src/gui_runtime/native_vello/generic_runtime/gpu_surface/gpu_surface_types/texture.rs",
    ))
    .expect("GPU surface texture type module should be readable");
    let type_custom_shader = fs::read_to_string(
        manifest_dir.join(
            "src/gui_runtime/native_vello/generic_runtime/gpu_surface/gpu_surface_types/custom_shader.rs",
        ),
    )
    .expect("GPU surface custom-shader type module should be readable");
    let type_signal = fs::read_to_string(manifest_dir.join(
        "src/gui_runtime/native_vello/generic_runtime/gpu_surface/gpu_surface_types/signal.rs",
    ))
    .expect("GPU surface signal type module should be readable");
    let type_signal_cache_key = fs::read_to_string(
        manifest_dir.join(
            "src/gui_runtime/native_vello/generic_runtime/gpu_surface/gpu_surface_types/signal/cache_key.rs",
        ),
    )
    .expect("GPU surface signal cache-key type module should be readable");
    let resource_cache = fs::read_to_string(
        manifest_dir
            .join("src/gui_runtime/native_vello/generic_runtime/gpu_surface/resources/cache.rs"),
    )
    .expect("GPU surface resource cache module should be readable");
    let resource_atlas = fs::read_to_string(
        manifest_dir
            .join("src/gui_runtime/native_vello/generic_runtime/gpu_surface/resources/atlas.rs"),
    )
    .expect("GPU surface atlas resource module should be readable");
    let resource_pipeline = fs::read_to_string(
        manifest_dir
            .join("src/gui_runtime/native_vello/generic_runtime/gpu_surface/resources/pipeline.rs"),
    )
    .expect("GPU surface pipeline resource module should be readable");
    let resource_signal = fs::read_to_string(
        manifest_dir
            .join("src/gui_runtime/native_vello/generic_runtime/gpu_surface/resources/signal.rs"),
    )
    .expect("GPU surface signal resource module should be readable");
    let resource_signal_summary = fs::read_to_string(manifest_dir.join(
        "src/gui_runtime/native_vello/generic_runtime/gpu_surface/resources/signal/summary.rs",
    ))
    .expect("GPU surface signal summary resource module should be readable");
    let visibility = fs::read_to_string(
        manifest_dir.join("src/gui_runtime/native_vello/generic_runtime/gpu_surface/visibility.rs"),
    )
    .expect("GPU surface visibility module should be readable");
    let visibility_occlusion =
        fs::read_to_string(manifest_dir.join(
            "src/gui_runtime/native_vello/generic_runtime/gpu_surface/visibility/occlusion.rs",
        ))
        .expect("GPU surface visibility occlusion module should be readable");

    assert!(
        renderer.contains("mod overlays;")
            && atlas.contains("use super::overlays::vertical_overlays;"),
        "GPU surface atlas rendering should route overlay uniform packing through a focused module"
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
    for (path, source) in [
        (
            "src/gui_runtime/native_vello/generic_runtime/gpu_surface/encoding.rs",
            encoding.as_str(),
        ),
        (
            "src/gui_runtime/native_vello/generic_runtime/gpu_surface/atlas.rs",
            atlas.as_str(),
        ),
        (
            "src/gui_runtime/native_vello/generic_runtime/gpu_surface/overlays.rs",
            overlays.as_str(),
        ),
        (
            "src/gui_runtime/native_vello/generic_runtime/gpu_surface/passes.rs",
            passes.as_str(),
        ),
        (
            "src/gui_runtime/native_vello/generic_runtime/gpu_surface/pipeline.rs",
            pipeline.as_str(),
        ),
        (
            "src/gui_runtime/native_vello/generic_runtime/gpu_surface/signal_pipeline.rs",
            signal_pipeline.as_str(),
        ),
        (
            "src/gui_runtime/native_vello/generic_runtime/gpu_surface/signal.rs",
            signal.as_str(),
        ),
        (
            "src/gui_runtime/native_vello/generic_runtime/gpu_surface/stats.rs",
            stats.as_str(),
        ),
        (
            "src/gui_runtime/native_vello/generic_runtime/gpu_surface/custom_shader.rs",
            custom_shader.as_str(),
        ),
        (
            "src/gui_runtime/native_vello/generic_runtime/gpu_surface/custom_shader/binding.rs",
            custom_shader_binding.as_str(),
        ),
        (
            "src/gui_runtime/native_vello/generic_runtime/gpu_surface/custom_shader/diagnostics.rs",
            custom_shader_diagnostics.as_str(),
        ),
        (
            "src/gui_runtime/native_vello/generic_runtime/gpu_surface/custom_shader/pipeline.rs",
            custom_shader_pipeline.as_str(),
        ),
        (
            "src/gui_runtime/native_vello/generic_runtime/gpu_surface/gpu_surface_types.rs",
            gpu_surface_types.as_str(),
        ),
        (
            "src/gui_runtime/native_vello/generic_runtime/gpu_surface/gpu_surface_types/pipeline.rs",
            type_pipeline.as_str(),
        ),
        (
            "src/gui_runtime/native_vello/generic_runtime/gpu_surface/gpu_surface_types/composite.rs",
            type_composite.as_str(),
        ),
        (
            "src/gui_runtime/native_vello/generic_runtime/gpu_surface/gpu_surface_types/texture.rs",
            type_texture.as_str(),
        ),
        (
            "src/gui_runtime/native_vello/generic_runtime/gpu_surface/gpu_surface_types/custom_shader.rs",
            type_custom_shader.as_str(),
        ),
        (
            "src/gui_runtime/native_vello/generic_runtime/gpu_surface/gpu_surface_types/signal.rs",
            type_signal.as_str(),
        ),
        (
            "src/gui_runtime/native_vello/generic_runtime/gpu_surface/gpu_surface_types/signal/cache_key.rs",
            type_signal_cache_key.as_str(),
        ),
        (
            "src/gui_runtime/native_vello/generic_runtime/gpu_surface/resources/cache.rs",
            resource_cache.as_str(),
        ),
        (
            "src/gui_runtime/native_vello/generic_runtime/gpu_surface/resources/atlas.rs",
            resource_atlas.as_str(),
        ),
        (
            "src/gui_runtime/native_vello/generic_runtime/gpu_surface/resources/pipeline.rs",
            resource_pipeline.as_str(),
        ),
        (
            "src/gui_runtime/native_vello/generic_runtime/gpu_surface/resources/signal.rs",
            resource_signal.as_str(),
        ),
        (
            "src/gui_runtime/native_vello/generic_runtime/gpu_surface/resources/signal/summary.rs",
            resource_signal_summary.as_str(),
        ),
        (
            "src/gui_runtime/native_vello/generic_runtime/gpu_surface/visibility.rs",
            visibility.as_str(),
        ),
        (
            "src/gui_runtime/native_vello/generic_runtime/gpu_surface/visibility/occlusion.rs",
            visibility_occlusion.as_str(),
        ),
    ] {
        let production_source = source
            .split("#[cfg(test)]")
            .next()
            .expect("production source should precede tests");
        assert!(
            !production_source.contains("use super::*;")
                && !production_source.contains("use super::super::*;")
                && !production_source.contains("use super::super::super::*;"),
            "{path} should import the GPU-surface dependencies it actually uses"
        );
    }
    assert!(
        !passes.contains("fn vertical_overlays")
            && !passes.contains("fn normalized_ratio")
            && overlays.contains("fn vertical_overlays")
            && overlays.contains("struct VerticalOverlayParts")
            && overlays.contains("fn vertical_overlay_parts")
            && overlays.contains("fn normalized_ratio")
            && overlays.contains("fn rgba_to_float"),
        "overlay uniform packing should use named parts in its focused module instead of living with WGPU render-pass and scissor setup"
    );
}
