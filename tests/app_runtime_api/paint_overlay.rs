use super::*;

#[test]
fn paint_only_command_skips_surface_reprojection() {
    let bridge = PaintOnlyBridge::default();
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(180.0, 40.0));

    assert_eq!(runtime.bridge().project_count, 1);
    assert_eq!(text_value(runtime.surface(), 10), "PaintOnly (0)");

    let outcome = runtime.dispatch_message(DemoMessage::Increment);

    assert!(outcome.repaint_requested);
    assert!(!outcome.surface_refresh_requested);
    assert_eq!(runtime.bridge().count, 1);
    assert_eq!(runtime.bridge().project_count, 1);
    assert_eq!(text_value(runtime.surface(), 10), "PaintOnly (0)");
}

#[test]
fn app_transient_overlay_painter_reads_state_and_cached_plan() {
    let bridge = app(DemoState::default())
        .view(|state: &DemoState| radiant::prelude::text(format!("Count {}", state.count)).id(10))
        .transient_overlay(|state, context, primitives| {
            assert_eq!(context.viewport, Vector2::new(180.0, 40.0));
            let Some(text) = context
                .plan
                .primitives
                .iter()
                .find_map(|primitive| match primitive {
                    PaintPrimitive::Text(text) => Some(text),
                    _ => None,
                })
            else {
                return;
            };
            primitives.push(PaintPrimitive::FillRect(PaintFillRect {
                widget_id: text.widget_id,
                rect: Rect::from_min_size(Point::new(4.0, 4.0), Vector2::new(8.0, 8.0)),
                color: Rgba8 {
                    r: state.count as u8,
                    g: 128,
                    b: 255,
                    a: 255,
                },
            }));
        })
        .handle_message(|state, message, context| {
            if matches!(message, DemoMessage::Increment) {
                state.count += 1;
                context.request_paint_only();
            }
        })
        .into_bridge();
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(180.0, 40.0));
    let plan = runtime.paint_plan(&ThemeTokens::default());
    let _ = runtime.dispatch_message(DemoMessage::Increment);
    let mut overlay = Vec::new();

    runtime.bridge_mut().paint_transient_overlay(
        radiant::runtime::TransientOverlayContext::new(
            &plan,
            Vector2::new(180.0, 40.0),
            std::time::Duration::ZERO,
        ),
        &mut overlay,
    );

    let [PaintPrimitive::FillRect(fill)] = overlay.as_slice() else {
        panic!("expected one transient fill rect");
    };
    assert_eq!(fill.color.r, 1);
}
