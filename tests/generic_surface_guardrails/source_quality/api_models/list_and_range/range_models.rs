use super::*;

#[test]
fn normalized_ranges_use_named_parts_for_milli_bounds() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let root = fs::read_to_string(manifest_dir.join("src/gui/range.rs"))
        .expect("range root should be readable");
    let source_path = manifest_dir.join("src/gui/range/interval.rs");
    let interval = fs::read_to_string(&source_path)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", source_path.display()));
    let interval_tests = fs::read_to_string(manifest_dir.join("src/gui/range/interval/tests.rs"))
        .expect("normalized range behavior tests should be readable");

    assert!(
        root.contains("mod interval;")
            && root.contains("NormalizedRange")
            && root.contains("NormalizedRangeDrag")
            && root.contains("NormalizedRangeEdge")
            && root.contains("NormalizedRangeParts")
            && !root.contains("pub struct NormalizedRange"),
        "range root should re-export the normalized interval model without owning its implementation"
    );

    assert!(
        interval.contains("pub struct NormalizedRangeParts")
            && interval.contains("pub fn from_parts(parts: NormalizedRangeParts) -> Self")
            && interval.contains("#[path = \"interval/tests.rs\"]")
            && !interval.contains("fn normalized_range_orders_and_clamps_nano_bounds"),
        "normalized ranges should expose named parts for start and end milli-unit bounds while delegating behavior tests"
    );
    assert!(
        interval_tests.contains("fn normalized_range_orders_and_clamps_nano_bounds")
            && interval_tests.contains("fn normalized_range_supports_named_parts_construction"),
        "normalized range behavior coverage should live in range/interval/tests.rs"
    );
    assert!(
        interval.contains("Self::from_parts(NormalizedRangeParts {"),
        "normalized range compatibility constructor should keep the named-parts path available"
    );
}

#[test]
fn index_viewport_model_keeps_behavior_tests_focused() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let range = fs::read_to_string(manifest_dir.join("src/gui/range.rs"))
        .expect("range facade should be readable");
    let model = fs::read_to_string(manifest_dir.join("src/gui/range/index_viewport.rs"))
        .expect("index viewport model should be readable");
    let tests = fs::read_to_string(manifest_dir.join("src/gui/range/index_viewport/tests.rs"))
        .expect("index viewport behavior tests should be readable");

    assert!(
        range.contains("pub use index_viewport::{IndexViewport, IndexViewportScope};")
            && model.contains("pub struct IndexViewport")
            && model.contains("pub struct IndexViewportScope")
            && model.contains("#[path = \"index_viewport/tests.rs\"]")
            && !model.contains("fn index_viewport_clamps_visible_span_and_offset_fraction"),
        "index viewport should stay exported through the range facade while keeping behavior tests out of the model root"
    );
    assert!(
        tests.contains("fn index_viewport_clamps_visible_span_and_offset_fraction")
            && tests.contains("fn index_viewport_zooms_and_pans_around_visible_anchor")
            && tests.contains("fn index_viewport_sets_offset_and_projects_visible_ratio"),
        "index viewport behavior coverage should live in range/index_viewport/tests.rs"
    );
}

