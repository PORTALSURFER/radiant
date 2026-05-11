use super::*;

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
fn retained_canvas_metadata_reaches_backend_neutral_paint_plan() {
    let retained = RetainedSurfaceDescriptor {
        key: 42,
        revision: 7,
        dirty_mask: 0b101,
        volatile: false,
    };
    let surface: UiSurface<DemoMessage> = UiSurface::new(SurfaceNode::retained_canvas_mapped(
        90,
        WidgetSizing::fixed(Vector2::new(240.0, 120.0)),
        retained,
        |message| match message {
            CanvasMessage::Input { input } => DemoMessage::CanvasInput(input),
        },
    ));
    let layout = layout_tree(
        &surface.layout_node(),
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(240.0, 120.0)),
    );

    let plan = surface.paint_plan(&layout, &ThemeTokens::default());

    let Some(PaintPrimitive::CustomSurface(custom)) = plan.primitives.first() else {
        panic!("retained canvas should emit one custom surface primitive");
    };
    assert_eq!(custom.widget_id, 90);
    assert_eq!(custom.retained, Some(retained));
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
fn view_and_element_aliases_match_runtime_surface_types() {
    let surface: Arc<View<DemoMessage>> = project_surface(&mut DemoState::default());
    let root: &Element<DemoMessage> = surface.root();

    assert_eq!(root.id(), 1);
    assert!(surface.find_widget(11).is_some());
}

#[test]
fn gpu_surface_widget_projects_generic_retained_gpu_primitive() {
    let atlas = Arc::new(ImageRgba::new(2, 1, vec![0, 0, 0, 255, 255, 255, 255, 255]).unwrap());
    let content = GpuSurfaceContent::RgbaAtlas {
        source_rect: Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(2.0, 1.0)),
        atlas: Arc::clone(&atlas),
    };
    let surface: UiSurface<()> = UiSurface::new(SurfaceNode::static_widget(
        GpuSurfaceWidget::new(
            41,
            WidgetSizing::fixed(Vector2::new(80.0, 20.0)),
            9001,
            7,
            content,
        )
        .with_capabilities(GpuSurfaceCapabilities {
            fast_pointer_move: true,
            coalesce_vertical_wheel: true,
            native_hover_cursor: Some(GpuHoverCursor {
                color: Rgba8 {
                    r: 255,
                    g: 255,
                    b: 255,
                    a: 255,
                },
                width: 1.0,
            }),
        })
        .with_overlays(vec![GpuSurfaceOverlay::VerticalCursor {
            ratio: 0.5,
            color: Rgba8 {
                r: 255,
                g: 255,
                b: 255,
                a: 255,
            },
            width: 1.0,
        }]),
    ));

    let output = layout_tree(
        &surface.layout_node(),
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(100.0, 40.0)),
    );
    let plan = surface.paint_plan(&output, &ThemeTokens::default());

    let Some(PaintPrimitive::GpuSurface(gpu)) = plan.primitives.first() else {
        panic!("expected gpu surface primitive");
    };
    assert_eq!(gpu.widget_id, 41);
    assert_eq!(gpu.key, 9001);
    assert_eq!(gpu.revision, 7);
    assert!(gpu.capabilities.fast_pointer_move);
    assert!(gpu.capabilities.coalesce_vertical_wheel);
    assert!(gpu.capabilities.native_hover_cursor.is_some());
    assert_eq!(gpu.overlays.len(), 1);
    let GpuSurfaceContent::RgbaAtlas { atlas, .. } = &gpu.content else {
        panic!("expected rgba atlas gpu content");
    };
    assert_eq!(atlas.width, 2);
}

#[test]
fn paint_plan_stats_count_backend_neutral_frame_shape() {
    let surface: UiSurface<DemoMessage> = UiSurface::new(SurfaceNode::stack(
        1,
        vec![
            SurfaceChild::fill(SurfaceNode::retained_canvas_mapped(
                40,
                WidgetSizing::fixed(Vector2::new(100.0, 40.0)),
                RetainedSurfaceDescriptor {
                    key: 40,
                    revision: 1,
                    dirty_mask: 0,
                    volatile: false,
                },
                |message| match message {
                    CanvasMessage::Input { input } => DemoMessage::CanvasInput(input),
                },
            )),
            SurfaceChild::fill(SurfaceNode::static_widget(GpuSurfaceWidget::new(
                41,
                WidgetSizing::fixed(Vector2::new(80.0, 20.0)),
                9001,
                7,
                GpuSurfaceContent::RgbaAtlas {
                    source_rect: Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(1.0, 1.0)),
                    atlas: Arc::new(ImageRgba::new(1, 1, vec![255, 255, 255, 255]).unwrap()),
                },
            ))),
            SurfaceChild::fill(SurfaceNode::text(
                42,
                "Stats",
                WidgetSizing::fixed(Vector2::new(80.0, 20.0)),
            )),
        ],
    ));
    let frame = surface.frame(
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(140.0, 80.0)),
        &ThemeTokens::default(),
    );

    let stats = frame.paint_plan.stats();

    assert_eq!(stats.total, frame.paint_plan.primitives.len());
    assert_eq!(stats.custom_surfaces, 1);
    assert_eq!(stats.gpu_surfaces, 1);
    assert_eq!(stats.text, 1);
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
fn retained_canvas_builder_projects_metadata_and_input_mapping() {
    let surface = radiant::prelude::retained_canvas(44)
        .revision(7)
        .dirty_mask(3)
        .volatile(true)
        .on_input(|message| match message {
            CanvasMessage::Input { input } => DemoMessage::CanvasInput(input),
        })
        .id(44)
        .size(120.0, 40.0)
        .into_surface();
    let plan = surface.paint_plan(
        &layout_tree(
            &surface.layout_node(),
            Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(120.0, 40.0)),
        ),
        &ThemeTokens::default(),
    );
    let Some(PaintPrimitive::CustomSurface(custom)) = plan.primitives.first() else {
        panic!("retained canvas should project one custom surface primitive");
    };
    assert_eq!(
        custom.retained,
        Some(RetainedSurfaceDescriptor {
            key: 44,
            revision: 7,
            dirty_mask: 3,
            volatile: true,
        })
    );
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
