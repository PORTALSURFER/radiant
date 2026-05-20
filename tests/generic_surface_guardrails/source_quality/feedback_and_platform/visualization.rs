use super::*;

#[test]
fn timeline_visualization_state_uses_named_parts_for_large_projection_buckets() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let timeline_dir = manifest_dir.join("src/gui/visualization/timeline");
    let mut violations = Vec::new();

    for path in [
        timeline_dir.join("edit.rs"),
        timeline_dir.join("surface.rs"),
    ] {
        let source = fs::read_to_string(&path)
            .unwrap_or_else(|err| panic!("failed to read {}: {err}", path.display()));
        if source.contains("too_many_arguments") {
            violations.push(relative_path(&manifest_dir, &path));
        }
    }

    assert!(
        violations.is_empty(),
        "timeline visualization state should use named projection parts instead of suppressing long positional constructors:\n{}",
        violations.join("\n")
    );
}
