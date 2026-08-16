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
            && !refresh_inner.contains("fresh_surface")
            && !refresh_inner.contains("FreshSurface")
            && !refresh_inner.contains("advance_fresh_surface_active_generation"),
        "refresh_with_scope_inner must keep fresh-surface preparation and bookkeeping out of the existing direct publication path"
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
