use super::*;

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
