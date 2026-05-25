use super::*;

#[test]
fn application_id_generation_keeps_policy_and_tests_focused() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let ids = fs::read_to_string(manifest_dir.join("src/application/ids.rs"))
        .expect("application id generation module should be readable");
    let tests = fs::read_to_string(manifest_dir.join("src/application/ids/tests.rs"))
        .expect("application id generation tests should be readable");

    assert!(
        ids.contains("pub(in crate::application) struct IdGenerator")
            && ids.contains("enum ReservedIds")
            && ids.contains("fn reserved_id_range(reserved: &[NodeId])")
            && ids.contains("pub(in crate::application) fn scoped_key_id")
            && ids.contains("#[path = \"ids/tests.rs\"]")
            && !ids.contains("fn id_generator_skips_dense_reserved_runs_after_collision"),
        "application id allocation should live in application/ids.rs while behavior tests stay delegated"
    );
    assert!(
        tests.contains("fn id_generator_skips_dense_reserved_runs_after_collision")
            && tests.contains("fn id_generator_keeps_sorted_reserved_ids_for_small_sets")
            && tests.contains("fn id_generator_skips_probing_after_reserved_range_is_exhausted"),
        "application id generation behavior coverage should live in application/ids/tests.rs"
    );
}
