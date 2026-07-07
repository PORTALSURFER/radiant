use super::super::{
    AnchoredPopoverAnchor, AnchoredPopoverParts, anchored_popover_from_parts,
    dismissible_anchored_popover_from_parts,
};
use crate::{
    application::{IntoView, button, stack, text},
    gui::types::{Point, Rect},
    layout::Vector2,
    runtime::{DeclarativeOwnedRuntimeBridge, Event, PaintPrimitive, SurfaceRuntime, UiSurface},
    widgets::{PointerButton, PointerModifiers},
};

#[derive(Clone, Debug, PartialEq)]
enum Message {
    Activate,
    Close,
}

#[test]
fn anchored_popover_positions_below_trigger_with_gap() {
    let frame = UiSurface::new(
        stack([
            text("").size(200.0, 120.0),
            anchored_popover_from_parts(
                AnchoredPopoverParts::<()>::below(
                    text("popup").id(92),
                    AnchoredPopoverAnchor::trigger(12.0, 20.0, 80.0, 24.0),
                    Vector2::new(100.0, 30.0),
                )
                .gap(4.0),
            ),
        ])
        .into_node(),
    )
    .frame(
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(200.0, 120.0)),
        &Default::default(),
    );

    let text_rect = frame
        .paint_plan
        .primitives
        .iter()
        .find_map(|primitive| match primitive {
            PaintPrimitive::Text(text) if text.widget_id == 92 => Some(text.rect),
            _ => None,
        })
        .expect("popover child should paint");

    assert!((text_rect.min.x - 12.0).abs() < 0.01, "{text_rect:?}");
    assert!((text_rect.min.y - 48.0).abs() < 0.01, "{text_rect:?}");
}

#[test]
fn anchored_popover_clamps_horizontal_origin_to_viewport() {
    let frame = UiSurface::new(
        stack([
            text("").size(200.0, 120.0),
            anchored_popover_from_parts(AnchoredPopoverParts::<()>::below(
                text("popup").id(93),
                AnchoredPopoverAnchor::pointer(Point::new(180.0, 20.0)),
                Vector2::new(80.0, 30.0),
            )),
        ])
        .into_node(),
    )
    .frame(
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(200.0, 120.0)),
        &Default::default(),
    );

    let text_rect = frame
        .paint_plan
        .primitives
        .iter()
        .find_map(|primitive| match primitive {
            PaintPrimitive::Text(text) if text.widget_id == 93 => Some(text.rect),
            _ => None,
        })
        .expect("popover child should paint");

    assert!((text_rect.min.x - 120.0).abs() < 0.01, "{text_rect:?}");
}

#[test]
fn anchored_popover_routes_clicks_after_flip_and_clamp() {
    let bridge = DeclarativeOwnedRuntimeBridge::new(
        Vec::<Message>::new(),
        |_| {
            UiSurface::new(
                stack([
                    text("").size(200.0, 120.0),
                    anchored_popover_from_parts(AnchoredPopoverParts::below(
                        button("Run").message(Message::Activate),
                        AnchoredPopoverAnchor::pointer(Point::new(180.0, 112.0)),
                        Vector2::new(80.0, 30.0),
                    )),
                ])
                .into_node(),
            )
        },
        |messages: &mut Vec<Message>, message| messages.push(message),
    );
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(200.0, 120.0));
    let painted_button = Point::new(140.0, 96.0);

    runtime.dispatch_event(Event::PointerPress {
        position: painted_button,
        button: PointerButton::Primary,
        modifiers: PointerModifiers::default(),
    });
    runtime.dispatch_event(Event::PointerRelease {
        position: painted_button,
        button: PointerButton::Primary,
        modifiers: PointerModifiers::default(),
    });

    assert_eq!(runtime.bridge().state(), &[Message::Activate]);
}

#[test]
fn dismissible_anchored_popover_backing_emits_close_message() {
    let bridge = DeclarativeOwnedRuntimeBridge::new(
        Vec::<Message>::new(),
        |_| {
            UiSurface::new(
                dismissible_anchored_popover_from_parts(
                    AnchoredPopoverParts::below(
                        button("Run").message(Message::Activate),
                        AnchoredPopoverAnchor::pointer(Point::new(80.0, 40.0)),
                        Vector2::new(80.0, 30.0),
                    ),
                    Message::Close,
                )
                .into_node(),
            )
        },
        |messages: &mut Vec<Message>, message| messages.push(message),
    );
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(200.0, 120.0));
    let outside = Point::new(8.0, 8.0);

    runtime.dispatch_event(Event::PointerPress {
        position: outside,
        button: PointerButton::Primary,
        modifiers: PointerModifiers::default(),
    });
    runtime.dispatch_event(Event::PointerRelease {
        position: outside,
        button: PointerButton::Primary,
        modifiers: PointerModifiers::default(),
    });

    assert_eq!(runtime.bridge().state(), &[Message::Close]);
}
