use super::{DemoMessage, DemoState, intrinsic_slot, text_value};
use radiant::{
    app,
    gui::types::{Point, Vector2},
    prelude::{self as ui, ViewProjection},
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
        .view(|state: &DemoState| {
            ViewProjection::from_surface(UiSurface::new(SurfaceNode::scroll_area(
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
            )))
        })
        .on_scroll(move |state, update, _context| {
            state.last_scroll_y = update.offset.y;
            *observed_scroll_y_for_hook
                .lock()
                .expect("scroll observer lock should be available") = Some(update.offset.y);
        })
        .handle_message(|_state, _message: DemoMessage, _context| {})
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
fn declarative_virtual_list_window_change_routes_through_update() {
    let bridge = app(DemoState::default())
        .view(|state: &DemoState| {
            let viewport_start = (state.last_scroll_y / 20.0).floor() as usize;
            let window = ui::resolve_virtual_list_window(ui::VirtualListWindowRequest {
                total_items: 20,
                viewport_len: 4,
                requested_start: viewport_start,
                overscan: 1,
                focused_index: None,
                previous_start: None,
                guard_band: 0,
            });
            ui::virtual_list_windowed(|index| {
                ui::text_line(format!("Row {index}"), 20.0).id(100 + index as u64)
            })
            .row_height(20.0)
            .window(window)
            .overscan_px(20.0)
            .on_window_changed(DemoMessage::VirtualListWindowChanged)
            .view()
            .id(20)
            .fill()
        })
        .on_scroll(|state, _update, _context| {
            state.last_scroll_y = -1.0;
        })
        .handle_message(|state, message: DemoMessage, _context| {
            if let DemoMessage::VirtualListWindowChanged(change) = message {
                state.last_scroll_y = change.offset_y;
            }
        })
        .into_bridge();
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(220.0, 80.0));

    assert!(runtime.scroll_at(Point::new(20.0, 56.0), Vector2::new(0.0, 60.0)));

    assert_eq!(text_value(runtime.surface(), 103), "Row 3");
    assert!(runtime.take_repaint_requested());
}

#[test]
fn declarative_virtual_list_bottom_scroll_keeps_rows_materialized() {
    let bridge = app(DemoState::default())
        .view(|state: &DemoState| {
            let requested_start = (state.last_scroll_y / 20.0).floor() as usize;
            let window = ui::resolve_virtual_list_window(ui::VirtualListWindowRequest {
                total_items: 100,
                viewport_len: 4,
                requested_start,
                overscan: 1,
                focused_index: None,
                previous_start: None,
                guard_band: 0,
            });
            ui::virtual_list_windowed(|index| {
                ui::text_line(format!("Row {index}"), 20.0).id(100 + index as u64)
            })
            .row_height(20.0)
            .window(window)
            .overscan_px(20.0)
            .on_window_changed(DemoMessage::VirtualListWindowChanged)
            .view()
            .id(20)
            .fill()
        })
        .handle_message(|state, message: DemoMessage, _context| {
            if let DemoMessage::VirtualListWindowChanged(change) = message {
                state.last_scroll_y = change.offset_y;
            }
        })
        .into_bridge();
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(220.0, 80.0));

    assert!(runtime.scroll_at(Point::new(20.0, 56.0), Vector2::new(0.0, 100.0 * 20.0)));

    assert_eq!(text_value(runtime.surface(), 196), "Row 96");
    assert_eq!(text_value(runtime.surface(), 199), "Row 99");
}

#[test]
fn app_scroll_hook_observes_scrollbar_drag_offsets() {
    let observed_scroll_y = Arc::new(Mutex::new(None));
    let observed_scroll_y_for_hook = Arc::clone(&observed_scroll_y);
    let bridge = app(DemoState::default())
        .view(|state: &DemoState| {
            ViewProjection::from_surface(UiSurface::new(SurfaceNode::scroll_area(
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
            )))
        })
        .on_scroll(move |state, update, _context| {
            state.last_scroll_y = update.offset.y;
            *observed_scroll_y_for_hook
                .lock()
                .expect("scroll observer lock should be available") = Some(update.offset.y);
        })
        .handle_message(|_state, _message: DemoMessage, _context| {})
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
