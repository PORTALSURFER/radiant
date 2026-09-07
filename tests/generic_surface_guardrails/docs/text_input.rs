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
        "Multiline editing uses the separate controlled `TextEditorWidget`",
        "layout-aware vertical navigation",
        "Durable text values and undo/redo history belong to the application",
        "it does not retain a competing undo stack",
        "Secret mode masks paint content, omits automation values, and denies copying by default",
        "independent explicit policy opt-ins",
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
        state_tests.contains("Covered by dedicated editor, widget, geometry, and adapter tests:")
            && state_tests.contains("multiline Up/Down layout-aware navigation")
            && state_tests.contains("transient grouping and application-owned undo/redo")
            && state_tests.contains("secret masking and automation policy")
            && state_tests.contains("platform IME composition and bidirectional shaping behavior"),
        "single-line state tests should identify the owners of broader text coverage"
    );
    assert!(
        !normalized_docs.contains("Undo and redo should be widget-local edit history"),
        "durable history must remain application-owned"
    );
}
