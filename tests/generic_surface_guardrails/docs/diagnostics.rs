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

    assert!(
        docs.contains("NativeFrameDiagnostics::text")
            && docs.contains("text layout-cache hits, misses,")
            && docs.contains("text atom-cache activity"),
        "API docs should describe native text cache diagnostics"
    );
    assert!(
        runtime_diagnostics.contains("pub struct NativeTextDiagnostics")
            && runtime_diagnostics.contains("layout_cache_hits")
            && runtime_diagnostics.contains("atom_cache_evictions"),
        "runtime diagnostics should expose structured native text cache counters"
    );
    assert!(
        native_diagnostics.contains("text: crate::runtime::NativeTextDiagnostics")
            && native_diagnostics.contains("layout_cache_hits: parts.text_stats.layout_hits")
            && native_diagnostics.contains("atom_cache_evictions: parts.text_stats.atom_evictions")
            && native_diagnostics.contains("pub(super) struct NativeFrameDiagnosticsParts"),
        "native frame diagnostics should project text renderer cache counters"
    );
}
