use super::{DemoMessage, DemoState, text_value};
use radiant::{
    app,
    gui::types::{ImageRgba, Rect, Vector2},
    prelude::{IntoView, Rgba8},
    runtime::{
        PaintPrimitive, RenderCanvasCapabilities, RenderCanvasConfiguredParts, RenderCanvasContent,
        RenderCanvasInputParts, RenderCanvasLineStyle, RenderCanvasOverlay,
        RenderCanvasRuntimeOverlays, SurfaceRuntime, render_canvas,
        render_canvas_configured_from_parts, render_canvas_input, render_canvas_input_from_parts,
        render_canvas_with_capabilities,
    },
    theme::ThemeTokens,
    widgets::WidgetInput,
};
use std::sync::Arc;

#[test]
fn app_render_canvas_builder_lowers_through_normal_view_path() {
    let atlas = Arc::new(ImageRgba::new(2, 1, vec![255; 8]).expect("valid atlas"));
    let view = radiant::prelude::row([render_canvas::<DemoMessage>(
        41,
        7,
        RenderCanvasContent::RgbaAtlas {
            source_rect: Rect::from_min_size(
                radiant::layout::Point::new(0.0, 0.0),
                Vector2::new(2.0, 1.0),
            ),
            atlas: Arc::clone(&atlas),
        },
    )
    .id(90)
    .size(240.0, 120.0)
    .width(240.0)
    .height(120.0)])
    .align_cross(radiant::layout::CrossAlign::Start);
    let surface = view.into_surface();
    let layout = radiant::layout::layout_tree(
        &surface.layout_node(),
        Rect::from_min_size(
            radiant::layout::Point::new(0.0, 0.0),
            Vector2::new(320.0, 160.0),
        ),
    );

    let plan = surface.paint_plan(&layout, &ThemeTokens::default());

    let gpu = plan
        .primitives
        .iter()
        .find_map(|primitive| match primitive {
            PaintPrimitive::GpuSurface(gpu) => Some(gpu),
            _ => None,
        })
        .expect("app render canvas should emit a retained GPU paint primitive");
    assert_eq!(gpu.widget_id, 90);
    assert_eq!(gpu.key, 41);
    assert_eq!(gpu.revision, 7);
    assert_eq!(
        gpu.rect,
        Rect::from_min_size(
            radiant::layout::Point::new(0.0, 0.0),
            Vector2::new(240.0, 120.0)
        )
    );
    let RenderCanvasContent::RgbaAtlas { atlas: emitted, .. } = &gpu.content else {
        panic!("expected RGBA atlas content");
    };
    assert!(Arc::ptr_eq(&atlas, emitted));
}

#[test]
fn app_render_canvas_capabilities_helper_preserves_capabilities() {
    let atlas = Arc::new(ImageRgba::new(2, 1, vec![255; 8]).expect("valid atlas"));
    let line = RenderCanvasLineStyle {
        color: Rgba8 {
            r: 255,
            g: 255,
            b: 255,
            a: 235,
        },
        width: 1.0,
    };
    let view = render_canvas_with_capabilities::<DemoMessage>(
        41,
        7,
        RenderCanvasContent::RgbaAtlas {
            source_rect: Rect::from_min_size(
                radiant::layout::Point::new(0.0, 0.0),
                Vector2::new(2.0, 1.0),
            ),
            atlas,
        },
        RenderCanvasCapabilities {
            fast_pointer_move: true,
            coalesce_vertical_wheel: true,
            runtime_overlays: RenderCanvasRuntimeOverlays::pointer_vertical_line(line),
        },
    )
    .id(90)
    .size(240.0, 120.0);
    let surface = view.into_surface();
    let layout = radiant::layout::layout_tree(
        &surface.layout_node(),
        Rect::from_min_size(
            radiant::layout::Point::new(0.0, 0.0),
            Vector2::new(320.0, 160.0),
        ),
    );

    let plan = surface.paint_plan(&layout, &ThemeTokens::default());
    let gpu = plan
        .primitives
        .iter()
        .find_map(|primitive| match primitive {
            PaintPrimitive::GpuSurface(gpu) => Some(gpu),
            _ => None,
        })
        .expect("configured render canvas should emit a retained GPU paint primitive");

    assert_eq!(gpu.widget_id, 90);
    assert!(gpu.capabilities.fast_pointer_move);
    assert!(gpu.capabilities.coalesce_vertical_wheel);
    assert_eq!(
        gpu.capabilities.runtime_overlays.pointer_vertical_line,
        Some(line)
    );
}

