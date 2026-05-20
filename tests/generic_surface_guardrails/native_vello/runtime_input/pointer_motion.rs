use super::*;

#[test]
fn native_pointer_motion_tests_stay_grouped_by_redraw_concern() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let root = fs::read_to_string(
        manifest_dir.join("src/gui_runtime/native_vello/generic_runtime/tests/pointer_motion.rs"),
    )
    .expect("native pointer-motion test root should be readable");
    let fixtures = fs::read_to_string(
        manifest_dir
            .join("src/gui_runtime/native_vello/generic_runtime/tests/pointer_motion/fixtures.rs"),
    )
    .expect("native pointer-motion fixtures should be readable");
    let redraw = fs::read_to_string(
        manifest_dir
            .join("src/gui_runtime/native_vello/generic_runtime/tests/pointer_motion/redraw.rs"),
    )
    .expect("native pointer-motion redraw tests should be readable");
    let paint_only =
        fs::read_to_string(manifest_dir.join(
            "src/gui_runtime/native_vello/generic_runtime/tests/pointer_motion/paint_only.rs",
        ))
        .expect("native pointer-motion paint-only tests should be readable");

    assert!(
        root.contains("mod fixtures;")
            && root.contains("mod redraw;")
            && root.contains("mod paint_only;")
            && !root.contains("struct PointerMoveWidget")
            && !root.contains("fn pointer_move_inside_same_widget"),
        "native pointer-motion test root should index focused redraw groups instead of owning fixtures and all cases"
    );
    assert!(
        fixtures.contains("struct PointerMoveWidget")
            && fixtures.contains("struct PaintOnlyPointerMoveWidget")
            && redraw
                .contains("fn pointer_move_inside_same_widget_does_not_request_redundant_redraw")
            && redraw.contains("fn local_pointer_move_state_inside_same_widget_requests_redraw")
            && paint_only.contains(
                "fn paint_only_pointer_move_overlay_skips_scene_rebuild_after_hover_enters"
            ),
        "native pointer-motion tests should stay grouped by fixtures, redraw decisions, and paint-only overlay behavior"
    );
}
