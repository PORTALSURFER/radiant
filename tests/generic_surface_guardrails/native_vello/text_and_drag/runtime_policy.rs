use super::*;

#[test]
fn native_vello_runtime_does_not_hide_dead_code() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let runtime_dir = manifest_dir.join("src/gui_runtime/native_vello");
    let violations = rust_sources_under(&runtime_dir)
        .into_iter()
        .filter(|path| {
            fs::read_to_string(path)
                .unwrap_or_else(|err| panic!("failed to read {}: {err}", path.display()))
                .contains("#[allow(dead_code)]")
        })
        .map(|path| relative_path(&manifest_dir, &path))
        .collect::<Vec<_>>();

    assert!(
        violations.is_empty(),
        "native Vello runtime modules should export, test, or remove code instead of hiding dead-code warnings:\n{}",
        violations.join("\n")
    );
}
