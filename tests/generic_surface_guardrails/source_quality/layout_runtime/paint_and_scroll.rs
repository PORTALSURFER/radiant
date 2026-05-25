use super::*;

#[test]
fn surface_paint_plan_buffering_stays_with_capacity_policy() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let projection = fs::read_to_string(manifest_dir.join("src/runtime/surface/projection.rs"))
        .expect("surface paint projection should be readable");
    let frame = fs::read_to_string(manifest_dir.join("src/runtime/controller/context/frame.rs"))
        .expect("runtime frame paint projection should be readable");
    let capacity = fs::read_to_string(manifest_dir.join("src/runtime/surface/paint/capacity.rs"))
        .expect("surface paint capacity policy should be readable");
    let capacity_tests =
        fs::read_to_string(manifest_dir.join("src/runtime/surface/paint/capacity/tests.rs"))
            .expect("surface paint capacity tests should be readable");

    assert!(
        capacity.contains("fn empty_paint_plan_for_layout")
            && capacity.contains("fn clear_paint_plan_for_layout")
            && capacity.contains("fn estimated_paint_primitive_capacity")
            && capacity.contains("#[path = \"capacity/tests.rs\"]")
            && !capacity.contains("fn estimated_paint_primitive_capacity_scales_for_small_layouts"),
        "layout-aware paint-plan buffer lifecycle should live with the capacity policy while behavior tests stay delegated"
    );
    assert!(
        capacity_tests.contains("fn estimated_paint_primitive_capacity_scales_for_small_layouts")
            && capacity_tests.contains("fn clear_paint_plan_for_layout_reuses_existing_capacity"),
        "surface paint capacity behavior coverage should live in surface/paint/capacity/tests.rs"
    );
    assert!(
        projection.contains("empty_paint_plan_for_layout(layout, theme)")
            && projection.contains("clear_paint_plan_for_layout(plan, layout, theme)")
            && frame.contains("empty_paint_plan_for_layout(&self.layout, theme)"),
        "surface and runtime paint projection should consume layout-aware plan helpers"
    );
    assert!(
        !projection.contains("estimated_paint_primitive_capacity")
            && !frame.contains("estimated_paint_primitive_capacity")
            && !projection.contains("SurfacePaintPlan::empty_with_capacity")
            && !frame.contains("SurfacePaintPlan::empty_with_capacity"),
        "paint projection callers should not duplicate capacity-policy mechanics"
    );
}

