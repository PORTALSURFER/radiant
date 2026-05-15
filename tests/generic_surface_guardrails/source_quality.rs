//! Source-quality guardrails for focused modules and readable public models.

use std::{fs, path::PathBuf};

use super::relative_path;

#[test]
fn application_view_lowering_keeps_container_defaults_focused() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let module = fs::read_to_string(manifest_dir.join("src/application/view_node.rs"))
        .expect("application view node module should be readable");
    let lowering = fs::read_to_string(manifest_dir.join("src/application/view_node/lowering.rs"))
        .expect("application view lowering should be readable");
    let defaults =
        fs::read_to_string(manifest_dir.join("src/application/view_node/lowering_defaults.rs"))
            .expect("application view lowering defaults should be readable");

    assert!(
        module.contains("mod lowering_defaults;")
            && lowering.contains("ViewNodeContainerDefaults::new("),
        "view lowering should consume container defaults from a focused helper"
    );
    assert!(
        !lowering.contains("DEFAULT_STYLED_CONTAINER_PADDING")
            && defaults.contains("DEFAULT_STYLED_CONTAINER_PADDING")
            && defaults.contains("fn default_container_padding")
            && defaults.contains("fn base_policy"),
        "declarative container default policy should stay outside the main view lowering match"
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

#[test]
fn editable_tree_rows_use_named_parts_instead_of_boolean_constructor_lists() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let source_path = manifest_dir.join("src/gui/list/editable/row.rs");
    let source = fs::read_to_string(&source_path)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", source_path.display()));

    assert!(
        source.contains("pub struct EditableTreeRowParts")
            && source.contains("pub fn from_parts(parts: EditableTreeRowParts) -> Self"),
        "editable tree rows should expose a named parts object for readable public construction"
    );
    assert!(
        !source.contains("too_many_arguments"),
        "editable tree rows should not hide long positional constructors behind clippy suppressions"
    );
}

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

#[test]
fn layout_virtual_cache_invalidation_stays_with_cache_types() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let engine = fs::read_to_string(manifest_dir.join("src/gui/layout_core/engine/mod.rs"))
        .expect("layout engine module should be readable");
    let cache = fs::read_to_string(manifest_dir.join("src/gui/layout_core/engine/cache.rs"))
        .expect("layout cache module should be readable");

    assert!(
        engine.contains("invalidate_virtual_cache_for(&mut self.virtual_cache, node_id)")
            && engine.contains("invalidate_virtual_cache_for_any(&mut self.virtual_cache"),
        "layout engine should delegate virtual-cache invalidation to the cache module"
    );
    assert!(
        !engine.contains("fn invalidate_virtual_cache_for_any")
            && !engine.contains("entry.dependencies.iter()"),
        "layout engine orchestration should not own cached virtual-metric dependency scans"
    );
    assert!(
        cache.contains("fn invalidate_virtual_cache_for(")
            && cache.contains("fn invalidate_virtual_cache_for_any(")
            && cache.contains("impl CachedVirtualMetrics")
            && cache.contains("fn depends_on"),
        "virtual cache dependency invalidation should live with cached metric types"
    );
}
