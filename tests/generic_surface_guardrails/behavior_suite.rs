use super::*;

#[test]
fn behavior_test_suite_is_explicit_and_local_to_generic_surfaces() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let tests_dir = manifest_dir.join("tests");
    let mut test_files = fs::read_dir(&tests_dir)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", tests_dir.display()))
        .map(|entry| {
            entry
                .unwrap_or_else(|err| {
                    panic!("failed to read entry in {}: {err}", tests_dir.display())
                })
                .path()
        })
        .filter(|path| path.extension().and_then(|extension| extension.to_str()) == Some("rs"))
        .map(|path| {
            path.file_name()
                .and_then(|file_name| file_name.to_str())
                .expect("test file should have utf-8 name")
                .to_owned()
        })
        .collect::<Vec<_>>();
    test_files.sort();

    assert_eq!(
        test_files, REQUIRED_BEHAVIOR_TESTS,
        "Radiant integration tests should stay focused on generic layout, runtime, widget, and boundary behavior"
    );
    assert!(
        !tests_dir.join("shots").exists()
            && !manifest_dir
                .join("src/gui_runtime/native_vello/tests")
                .exists()
            && !manifest_dir
                .join("src/gui_runtime/native_vello/tests.rs")
                .exists(),
        "renderer snapshots and backend fixture trees should live with their owning host or backend suite"
    );
}