#[test]
fn runtime_paint_primitive_support_keeps_models_queries_and_tests_focused() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let facade = fs::read_to_string(manifest_dir.join("src/runtime/paint.rs"))
        .expect("runtime paint facade should be readable");
    let stats = fs::read_to_string(manifest_dir.join("src/runtime/paint/primitives/stats.rs"))
        .expect("paint primitive stats module should be readable");
    let stats_tests =
        fs::read_to_string(manifest_dir.join("src/runtime/paint/primitives/stats/tests.rs"))
            .expect("paint primitive stats tests should be readable");
    let query = fs::read_to_string(manifest_dir.join("src/runtime/paint/primitives/query.rs"))
        .expect("paint primitive query module should be readable");
    let query_tests =
        fs::read_to_string(manifest_dir.join("src/runtime/paint/primitives/query/tests.rs"))
            .expect("paint primitive query tests should be readable");
    let path = fs::read_to_string(manifest_dir.join("src/runtime/paint/primitives/path.rs"))
        .expect("paint primitive path module should be readable");
    let path_tests =
        fs::read_to_string(manifest_dir.join("src/runtime/paint/primitives/path/tests.rs"))
            .expect("paint primitive path tests should be readable");
    let plan = fs::read_to_string(manifest_dir.join("src/runtime/paint/primitives/plan.rs"))
        .expect("paint primitive plan module should be readable");
    let plan_tests =
        fs::read_to_string(manifest_dir.join("src/runtime/paint/primitives/plan/tests.rs"))
            .expect("paint primitive plan tests should be readable");
    let text = fs::read_to_string(manifest_dir.join("src/runtime/paint/primitives/text.rs"))
        .expect("paint primitive text module should be readable");
    let text_tests =
        fs::read_to_string(manifest_dir.join("src/runtime/paint/primitives/text/tests.rs"))
            .expect("paint primitive text tests should be readable");

    assert!(
        facade.contains("pub use primitives::{")
            && facade.contains("PaintPrimitive")
            && facade.contains("SurfacePaintPlan")
            && facade.contains("TransientOverlayContext")
            && facade.contains("SvgParseError")
            && !facade.contains("pub use primitives::*;"),
        "runtime paint facade should explicitly name backend-neutral paint API exports"
    );
    assert!(
        stats.contains("pub struct SurfacePaintStats")
            && stats.contains("pub fn stats(&self) -> SurfacePaintStats")
            && stats.contains("#[path = \"stats/tests.rs\"]")
            && !stats.contains("fn surface_paint_plan_stats_count_core_primitive_groups"),
        "paint primitive stats should live in primitives/stats.rs while behavior tests stay delegated"
    );
    assert!(
        stats_tests.contains("fn surface_paint_plan_stats_count_core_primitive_groups"),
        "paint primitive stats behavior coverage should live in primitives/stats/tests.rs"
    );
    assert!(
        query.contains("pub fn first_widget_rect(&self, widget_id: WidgetId) -> Option<Rect>")
            && query.contains("pub fn widget_id(&self) -> Option<WidgetId>")
            && query.contains("pub fn rect(&self) -> Option<Rect>")
            && query.contains("#[path = \"query/tests.rs\"]")
            && !query
                .contains("fn first_widget_rect_returns_first_rectangle_anchor_in_paint_order"),
        "paint primitive query helpers should live in primitives/query.rs while behavior tests stay delegated"
    );
    assert!(
        query_tests.contains("fn first_widget_rect_returns_first_rectangle_anchor_in_paint_order")
            && query_tests
                .contains("fn paint_primitive_reports_widget_id_and_rect_for_anchor_primitives"),
        "paint primitive query behavior coverage should live in primitives/query/tests.rs"
    );
    assert!(
        path.contains("pub struct PaintPath")
            && path.contains("pub struct PaintTransform")
            && path.contains("pub enum PaintFillRule")
            && path.contains("#[path = \"path/tests.rs\"]")
            && !path.contains("fn paint_path_preserves_backend_neutral_commands"),
        "paint path models should live in primitives/path.rs while behavior tests stay delegated"
    );
    assert!(
        path_tests.contains("fn paint_path_preserves_backend_neutral_commands")
            && path_tests.contains("fn paint_transform_reports_finite_coefficients"),
        "paint path behavior coverage should live in primitives/path/tests.rs"
    );
    assert!(
        plan.contains("pub enum PaintPrimitive")
            && plan.contains("pub struct SurfacePaintPlan")
            && plan.contains("pub struct TransientOverlayContext")
            && plan.contains("#[path = \"plan/tests.rs\"]")
            && !plan.contains("fn empty_with_capacity_presizes_primitive_storage"),
        "paint plan models should live in primitives/plan.rs while behavior tests stay delegated"
    );
    assert!(
        plan_tests.contains("fn empty_with_capacity_presizes_primitive_storage")
            && plan_tests.contains("fn clear_for_theme_with_capacity_grows_to_requested_capacity"),
        "paint plan behavior coverage should live in primitives/plan/tests.rs"
    );
    assert!(
        text.contains("pub struct PaintText")
            && text.contains("pub struct PaintTextRun")
            && text.contains("pub struct PaintTextInput")
            && text.contains("#[path = \"text/tests.rs\"]")
            && !text.contains("fn paint_text_converts_compares_and_shares_storage"),
        "paint text models should live in primitives/text.rs while behavior tests stay delegated"
    );
    assert!(
        text_tests.contains("fn paint_text_converts_compares_and_shares_storage"),
        "paint text behavior coverage should live in primitives/text/tests.rs"
    );
}

#[test]
fn gui_text_field_paint_input_stays_grouped_by_paint_concern() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let facade = fs::read_to_string(manifest_dir.join("src/gui/paint.rs"))
        .expect("gui paint facade should be readable");
    let text_field = fs::read_to_string(manifest_dir.join("src/gui/paint/text_field.rs"))
        .expect("gui text-field paint helper should be readable");
    let tests = fs::read_to_string(manifest_dir.join("src/gui/paint/tests.rs"))
        .expect("gui paint behavior tests should be readable");

    assert!(
        text_field.contains("pub struct TextFieldPaintGeometry")
            && text_field.contains("pub struct TextFieldPaintContent")
            && text_field.contains("pub struct TextFieldPaintColors")
            && text_field.contains("pub struct TextFieldPaintStroke")
            && text_field.contains("pub struct TextFieldPaint")
            && facade.contains("TextFieldPaintGeometry")
            && facade.contains("TextFieldPaintColors"),
        "text-field paint inputs should stay grouped by geometry, content, colors, and stroke metrics"
    );
    assert!(
        tests.contains("fn text_field_paint_emits_chrome_selection_text_and_caret"),
        "text-field paint behavior coverage should stay in gui/paint/tests.rs"
    );
}

