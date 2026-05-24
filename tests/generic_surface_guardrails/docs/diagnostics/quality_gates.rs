use super::{normalized, read_project_file};

#[test]
fn clippy_quality_gate_is_documented_without_blanket_complexity_allow() {
    let docs = read_project_file("docs/API.md");
    let normalized_docs = normalized(&docs);
    let architecture = read_project_file("docs/ARCHITECTURE.md");
    let lib = read_project_file("src/lib.rs");

    assert!(
        docs.contains("cargo clippy --all-targets --all-features -- -D warnings"),
        "API docs should document the all-target Clippy quality gate"
    );
    assert!(
        docs.contains("cargo doc --no-deps")
            && normalized_docs.contains("rustdoc with broken intra-doc links denied")
            && architecture.contains("cargo doc --no-deps"),
        "API docs and architecture docs should include the rustdoc validation gate"
    );
    assert!(
        docs.contains("cargo test --doc")
            && normalized_docs.contains("doctests for public documentation examples")
            && architecture.contains("cargo test --doc"),
        "API docs and architecture docs should include the doctest validation gate"
    );
    assert!(
        !lib.contains("clippy::type_complexity"),
        "Radiant should not hide callback-shape drift behind a crate-level type_complexity allow"
    );
}

#[test]
fn runtime_diagnostics_use_tracing_outside_explicit_profile_artifacts() {
    let diagnostic_sources = [
        "src/application/runtime/threading.rs",
        "src/application/runtime/timer.rs",
        "src/application/runtime/subscription.rs",
        "src/gui_runtime/native_vello/text_renderer.rs",
    ];

    for source_path in diagnostic_sources {
        let source = read_project_file(source_path);
        assert!(
            !source.contains("eprintln!"),
            "{source_path} should route ordinary runtime diagnostics through tracing"
        );
    }

    let startup_profile = read_project_file("src/gui_runtime/native_vello/startup/logging.rs");
    assert!(
        startup_profile.contains("RADIANT_NATIVE_STARTUP_PROFILE")
            && startup_profile.contains("eprintln!"),
        "explicit startup profile artifacts may keep their opt-in stderr output"
    );
}
