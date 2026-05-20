use super::*;

#[test]
fn gui_geometry_tests_stay_grouped_by_rect_concern() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let root = fs::read_to_string(manifest_dir.join("src/gui/types/geometry/tests.rs"))
        .expect("gui geometry test root should be readable");
    let bounds =
        fs::read_to_string(manifest_dir.join("src/gui/types/geometry/tests/rect_bounds.rs"))
            .expect("gui geometry bounds tests should be readable");
    let insets =
        fs::read_to_string(manifest_dir.join("src/gui/types/geometry/tests/rect_insets.rs"))
            .expect("gui geometry inset tests should be readable");
    let squares =
        fs::read_to_string(manifest_dir.join("src/gui/types/geometry/tests/rect_squares.rs"))
            .expect("gui geometry square tests should be readable");
    let edges = fs::read_to_string(manifest_dir.join("src/gui/types/geometry/tests/rect_edges.rs"))
        .expect("gui geometry edge tests should be readable");

    assert!(
        root.contains("mod rect_bounds;")
            && root.contains("mod rect_insets;")
            && root.contains("mod rect_squares;")
            && root.contains("mod rect_edges;")
            && !root.contains("fn rect_centered_pixel_square")
            && !root.contains("fn rect_edge_strips_resolve_each_side"),
        "gui geometry test root should index focused rect behavior groups instead of owning all cases"
    );
    assert!(
        bounds.contains("fn rect_clamp_to_limits_rect_to_bounds")
            && bounds.contains("fn point_and_rect_finiteness_helpers_reject_invalid_geometry")
            && insets.contains("fn rect_inset_uniform_saturating_caps_at_half_extents")
            && squares.contains("fn rect_centered_odd_pixel_square_forces_odd_side")
            && edges.contains("fn rect_edge_strips_resolve_each_side"),
        "gui geometry tests should stay grouped by bounds, insets, centered squares, and edge helpers"
    );
}

#[test]
fn public_layout_policy_models_do_not_hide_dead_code() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let model_dir = manifest_dir.join("src/gui/layout_core/model");
    let mut violations = Vec::new();

    for path in [
        model_dir.join("alignment.rs"),
        model_dir.join("container.rs"),
        model_dir.join("virtualization.rs"),
    ] {
        let source = fs::read_to_string(&path)
            .unwrap_or_else(|err| panic!("failed to read {}: {err}", path.display()));
        if source.contains("#[allow(dead_code)]") {
            violations.push(relative_path(&manifest_dir, &path));
        }
    }

    assert!(
        violations.is_empty(),
        "public layout policy models should be exported, tested, or removed instead of hiding dead-code warnings:\n{}",
        violations.join("\n")
    );
}

#[test]
fn linear_layout_hot_path_uses_request_objects_instead_of_argument_suppressions() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let linear_dir = manifest_dir.join("src/gui/layout_core/engine/layout/linear");
    let mut violations = Vec::new();

    for path in [
        linear_dir.join("placement.rs"),
        linear_dir.join("sizing.rs"),
    ] {
        let source = fs::read_to_string(&path)
            .unwrap_or_else(|err| panic!("failed to read {}: {err}", path.display()));
        if source.contains("too_many_arguments") {
            violations.push(relative_path(&manifest_dir, &path));
        }
    }

    assert!(
        violations.is_empty(),
        "linear layout measurement and placement should use cohesive request objects instead of suppressing long parameter lists:\n{}",
        violations.join("\n")
    );
}
