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
