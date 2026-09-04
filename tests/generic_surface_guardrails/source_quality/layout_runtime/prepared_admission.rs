use super::*;

#[test]
fn direct_runtime_refresh_and_relayout_do_not_publish_prepared_candidates() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let refresh = fs::read_to_string(manifest_dir.join("src/runtime/controller/refresh.rs"))
        .expect("runtime refresh source should be readable");
    let layout = fs::read_to_string(manifest_dir.join("src/runtime/controller/state/layout.rs"))
        .expect("runtime layout source should be readable");

    let refresh_inner = refresh
        .split_once("fn refresh_with_scope_inner")
        .map(|(_, body)| body)
        .expect("refresh_with_scope_inner should remain in the runtime refresh controller");
    assert!(
        !refresh_inner.contains("prepare_runtime_layout_candidate")
            && !refresh_inner.contains("RuntimeLayoutCandidate")
            && !refresh_inner.contains("prepare_fresh_surface_refresh")
            && !refresh_inner.contains("PreparedSurfaceRefresh")
            && !refresh_inner.contains("publish_prepared_surface_refresh"),
        "refresh_with_scope_inner must keep staged fresh-surface preparation and publication out of the direct path"
    );
    assert!(
        refresh_inner.contains("self.relayout_with_traversal(traversal)")
            && refresh_inner
                .contains("self.install_traversal_with_candidate(traversal, candidate)"),
        "refresh_with_scope_inner should retain direct relayout and mounted-state installation"
    );

    assert!(
        !layout.contains("prepare_runtime_layout_candidate")
            && !layout.contains("RuntimeLayoutCandidate")
            && layout.contains("self.layout_engine.layout_with_state_and_source_into(")
            && layout.contains("self.install_traversal_with_candidate(traversal, candidate)"),
        "relayout_with_traversal must remain the direct fallback path"
    );
}

#[test]
fn direct_paint_frame_and_native_paths_do_not_consume_fresh_paint_candidates() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let paths = [
        "src/runtime/controller/context/frame/projection.rs",
        "src/gui_runtime/native_vello/generic_runtime/core.rs",
        "src/gui_runtime/native_vello/generic_runtime/frame_prepare.rs",
        "src/gui_runtime/native_vello/generic_runtime/runner.rs",
    ];

    for relative_path in paths {
        let source = fs::read_to_string(manifest_dir.join(relative_path))
            .expect("direct paint/frame/native source should be readable");
        assert!(
            !source.contains("prepare_fresh_surface_paint")
                && !source.contains("FreshSurfacePaintCandidate"),
            "{relative_path} must keep fresh paint candidate consumption out of direct paths"
        );
    }
}
