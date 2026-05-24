use super::*;

#[test]
fn layout_constraints_use_named_parts_for_min_max_bounds() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let source_path = manifest_dir.join("src/gui/layout_core/constraints.rs");
    let source = fs::read_to_string(&source_path)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", source_path.display()));
    let tests = fs::read_to_string(manifest_dir.join("src/gui/layout_core/constraints/tests.rs"))
        .expect("layout constraint behavior tests should be readable");
    let module = fs::read_to_string(manifest_dir.join("src/gui/layout_core/mod.rs"))
        .expect("layout module should be readable");

    assert!(
        source.contains("pub struct ConstraintsParts")
            && source.contains("pub fn from_parts(parts: ConstraintsParts) -> Self"),
        "layout constraints should expose named parts for readable min/max bound construction"
    );
    assert!(
        !source.contains("pub fn new(min_w: f32, max_w: f32, min_h: f32, max_h: f32)"),
        "layout constraints should not expose a public four-argument positional constructor"
    );
    assert!(
        source.contains("Self::from_parts(ConstraintsParts {")
            && module.contains("pub use constraints::{Constraints, ConstraintsParts};"),
        "layout constraint constructors and public exports should keep the named-parts path available"
    );
    assert!(
        source.contains("#[path = \"constraints/tests.rs\"]")
            && !source.contains("fn constraints_normalize_invalid_ranges"),
        "layout constraint behavior tests should stay delegated"
    );
    assert!(
        tests.contains("fn constraints_normalize_invalid_ranges"),
        "layout constraint behavior coverage should live in constraints/tests.rs"
    );
}

#[test]
fn layout_tree_nodes_use_named_parts_for_public_tree_construction() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let source_path = manifest_dir.join("src/gui/layout_core/tree.rs");
    let source = fs::read_to_string(&source_path)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", source_path.display()));
    let module = fs::read_to_string(manifest_dir.join("src/gui/layout_core/mod.rs"))
        .expect("layout module should be readable");

    for (parts, from_parts, wrapper) in [
        (
            "pub struct SlotChildParts",
            "pub fn from_parts(parts: SlotChildParts) -> Self",
            "Self::from_parts(SlotChildParts {",
        ),
        (
            "pub struct ContainerNodeParts",
            "pub fn from_parts(parts: ContainerNodeParts) -> Self",
            "Self::from_parts(ContainerNodeParts {",
        ),
        (
            "pub struct WidgetNodeParts",
            "pub fn from_parts(parts: WidgetNodeParts) -> Self",
            "Self::from_parts(WidgetNodeParts {",
        ),
    ] {
        assert!(
            source.contains(parts) && source.contains(from_parts) && source.contains(wrapper),
            "layout tree public nodes should expose named parts and compatibility wrappers for {parts}"
        );
    }
    assert!(
        source.contains("pub fn container_from_parts(parts: ContainerNodeParts) -> Self")
            && source.contains("pub fn widget_from_parts(parts: WidgetNodeParts) -> Self")
            && module.contains("ContainerNodeParts")
            && module.contains("SlotChildParts")
            && module.contains("WidgetNodeParts"),
        "layout tree named parts should be available through the public layout module"
    );
}

#[test]
fn layout_tree_derived_state_keeps_metrics_and_tests_focused() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let derived = fs::read_to_string(manifest_dir.join("src/gui/layout_core/tree/derived.rs"))
        .expect("layout tree derived state module should be readable");
    let tests = fs::read_to_string(manifest_dir.join("src/gui/layout_core/tree/derived/tests.rs"))
        .expect("layout tree derived state tests should be readable");

    assert!(
        derived.contains("fn container_derived_state")
            && derived.contains("KnownMainMetrics")
            && derived.contains("#[path = \"derived/tests.rs\"]")
            && !derived.contains("fn container_precomputes_uniform_main_size_with_extent"),
        "layout tree derived metrics should live in tree/derived.rs while behavior tests stay delegated"
    );
    assert!(
        tests.contains("fn container_precomputes_uniform_main_size_with_extent")
            && tests.contains("fn container_does_not_mark_margin_rows_as_uniform"),
        "layout tree derived behavior coverage should live in tree/derived/tests.rs"
    );
}