#[test]
fn app_configured_render_canvas_parts_preserve_capabilities_and_overlays() {
    let atlas = Arc::new(ImageRgba::new(2, 1, vec![255; 8]).expect("valid atlas"));
    let line = RenderCanvasLineStyle {
        color: Rgba8 {
            r: 255,
            g: 255,
            b: 255,
            a: 235,
        },
        width: 1.0,
    };
    let view = render_canvas_configured_from_parts::<DemoMessage>(
        RenderCanvasConfiguredParts::new(
            41,
            7,
            RenderCanvasContent::RgbaAtlas {
                source_rect: Rect::from_min_size(
                    radiant::layout::Point::new(0.0, 0.0),
                    Vector2::new(2.0, 1.0),
                ),
                atlas,
            },
        )
        .capabilities(RenderCanvasCapabilities {
            fast_pointer_move: true,
            coalesce_vertical_wheel: true,
            runtime_overlays: RenderCanvasRuntimeOverlays::pointer_vertical_line(line),
        })
        .overlays(vec![RenderCanvasOverlay::VerticalCursor {
            ratio: 0.5,
            color: line.color,
            width: line.width,
        }]),
    )
    .id(90)
    .size(240.0, 120.0);
    let surface = view.into_surface();
    let layout = radiant::layout::layout_tree(
        &surface.layout_node(),
        Rect::from_min_size(
            radiant::layout::Point::new(0.0, 0.0),
            Vector2::new(320.0, 160.0),
        ),
    );

    let plan = surface.paint_plan(&layout, &ThemeTokens::default());
    let gpu = plan
        .primitives
        .iter()
        .find_map(|primitive| match primitive {
            PaintPrimitive::GpuSurface(gpu) => Some(gpu),
            _ => None,
        })
        .expect("configured render canvas should emit a retained GPU paint primitive");

    assert_eq!(gpu.widget_id, 90);
    assert!(gpu.capabilities.fast_pointer_move);
    assert!(gpu.capabilities.coalesce_vertical_wheel);
    assert_eq!(
        gpu.capabilities.runtime_overlays.pointer_vertical_line,
        Some(line)
    );
    assert_eq!(gpu.overlays.len(), 1);
}

#[test]
fn app_render_canvas_input_parts_route_through_normal_message_path() {
    let atlas = Arc::new(ImageRgba::new(2, 1, vec![255; 8]).expect("valid atlas"));
    let bridge = app(DemoState::default())
        .view(move |state: &DemoState| {
            radiant::prelude::column([
                radiant::prelude::text(format!("render-canvas inputs: {}", state.count)).id(91),
                render_canvas_input_from_parts(RenderCanvasInputParts {
                    key: 41,
                    revision: 7,
                    content: RenderCanvasContent::RgbaAtlas {
                        source_rect: Rect::from_min_size(
                            radiant::layout::Point::new(0.0, 0.0),
                            Vector2::new(2.0, 1.0),
                        ),
                        atlas: Arc::clone(&atlas),
                    },
                    map: DemoMessage::CanvasInput,
                })
                .id(90)
                .size(240.0, 120.0),
            ])
        })
        .handle_message(|state, message, _context| {
            if let DemoMessage::CanvasInput(WidgetInput::PointerPress { .. }) = message {
                state.count += 1;
            }
        })
        .into_bridge();
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(320.0, 160.0));

    let handled = runtime.dispatch_input(
        90,
        WidgetInput::PointerPress {
            position: radiant::layout::Point::new(24.0, 24.0),
            button: radiant::widgets::PointerButton::Primary,
            modifiers: Default::default(),
        },
    );

    assert!(handled);
    assert_eq!(text_value(runtime.surface(), 91), "render-canvas inputs: 1");
}

#[test]
fn app_render_canvas_input_helper_routes_through_normal_message_path() {
    let atlas = Arc::new(ImageRgba::new(2, 1, vec![255; 8]).expect("valid atlas"));
    let bridge = app(DemoState::default())
        .view(move |state: &DemoState| {
            radiant::prelude::column([
                radiant::prelude::text(format!("render-canvas inputs: {}", state.count)).id(91),
                render_canvas_input(
                    41,
                    7,
                    RenderCanvasContent::RgbaAtlas {
                        source_rect: Rect::from_min_size(
                            radiant::layout::Point::new(0.0, 0.0),
                            Vector2::new(2.0, 1.0),
                        ),
                        atlas: Arc::clone(&atlas),
                    },
                    DemoMessage::CanvasInput,
                )
                .id(90)
                .size(240.0, 120.0),
            ])
        })
        .handle_message(|state, message, _context| {
            if let DemoMessage::CanvasInput(WidgetInput::PointerPress { .. }) = message {
                state.count += 1;
            }
        })
        .into_bridge();
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(320.0, 160.0));

    let handled = runtime.dispatch_input(
        90,
        WidgetInput::PointerPress {
            position: radiant::layout::Point::new(24.0, 24.0),
            button: radiant::widgets::PointerButton::Primary,
            modifiers: Default::default(),
        },
    );

    assert!(handled);
    assert_eq!(text_value(runtime.surface(), 91), "render-canvas inputs: 1");
}
