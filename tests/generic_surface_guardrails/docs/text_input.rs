use super::*;

#[test]
fn api_docs_define_advanced_text_input_capability_boundary() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let docs = fs::read_to_string(manifest_dir.join("docs/API.md"))
        .expect("docs/API.md should be readable");
    let normalized_docs = docs.split_whitespace().collect::<Vec<_>>().join(" ");
    let state_tests =
        fs::read_to_string(manifest_dir.join("src/widgets/primitives/text_input/tests/state.rs"))
            .expect("text input state tests should be readable");

    for required in [
        "Advanced text input capabilities are intentionally staged behind this single-line contract",
        "generic text-area capability with layout-aware vertical navigation",
        "Undo and redo should be widget-local edit history",
        "separate from application undo stacks",
        "first-class masked text-input mode",
        "automation value text should be masked",
        "copying selected text should be disabled by default",
        "Native IME composition belongs at the platform adapter boundary",
        "preedit/commit/cancel events",
        "Bidirectional text and complex shaping belong to renderer text layout",
        "`TextInputState` continues to store logical Unicode-scalar positions",
    ] {
        assert!(
            normalized_docs.contains(required),
            "docs/API.md should define advanced text input boundary: {required}"
        );
    }

    assert!(
        state_tests
            .contains("Explicitly not covered because Radiant does not yet expose the feature")
            && state_tests.contains("multiline Up/Down layout-aware navigation")
            && state_tests.contains("undo/redo grouping")
            && state_tests.contains("password masking mode")
            && state_tests.contains("platform IME composition and bidirectional shaping behavior"),
        "single-line text input tests should keep advanced capability exclusions visible"
    );
}