#[test]
fn runtime_scroll_support_keeps_affordance_and_hit_tests_focused() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let paint_scroll = fs::read_to_string(manifest_dir.join("src/runtime/paint/scroll.rs"))
        .expect("runtime paint scroll helpers should be readable");
    let paint_scroll_tests =
        fs::read_to_string(manifest_dir.join("src/runtime/paint/scroll/tests.rs"))
            .expect("runtime paint scroll tests should be readable");
    let controller_scroll =
        fs::read_to_string(manifest_dir.join("src/runtime/controller/scroll.rs"))
            .expect("runtime controller scroll root should be readable");
    let controller_scrollbar =
        fs::read_to_string(manifest_dir.join("src/runtime/controller/scroll/scrollbar.rs"))
            .expect("runtime controller scrollbar helpers should be readable");
    let controller_scrollbar_tests =
        fs::read_to_string(manifest_dir.join("src/runtime/controller/scroll/scrollbar/tests.rs"))
            .expect("runtime controller scrollbar tests should be readable");

    assert!(
        paint_scroll.contains("struct ScrollAffordance")
            && paint_scroll.contains("fn push_scroll_affordance")
            && paint_scroll.contains("fn resolve_scroll_affordance")
            && paint_scroll.contains("#[path = \"scroll/tests.rs\"]")
            && !paint_scroll.contains("fn scroll_affordance_clamps_thumb_to_cramped_track"),
        "runtime scroll paint affordance helpers should live in paint/scroll.rs while behavior tests stay delegated"
    );
    assert!(
        paint_scroll_tests.contains("fn scroll_affordance_clamps_thumb_to_cramped_track")
            && paint_scroll_tests.contains("fn scroll_affordance_rejects_nonfinite_layout_rects"),
        "runtime scroll paint behavior coverage should live in paint/scroll/tests.rs"
    );
    assert!(
        controller_scroll.contains("use super::SurfaceRuntime;")
            && controller_scroll.contains("gui::types::{Point, Vector2}")
            && controller_scroll.contains("layout::{NodeId, OverflowPolicy}")
            && controller_scroll.contains("runtime::RuntimeBridge")
            && !controller_scroll.starts_with("use super::*;")
            && controller_scroll.contains("pub struct ScrollUpdate")
            && controller_scroll.contains("fn scroll_container_at"),
        "runtime controller scroll root should name controller, geometry, layout, bridge, and scroll-update dependencies"
    );
    assert!(
        controller_scrollbar.contains("super::{ScrollDragCapture, SurfaceRuntime}")
            && controller_scrollbar.contains("ScrollUpdate")
            && controller_scrollbar.contains("gui::types::{Point, Rect, Vector2}")
            && controller_scrollbar.contains("layout::NodeId")
            && controller_scrollbar
                .contains("runtime::{RuntimeBridge, paint::resolve_scroll_affordance}")
            && !controller_scrollbar.starts_with("use super::{super::*")
            && controller_scrollbar.contains("fn scrollbar_hit_column_contains_point")
            && controller_scrollbar.contains("fn scrollbar_thumb_hit_rect")
            && controller_scrollbar.contains("#[path = \"scrollbar/tests.rs\"]")
            && !controller_scrollbar
                .contains("fn scrollbar_hit_column_rejects_points_far_from_right_edge"),
        "runtime scrollbar hit and drag helpers should name controller, state, scroll update, geometry, layout, bridge, and paint affordance dependencies while behavior tests stay delegated"
    );
    assert!(
        controller_scrollbar_tests
            .contains("fn scrollbar_hit_column_rejects_points_far_from_right_edge"),
        "runtime scrollbar behavior coverage should live in controller/scroll/scrollbar/tests.rs"
    );
}
