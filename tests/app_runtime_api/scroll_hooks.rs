use super::{DemoMessage, DemoState, intrinsic_slot, text_value};
use radiant::{
    app,
    gui::types::{Point, Vector2},
    runtime::{Event, PaintPrimitive, SurfaceChild, SurfaceNode, SurfaceRuntime, UiSurface},
    theme::ThemeTokens,
    widgets::{PointerButton, TextWidget, WidgetSizing},
};
use std::sync::{Arc, Mutex};

#[test]
fn app_scroll_hook_observes_runtime_scroll_offsets() {
    let observed_scroll_y = Arc::new(Mutex::new(None));
    let observed_scroll_y_for_hook = Arc::clone(&observed_scroll_y);
    let bridge = app(DemoState::default())
        .view(|state: &mut DemoState| {
            UiSurface::new(SurfaceNode::scroll_area(
                20,
                SurfaceNode::column(
                    21,
                    0.0,
                    (0..12)
                        .map(|index| {
                            SurfaceChild::new(
                                intrinsic_slot(),
                                SurfaceNode::static_widget(TextWidget::new(
                                    100 + index,
                                    format!("Row {index} at {:.0}", state.last_scroll_y),
                                    WidgetSizing::fixed(Vector2::new(160.0, 28.0))
                                        .with_baseline(18.0),
                                )),
                            )
                        })
                        .collect(),
                ),
            ))
        })
        .on_scroll(move |state, update, _context| {
            state.last_scroll_y = update.offset.y;
            *observed_scroll_y_for_hook
                .lock()
                .expect("scroll observer lock should be available") = Some(update.offset.y);
        })
        .update_with(|_state, _message: DemoMessage, _context| {})
        .into_bridge();
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(220.0, 96.0));

    assert!(runtime.scroll_at(Point::new(20.0, 56.0), Vector2::new(0.0, 48.0)));

    assert_eq!(
        *observed_scroll_y
            .lock()
            .expect("scroll observer lock should be available"),
        Some(48.0)
    );
    assert_eq!(text_value(runtime.surface(), 100), "Row 0 at 48");
    assert!(runtime.take_repaint_requested());
}

#[test]
fn app_scroll_hook_observes_scrollbar_drag_offsets() {
    let observed_scroll_y = Arc::new(Mutex::new(None));
    let observed_scroll_y_for_hook = Arc::clone(&observed_scroll_y);
    let bridge = app(DemoState::default())
        .view(|state: &mut DemoState| {
            UiSurface::new(SurfaceNode::scroll_area(
                20,
                SurfaceNode::column(
                    21,
                    0.0,
                    (0..16)
                        .map(|index| {
                            SurfaceChild::new(
                                intrinsic_slot(),
                                SurfaceNode::static_widget(TextWidget::new(
                                    100 + index,
                                    format!("Row {index} at {:.0}", state.last_scroll_y),
                                    WidgetSizing::fixed(Vector2::new(160.0, 28.0))
                                        .with_baseline(18.0),
                                )),
                            )
                        })
                        .collect(),
                ),
            ))
        })
        .on_scroll(move |state, update, _context| {
            state.last_scroll_y = update.offset.y;
            *observed_scroll_y_for_hook
                .lock()
                .expect("scroll observer lock should be available") = Some(update.offset.y);
        })
        .update_with(|_state, _message: DemoMessage, _context| {})
        .into_bridge();
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(220.0, 96.0));
    let thumb = runtime
        .paint_plan(&ThemeTokens::default())
        .primitives
        .iter()
        .find_map(|primitive| match primitive {
            PaintPrimitive::FillRect(fill) if fill.widget_id == 20 => Some(fill.rect),
            _ => None,
        })
        .expect("scroll area should paint a draggable thumb");

    runtime.dispatch_event(Event::PointerPress {
        position: thumb.center(),
        button: PointerButton::Primary,
        modifiers: Default::default(),
    });
    runtime.dispatch_event(Event::PointerMove {
        position: Point::new(thumb.center().x, thumb.center().y + 36.0),
    });

    let observed = observed_scroll_y
        .lock()
        .expect("scroll observer lock should be available")
        .expect("scroll drag should notify host scroll hook");
    assert!(observed > 0.0);
    assert_eq!(
        text_value(runtime.surface(), 100),
        format!("Row 0 at {:.0}", observed)
    );
    assert!(runtime.take_repaint_requested());
}
