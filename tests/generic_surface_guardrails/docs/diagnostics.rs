use super::*;

#[test]
fn clippy_quality_gate_is_documented_without_blanket_complexity_allow() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let docs = fs::read_to_string(manifest_dir.join("docs/API.md"))
        .expect("Radiant API docs should be readable");
    let lib = fs::read_to_string(manifest_dir.join("src/lib.rs"))
        .expect("Radiant lib.rs should be readable");

    assert!(
        docs.contains("cargo clippy --all-targets --all-features -- -D warnings"),
        "API docs should document the all-target Clippy quality gate"
    );
    assert!(
        !lib.contains("clippy::type_complexity"),
        "Radiant should not hide callback-shape drift behind a crate-level type_complexity allow"
    );
}

#[test]
fn runtime_diagnostics_use_tracing_outside_explicit_profile_artifacts() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let diagnostic_sources = [
        "src/application/runtime/threading.rs",
        "src/application/runtime/timer.rs",
        "src/application/runtime/subscription.rs",
        "src/gui_runtime/native_vello/text_renderer.rs",
    ];

    for source_path in diagnostic_sources {
        let source = fs::read_to_string(manifest_dir.join(source_path))
            .unwrap_or_else(|err| panic!("{source_path} should be readable: {err}"));
        assert!(
            !source.contains("eprintln!"),
            "{source_path} should route ordinary runtime diagnostics through tracing"
        );
    }

    let startup_profile =
        fs::read_to_string(manifest_dir.join("src/gui_runtime/native_vello/startup/logging.rs"))
            .expect("native startup profile logging should be readable");
    assert!(
        startup_profile.contains("RADIANT_NATIVE_STARTUP_PROFILE")
            && startup_profile.contains("eprintln!"),
        "explicit startup profile artifacts may keep their opt-in stderr output"
    );
}

#[test]
fn api_docs_describe_text_cache_frame_diagnostics() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let docs = fs::read_to_string(manifest_dir.join("docs/API.md"))
        .expect("Radiant API docs should be readable");
    let runtime_diagnostics = fs::read_to_string(manifest_dir.join("src/runtime/diagnostics.rs"))
        .expect("runtime diagnostics models should be readable");
    let native_diagnostics = fs::read_to_string(
        manifest_dir.join("src/gui_runtime/native_vello/generic_runtime/present/diagnostics.rs"),
    )
    .expect("native frame diagnostics projection should be readable");
    let render_profile = fs::read_to_string(
        manifest_dir.join("src/gui_runtime/native_vello/generic_runtime/render_profile.rs"),
    )
    .expect("native render profile should be readable");

    assert!(
        docs.contains("NativeFrameDiagnostics::text")
            && docs.contains("text layout-cache hits, misses,")
            && docs.contains("text atom-cache activity")
            && docs.contains("shaping-sensitive run/scalar counts")
            && docs.contains("fallback/missing glyph counts"),
        "API docs should describe native text cache diagnostics"
    );
    assert!(
        runtime_diagnostics.contains("pub struct NativeTextDiagnostics")
            && runtime_diagnostics.contains("layout_cache_hits")
            && runtime_diagnostics.contains("atom_cache_evictions")
            && runtime_diagnostics.contains("unsupported_shaping_runs")
            && runtime_diagnostics.contains("unsupported_shaping_scalars")
            && runtime_diagnostics.contains("fallback_glyphs")
            && runtime_diagnostics.contains("missing_glyphs"),
        "runtime diagnostics should expose structured native text cache counters"
    );
    assert!(
        native_diagnostics.contains("text: crate::runtime::NativeTextDiagnostics")
            && native_diagnostics.contains("layout_cache_hits: parts.text_stats.layout_hits")
            && native_diagnostics.contains("atom_cache_evictions: parts.text_stats.atom_evictions")
            && native_diagnostics
                .contains("unsupported_shaping_runs: parts.text_stats.unsupported_shaping_runs")
            && native_diagnostics.contains(
                "unsupported_shaping_scalars: parts.text_stats.unsupported_shaping_scalars"
            )
            && native_diagnostics.contains("fallback_glyphs: parts.text_stats.fallback_glyphs")
            && native_diagnostics.contains("missing_glyphs: parts.text_stats.missing_glyphs")
            && native_diagnostics.contains("pub(super) struct NativeFrameDiagnosticsParts"),
        "native frame diagnostics should project text renderer cache counters"
    );
    assert!(
        render_profile.contains("text_unsupported_shaping_runs")
            && render_profile.contains("text_unsupported_shaping_scalars"),
        "native render profile should include shaping-limit text counters"
    );
}

#[test]
fn api_docs_describe_custom_shader_frame_diagnostics() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let docs = fs::read_to_string(manifest_dir.join("docs/API.md"))
        .expect("Radiant API docs should be readable");
    let runtime_diagnostics = fs::read_to_string(manifest_dir.join("src/runtime/diagnostics.rs"))
        .expect("runtime diagnostics models should be readable");
    let native_diagnostics = fs::read_to_string(
        manifest_dir.join("src/gui_runtime/native_vello/generic_runtime/present/diagnostics.rs"),
    )
    .expect("native frame diagnostics projection should be readable");
    let render_profile = fs::read_to_string(
        manifest_dir.join("src/gui_runtime/native_vello/generic_runtime/render_profile.rs"),
    )
    .expect("native render profile should be readable");

    for required in [
        "custom shader pipeline rebuilds",
        "`NativeGpuSurfaceDiagnostics::custom_shader_surfaces_rendered`",
        "`custom_shader_pipeline_rebuilds`",
        "`custom_shader_binding_rebuilds`",
        "`custom_shader_binding_cache_hits`",
    ] {
        assert!(
            docs.contains(required),
            "API docs should describe custom shader frame diagnostics with `{required}`"
        );
    }
    for required in [
        "custom_shader_surfaces_rendered",
        "custom_shader_pipeline_rebuilds",
        "custom_shader_binding_rebuilds",
        "custom_shader_binding_cache_hits",
    ] {
        assert!(
            runtime_diagnostics.contains(required)
                && native_diagnostics.contains(required)
                && render_profile.contains(required),
            "custom shader diagnostic field `{required}` should flow through public diagnostics and the render profile"
        );
    }
}