#[test]
fn normalized_viewports_use_named_parts_for_precision_bounds() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let source_path = manifest_dir.join("src/gui/range/viewport.rs");
    let module_path = manifest_dir.join("src/gui/range.rs");
    let source = fs::read_to_string(&source_path)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", source_path.display()));
    let projection = fs::read_to_string(manifest_dir.join("src/gui/range/viewport/projection.rs"))
        .expect("normalized viewport projection source should be readable");
    let viewport_tests = fs::read_to_string(manifest_dir.join("src/gui/range/viewport/tests.rs"))
        .expect("normalized viewport behavior tests should be readable");
    let module = fs::read_to_string(&module_path)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", module_path.display()));

    assert!(
        source.contains("pub struct NormalizedViewportParts")
            && source.contains("pub fn from_parts(parts: NormalizedViewportParts) -> Self"),
        "normalized viewports should expose named parts for micro and optional nano bounds"
    );
    assert!(
        source.contains("Self::from_parts(NormalizedViewportParts {")
            && source.contains("mod projection;")
            && source.contains("#[path = \"viewport/tests.rs\"]")
            && source.contains("projection::x_for_ratio")
            && !source.contains("fn finite_ordered_x_bounds")
            && !source.contains("fn normalized_viewport_projects_absolute_ratios_into_rect")
            && module.contains("NormalizedViewportParts"),
        "normalized viewport compatibility constructor and range export should keep the named-parts path available while behavior tests stay delegated"
    );
    assert!(
        viewport_tests.contains("fn normalized_viewport_projects_absolute_ratios_into_rect")
            && viewport_tests.contains("fn normalized_viewport_supports_named_parts_construction"),
        "normalized viewport behavior coverage should live in range/viewport/tests.rs"
    );
    assert!(
        projection.contains("fn local_ratio")
            && projection.contains("fn x_for_ratio")
            && projection.contains("fn finite_ordered_x_bounds"),
        "normalized viewport projection math and x-bound sanitization should live in viewport/projection.rs"
    );
}

#[test]
fn normalized_scrollbars_keep_model_and_geometry_focused() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let root = fs::read_to_string(manifest_dir.join("src/gui/range/scrollbar.rs"))
        .expect("normalized scrollbar root should be readable");
    let model = fs::read_to_string(manifest_dir.join("src/gui/range/scrollbar/model.rs"))
        .expect("normalized scrollbar model should be readable");
    let geometry = fs::read_to_string(manifest_dir.join("src/gui/range/scrollbar/geometry.rs"))
        .expect("normalized scrollbar geometry should be readable");
    let tests = fs::read_to_string(manifest_dir.join("src/gui/range/scrollbar/tests.rs"))
        .expect("normalized scrollbar behavior tests should be readable");
    let range = fs::read_to_string(manifest_dir.join("src/gui/range.rs"))
        .expect("range facade should be readable");

    assert!(
        root.contains("mod geometry;")
            && root.contains("mod model;")
            && root.contains("#[path = \"scrollbar/tests.rs\"]")
            && root.contains("pub use geometry::{")
            && root.contains("pub use model::{NormalizedScrollbar")
            && !root.contains("pub struct NormalizedScrollbarRequest")
            && !root.contains("fn clamped_normalized_span"),
        "normalized scrollbar root should re-export focused model and geometry modules while delegating behavior tests"
    );
    assert!(
        tests.contains("fn normalized_scrollbar_maps_viewport_to_horizontal_thumb")
            && tests.contains("fn normalized_scrollbar_resolves_drag_and_track_click_center"),
        "normalized scrollbar behavior coverage should live in range/scrollbar/tests.rs"
    );
    assert!(
        model.contains("pub struct NormalizedScrollbarRequest")
            && model.contains("pub struct NormalizedScrollbar"),
        "normalized scrollbar public DTOs should live in scrollbar/model.rs"
    );
    assert!(
        geometry.contains("pub fn resolve_normalized_scrollbar")
            && geometry.contains("pub fn normalized_scrollbar_center_for_pointer")
            && geometry.contains("struct NormalizedScrollbarSpan")
            && geometry.contains("fn width_ratio")
            && geometry.contains("fn max_start_micros")
            && geometry.contains("fn center_for_start")
            && !geometry.contains("fn clamped_normalized_span")
            && !geometry.contains("-> (u32, u32, u32)")
            && !geometry.contains("pub struct NormalizedScrollbar"),
        "normalized scrollbar geometry should use a named internal span model instead of positional normalized-span tuples"
    );
    assert!(
        range.contains("NormalizedScrollbarRequest")
            && range.contains("resolve_normalized_scrollbar")
            && range.contains("normalized_scrollbar_center_at_point"),
        "normalized scrollbar public API should remain available through the range facade"
    );
}
