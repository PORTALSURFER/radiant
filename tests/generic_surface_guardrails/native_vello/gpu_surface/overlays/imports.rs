use super::*;

#[test]
fn native_gpu_surface_modules_use_explicit_imports() {
    let renderer =
        gpu_surface_source("src/gui_runtime/native_vello/generic_runtime/gpu_surface.rs");
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

    for path in [
        "src/gui_runtime/native_vello/generic_runtime/gpu_surface/encoding.rs",
        "src/gui_runtime/native_vello/generic_runtime/gpu_surface/atlas.rs",
        "src/gui_runtime/native_vello/generic_runtime/gpu_surface/overlays.rs",
        "src/gui_runtime/native_vello/generic_runtime/gpu_surface/passes.rs",
        "src/gui_runtime/native_vello/generic_runtime/gpu_surface/pipeline.rs",
        "src/gui_runtime/native_vello/generic_runtime/gpu_surface/signal_pipeline.rs",
        "src/gui_runtime/native_vello/generic_runtime/gpu_surface/signal.rs",
        "src/gui_runtime/native_vello/generic_runtime/gpu_surface/stats.rs",
        "src/gui_runtime/native_vello/generic_runtime/gpu_surface/custom_shader.rs",
        "src/gui_runtime/native_vello/generic_runtime/gpu_surface/custom_shader/binding.rs",
        "src/gui_runtime/native_vello/generic_runtime/gpu_surface/custom_shader/diagnostics.rs",
        "src/gui_runtime/native_vello/generic_runtime/gpu_surface/custom_shader/pipeline.rs",
        "src/gui_runtime/native_vello/generic_runtime/gpu_surface/gpu_surface_types.rs",
        "src/gui_runtime/native_vello/generic_runtime/gpu_surface/gpu_surface_types/pipeline.rs",
        "src/gui_runtime/native_vello/generic_runtime/gpu_surface/gpu_surface_types/composite.rs",
        "src/gui_runtime/native_vello/generic_runtime/gpu_surface/gpu_surface_types/texture.rs",
        "src/gui_runtime/native_vello/generic_runtime/gpu_surface/gpu_surface_types/custom_shader.rs",
        "src/gui_runtime/native_vello/generic_runtime/gpu_surface/gpu_surface_types/signal.rs",
        "src/gui_runtime/native_vello/generic_runtime/gpu_surface/gpu_surface_types/signal/cache_key.rs",
        "src/gui_runtime/native_vello/generic_runtime/gpu_surface/resources/cache.rs",
        "src/gui_runtime/native_vello/generic_runtime/gpu_surface/resources/atlas.rs",
        "src/gui_runtime/native_vello/generic_runtime/gpu_surface/resources/pipeline.rs",
        "src/gui_runtime/native_vello/generic_runtime/gpu_surface/resources/signal.rs",
        "src/gui_runtime/native_vello/generic_runtime/gpu_surface/resources/signal/summary.rs",
        "src/gui_runtime/native_vello/generic_runtime/gpu_surface/visibility.rs",
        "src/gui_runtime/native_vello/generic_runtime/gpu_surface/visibility/occlusion.rs",
    ] {
        let source = gpu_surface_source(path);
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
}
