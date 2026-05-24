use super::*;

#[test]
fn surface_paint_plan_into_reuses_existing_primitive_storage() {
    let surface = project_surface(&mut DemoState::default());
    let output = layout_tree(
        &surface.layout_node(),
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(420.0, 32.0)),
    );
    let theme = ThemeTokens::default();
    let expected = surface.paint_plan(&output, &theme);
    let mut plan = SurfacePaintPlan::empty(&theme);
    plan.primitives.reserve(128);
    let capacity = plan.primitives.capacity();

    surface.paint_plan_into(&output, &theme, &mut plan);

    assert_eq!(plan, expected);
    assert_eq!(plan.primitives.capacity(), capacity);
}

#[test]
fn control_heavy_paint_plan_presizes_for_button_chrome() {
    let surface: UiSurface<DemoMessage> = UiSurface::new(SurfaceNode::row(
        1,
        2.0,
        (0..100)
            .map(|index| {
                SurfaceChild::fill(SurfaceNode::button(
                    10 + index,
                    "Run",
                    WidgetSizing::fixed(Vector2::new(48.0, 24.0)),
                    DemoMessage::Increment,
                ))
            })
            .collect(),
    ));
    let layout = layout_tree(
        &surface.layout_node(),
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(5_000.0, 32.0)),
    );

    let plan = surface.paint_plan(&layout, &ThemeTokens::default());

    assert_eq!(plan.primitives.len(), 500);
    assert!(
        plan.primitives.capacity() <= layout.rects.len().saturating_mul(5),
        "control-heavy paint plans should not grow beyond the initial estimate"
    );
}

#[test]
fn runtime_layout_debug_options_append_red_node_border_strokes() {
    let theme = ThemeTokens::default();
    let bridge = declarative_runtime_bridge(
        DemoState {
            count: 5,
            name: String::from("Debug"),
        },
        project_surface,
        |state: &mut DemoState, message| match message {
            DemoMessage::Increment => state.count += 1,
            DemoMessage::Rename(name) => state.name = name,
            DemoMessage::CanvasInput(_) => {}
        },
    );
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(420.0, 32.0));

    assert!(
        runtime.layout().debug_primitives.is_empty(),
        "layout debug primitives should stay off by default"
    );
    runtime.set_layout_debug_options(radiant::layout::LayoutDebugOptions::bounds_only());

    assert!(!runtime.layout().debug_primitives.is_empty());
    let plan = runtime.paint_plan(&theme);
    let red_debug_strokes = plan
        .primitives
        .iter()
        .filter(|primitive| {
            matches!(
                primitive,
                PaintPrimitive::StrokeRect(stroke)
                    if stroke.color == (Rgba8 { r: 255, g: 0, b: 0, a: 255 })
                        && (stroke.width - 1.0).abs() < f32::EPSILON
            )
        })
        .count();

    assert_eq!(red_debug_strokes, runtime.layout().debug_primitives.len());
}

#[test]
fn generic_surface_projects_deterministic_paint_without_legacy_shell_contracts() {
    let theme = ThemeTokens::default();
    let bridge = declarative_runtime_bridge(
        DemoState {
            count: 2,
            name: String::from("Crates"),
        },
        project_surface,
        |state: &mut DemoState, message| match message {
            DemoMessage::Increment => state.count += 1,
            DemoMessage::Rename(name) => state.name = name,
            DemoMessage::CanvasInput(_) => {}
        },
    );
    let runtime = SurfaceRuntime::new(bridge, Vector2::new(420.0, 32.0));

    let direct_plan = runtime.surface().paint_plan(runtime.layout(), &theme);
    let runtime_plan = runtime.paint_plan(&theme);

    assert_eq!(runtime_plan, direct_plan);
    assert_eq!(runtime_plan.clear_color, theme.clear_color);
    assert_eq!(runtime_plan.primitives.len(), 13);

    let texts: Vec<_> = runtime_plan
        .primitives
        .iter()
        .filter_map(|primitive| match primitive {
            PaintPrimitive::Text(text) => Some((text.widget_id, text.text.as_str())),
            _ => None,
        })
        .collect();
    assert_eq!(texts, vec![(10, "Crates (2)"), (11, "Increment")]);
    let text_inputs: Vec<_> = runtime_plan
        .primitives
        .iter()
        .filter_map(|primitive| match primitive {
            PaintPrimitive::TextInput(input) => Some((input.widget_id, input.state.value.as_str())),
            _ => None,
        })
        .collect();
    assert_eq!(text_inputs, vec![(12, "Crates")]);

    let fills: Vec<_> = runtime_plan
        .primitives
        .iter()
        .filter_map(|primitive| match primitive {
            PaintPrimitive::FillRect(fill) => Some((fill.widget_id, fill.color)),
            _ => None,
        })
        .collect();
    assert_eq!(fills, vec![(12, theme.bg_primary)]);

    let button_polygons: Vec<_> = runtime_plan
        .primitives
        .iter()
        .filter_map(|primitive| match primitive {
            PaintPrimitive::FillPolygon(fill) => Some((fill.widget_id, fill.points.len())),
            _ => None,
        })
        .collect();
    assert_eq!(button_polygons, vec![(11, 5)]);
}
