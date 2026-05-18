use super::*;

#[path = "paint_projection/advanced_surfaces.rs"]
mod advanced_surfaces;
#[path = "paint_projection/gpu_surfaces.rs"]
mod gpu_surfaces;

#[derive(Default)]
struct CountingRenderer {
    rendered_primitives: usize,
}

impl Renderer for CountingRenderer {
    type Error = std::convert::Infallible;

    fn render(&mut self, plan: &radiant::runtime::SurfacePaintPlan) -> Result<(), Self::Error> {
        self.rendered_primitives += plan.primitives.len();
        Ok(())
    }
}

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

    assert_eq!(plan.primitives.len(), 300);
    assert!(
        plan.primitives.capacity() <= layout.rects.len().saturating_mul(3),
        "control-heavy paint plans should not grow beyond the initial estimate"
    );
}

#[test]
fn view_and_element_aliases_match_runtime_surface_types() {
    let surface: Arc<View<DemoMessage>> = project_surface(&mut DemoState::default());
    let root: &Element<DemoMessage> = surface.root();

    assert_eq!(root.id(), 1);
    assert!(surface.find_widget(11).is_some());
}

#[test]
fn overlay_panel_nodes_paint_without_joining_widget_hit_testing() {
    let surface: UiSurface<()> = UiSurface::new(SurfaceNode::stack(
        1,
        vec![
            SurfaceChild::fill(SurfaceNode::text(
                2,
                "Content",
                WidgetSizing::fixed(Vector2::new(120.0, 24.0)),
            )),
            SurfaceChild::fill(SurfaceNode::overlay_panel(
                3,
                Rect::from_min_size(Point::new(12.0, 18.0), Vector2::new(180.0, 44.0)),
                "Dragging",
                WidgetStyle {
                    tone: WidgetTone::Accent,
                    prominence: WidgetProminence::Subtle,
                },
            )),
        ],
    ));
    let output = layout_tree(
        &surface.layout_node(),
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(240.0, 96.0)),
    );
    let plan = surface.paint_plan(&output, &ThemeTokens::default());

    assert!(surface.find_widget(3).is_none());
    assert!(
        plan.primitives.iter().any(
            |primitive| matches!(primitive, PaintPrimitive::Text(text) if text.widget_id == 3 && text.text == "Dragging")
        )
    );
}

#[test]
fn runtime_context_and_renderer_cover_paint_plan_boundary() {
    let theme = ThemeTokens::default();
    let bridge = declarative_runtime_bridge(
        DemoState {
            count: 3,
            name: String::from("Panels"),
        },
        project_surface,
        |state: &mut DemoState, message| match message {
            DemoMessage::Increment => state.count += 1,
            DemoMessage::Rename(name) => state.name = name,
            DemoMessage::CanvasInput(_) => {}
        },
    );
    let runtime = SurfaceRuntime::new(bridge, Vector2::new(420.0, 32.0));
    let context = runtime.context();

    assert_eq!(context.viewport.width(), 420.0);
    assert!(context.surface.find_widget(11).is_some());
    assert!(context.layout.rects.contains_key(&11));

    let plan = runtime.paint_plan(&theme);
    let mut renderer = CountingRenderer::default();
    renderer
        .render(&plan)
        .expect("counting renderer cannot fail");
    assert_eq!(renderer.rendered_primitives, plan.primitives.len());
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
fn runtime_borrowed_frame_reuses_current_layout_without_cloning() {
    let theme = ThemeTokens::default();
    let bridge = declarative_runtime_bridge(
        DemoState {
            count: 5,
            name: String::from("Borrowed"),
        },
        project_surface,
        |state: &mut DemoState, message| match message {
            DemoMessage::Increment => state.count += 1,
            DemoMessage::Rename(name) => state.name = name,
            DemoMessage::CanvasInput(_) => {}
        },
    );
    let runtime = SurfaceRuntime::new(bridge, Vector2::new(420.0, 32.0));

    let frame: radiant::runtime::RuntimeSurfaceFrame<'_> = runtime.borrowed_frame(&theme);

    assert_eq!(frame.viewport, runtime.context().viewport);
    assert!(std::ptr::eq(frame.layout, runtime.layout()));
    assert_eq!(frame.paint_plan, runtime.paint_plan(&theme));
}

#[test]
fn runtime_borrowed_frame_into_reuses_layout_and_paint_plan_storage() {
    let theme = ThemeTokens::default();
    let bridge = declarative_runtime_bridge(
        DemoState {
            count: 5,
            name: String::from("Borrowed"),
        },
        project_surface,
        |state: &mut DemoState, message| match message {
            DemoMessage::Increment => state.count += 1,
            DemoMessage::Rename(name) => state.name = name,
            DemoMessage::CanvasInput(_) => {}
        },
    );
    let runtime = SurfaceRuntime::new(bridge, Vector2::new(420.0, 32.0));
    let mut paint_plan = SurfacePaintPlan::empty(&theme);
    paint_plan.primitives.reserve(128);
    let plan_ptr = std::ptr::addr_of!(paint_plan);
    let capacity = paint_plan.primitives.capacity();

    let frame: radiant::runtime::RuntimeSurfaceFrameRef<'_, '_> =
        runtime.borrowed_frame_into(&theme, &mut paint_plan);

    assert_eq!(frame.viewport, runtime.context().viewport);
    assert!(std::ptr::eq(frame.layout, runtime.layout()));
    assert!(std::ptr::eq(frame.paint_plan, plan_ptr));
    assert_eq!(frame.paint_plan, &runtime.paint_plan(&theme));
    assert_eq!(frame.paint_plan.primitives.capacity(), capacity);
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

#[test]
fn generic_public_surface_resolves_theme_without_legacy_shell_contracts() {
    let theme = ThemeTokens::default();
    let visuals = resolve_widget_visual_tokens(
        &theme,
        WidgetStyle::default(),
        WidgetState {
            focused: true,
            ..WidgetState::default()
        },
    );

    assert_eq!(visuals.border, theme.border_emphasis);
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
    assert_eq!(runtime_plan.primitives.len(), 7);

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
