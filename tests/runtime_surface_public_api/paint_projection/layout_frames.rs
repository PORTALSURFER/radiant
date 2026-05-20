use super::*;

#[test]
fn generic_runtime_surface_projects_layout_without_legacy_app_contracts() {
    let surface = project_surface(&mut DemoState::default());
    let output = layout_tree(
        &surface.layout_node(),
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(420.0, 32.0)),
    );

    assert!(output.rects.contains_key(&10));
    assert!(output.rects.contains_key(&11));
    assert!(output.rects.contains_key(&12));
}

#[test]
fn host_controlled_surface_can_resolve_layout_without_manual_layout_node_projection() {
    let surface = project_surface(&mut DemoState::default());
    let viewport = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(420.0, 32.0));

    let direct = layout_tree(&surface.layout_node(), viewport);
    let surface_layout = surface.layout(viewport);

    assert_eq!(surface_layout, direct);
    assert!(surface_layout.rects.contains_key(&11));
}

#[test]
fn host_controlled_surface_layout_options_match_manual_stateful_layout() {
    let surface: UiSurface<()> = radiant::prelude::scroll(
        radiant::prelude::column((0..10).map(|index| {
            radiant::prelude::text(format!("Debug row {index}"))
                .height(28.0)
                .fill_width()
        }))
        .id(20)
        .fill_width(),
    )
    .id(10)
    .fill()
    .into_surface();
    let viewport = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(240.0, 80.0));
    let mut layout_state = radiant::layout::LayoutState::default();
    layout_state
        .scroll_offsets
        .insert(10, Vector2::new(0.0, 1_000.0));

    let direct = layout_tree_with_state(
        &surface.layout_node(),
        viewport,
        &layout_state,
        radiant::layout::LayoutDebugOptions::all_enabled(),
    );
    let surface_layout = surface.layout_with_options(
        viewport,
        &layout_state,
        radiant::layout::LayoutDebugOptions::all_enabled(),
    );

    assert_eq!(surface_layout, direct);
    assert!(surface_layout.diagnostics.iter().any(|diagnostic| {
        diagnostic.code == radiant::layout::LayoutDiagnosticCode::InvalidScrollOffsetClamped
    }));
    assert!(!surface_layout.debug_primitives.is_empty());
}

#[test]
fn host_controlled_surface_frame_packages_layout_and_paint_plan() {
    let surface = project_surface(&mut DemoState {
        count: 4,
        name: String::from("Embedded"),
    });
    let theme = ThemeTokens::default();
    let viewport = Rect::from_min_size(Point::new(4.0, 6.0), Vector2::new(420.0, 32.0));

    let frame: radiant::runtime::SurfaceFrame = surface.frame(viewport, &theme);

    assert_eq!(frame.viewport, viewport);
    assert!(frame.layout.rects.contains_key(&11));
    assert_eq!(frame.paint_plan.clear_color, theme.clear_color);
    assert_eq!(
        frame.paint_plan,
        surface.paint_plan(&frame.layout, &theme),
        "host frame should use the same deterministic paint-plan path as manual layout"
    );
}

#[test]
fn host_controlled_surface_frame_can_collect_layout_debug_output() {
    let surface: UiSurface<()> = radiant::prelude::scroll(
        radiant::prelude::column((0..10).map(|index| {
            radiant::prelude::text(format!("Debug row {index}"))
                .height(28.0)
                .fill_width()
        }))
        .id(20)
        .fill_width(),
    )
    .id(10)
    .fill()
    .into_surface();
    let mut layout_state = radiant::layout::LayoutState::default();
    layout_state
        .scroll_offsets
        .insert(10, Vector2::new(0.0, 1_000.0));

    let frame = surface.frame_with_layout_options(
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(240.0, 80.0)),
        &ThemeTokens::default(),
        &layout_state,
        radiant::layout::LayoutDebugOptions::all_enabled(),
    );

    assert!(frame.layout.diagnostics.iter().any(|diagnostic| {
        diagnostic.code == radiant::layout::LayoutDiagnosticCode::InvalidScrollOffsetClamped
    }));
    assert!(!frame.layout.debug_primitives.is_empty());
    assert!(!frame.paint_plan.primitives.is_empty());
}
