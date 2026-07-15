use super::*;

#[test]
fn confirm_dialogs_use_named_parts_for_public_prompt_fields() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let source_path = manifest_dir.join("src/runtime/platform.rs");
    let source = fs::read_to_string(&source_path)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", source_path.display()));
    let runtime = fs::read_to_string(manifest_dir.join("src/runtime/mod.rs"))
        .expect("runtime module should be readable");
    let prelude = public_prelude_source(&manifest_dir);

    assert!(
        source.contains("pub struct ConfirmDialogParts")
            && source.contains("pub fn from_parts(parts: ConfirmDialogParts) -> Self"),
        "confirmation dialogs should expose named parts for title, message, level, and buttons"
    );
    assert!(
        source.contains("Self::from_parts(ConfirmDialogParts {")
            && runtime.contains("ConfirmDialogParts")
            && !prelude.contains("ConfirmDialogParts"),
        "confirmation dialog parts should remain available from runtime ownership without entering the common prelude"
    );
}
