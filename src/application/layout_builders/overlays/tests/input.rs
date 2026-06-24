use super::super::{
    dismissible_overlay, dismissible_overlay_with_interactive_base, floating_layer_with_input,
    input_overlay, input_underlay,
};
use crate::{
    application::{app, button, row, text},
    gui::types::Point,
    layout::Vector2,
    runtime::{Event, SurfaceRuntime},
    widgets::{PointerButton, PointerModifiers, TextWidget, WidgetInput},
};

#[derive(Clone, Debug, PartialEq)]
enum DemoMessage {
    Activate,
    Dismiss,
}

#[derive(Default)]
struct DemoState {
    activated: bool,
    dismissed: bool,
}

#[test]
fn input_overlay_routes_transparent_input_above_content() {
    let bridge = app(DemoState::default())
        .view(|state| {
            input_overlay(
                text(if state.activated { "activated" } else { "idle" })
                    .id(90)
                    .fill_width()
                    .height(22.0),
                button("").message(DemoMessage::Activate).fill(),
            )
            .fill()
        })
        .update(|state, message| match message {
            DemoMessage::Activate => state.activated = true,
            DemoMessage::Dismiss => state.dismissed = true,
        })
        .into_bridge();
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(120.0, 22.0));
    let position = Point::new(8.0, 8.0);

    runtime.dispatch_input_at(
        position,
        WidgetInput::PointerPress {
            position,
            button: PointerButton::Primary,
            modifiers: PointerModifiers::default(),
        },
    );
    runtime.dispatch_input_at(
        position,
        WidgetInput::PointerRelease {
            position,
            button: PointerButton::Primary,
            modifiers: PointerModifiers::default(),
        },
    );

    assert_eq!(
        runtime
            .surface()
            .find_widget(90)
            .and_then(|widget| widget.widget_object().as_any().downcast_ref::<TextWidget>())
            .map(|widget| widget.text.as_str()),
        Some("activated")
    );
}

#[test]
fn input_underlay_routes_input_below_visible_content() {
    let bridge = app(DemoState::default())
        .view(|state| {
            input_underlay(
                text(if state.activated { "activated" } else { "idle" })
                    .id(91)
                    .fill_width()
                    .height(22.0),
                button("").message(DemoMessage::Activate).fill(),
            )
            .fill()
        })
        .update(|state, message| match message {
            DemoMessage::Activate => state.activated = true,
            DemoMessage::Dismiss => state.dismissed = true,
        })
        .into_bridge();
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(120.0, 22.0));
    let position = Point::new(8.0, 8.0);

    runtime.dispatch_input_at(
        position,
        WidgetInput::PointerPress {
            position,
            button: PointerButton::Primary,
            modifiers: PointerModifiers::default(),
        },
    );
    runtime.dispatch_input_at(
        position,
        WidgetInput::PointerRelease {
            position,
            button: PointerButton::Primary,
            modifiers: PointerModifiers::default(),
        },
    );

    assert_eq!(
        runtime
            .surface()
            .find_widget(91)
            .and_then(|widget| widget.widget_object().as_any().downcast_ref::<TextWidget>())
            .map(|widget| widget.text.as_str()),
        Some("activated")
    );
}

#[test]
fn dismissible_overlay_routes_outside_activation_to_dismiss_layer() {
    let bridge = app(DemoState::default())
        .view(|state| {
            let status = if state.dismissed {
                "dismissed"
            } else if state.activated {
                "activated"
            } else {
                "open"
            };
            dismissible_overlay(
                text(status).id(92).fill(),
                floating_layer_with_input(
                    Point::new(0.0, 0.0),
                    Vector2::new(60.0, 24.0),
                    button("menu").message(DemoMessage::Activate).fill(),
                    true,
                ),
                DemoMessage::Dismiss,
            )
        })
        .update(|state, message| match message {
            DemoMessage::Activate => state.activated = true,
            DemoMessage::Dismiss => state.dismissed = true,
        })
        .into_bridge();
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(140.0, 80.0));
    let outside_overlay = Point::new(90.0, 8.0);

    runtime.dispatch_event(Event::PointerPress {
        position: outside_overlay,
        button: PointerButton::Primary,
        modifiers: PointerModifiers::default(),
    });
    runtime.dispatch_event(Event::PointerRelease {
        position: outside_overlay,
        button: PointerButton::Primary,
        modifiers: PointerModifiers::default(),
    });

    assert_eq!(
        runtime
            .surface()
            .find_widget(92)
            .and_then(|widget| widget.widget_object().as_any().downcast_ref::<TextWidget>())
            .map(|widget| widget.text.as_str()),
        Some("dismissed")
    );
}

#[test]
fn dismissible_overlay_with_interactive_base_keeps_base_controls_above_dismiss_layer() {
    let bridge = app(DemoState::default())
        .view(|state| {
            let status = if state.dismissed {
                "dismissed"
            } else if state.activated {
                "activated"
            } else {
                "open"
            };
            let base = row([
                button("base").message(DemoMessage::Activate).width(60.0),
                text(status).id(93).width(80.0),
            ])
            .height(24.0);
            dismissible_overlay_with_interactive_base(
                base,
                floating_layer_with_input(
                    Point::new(80.0, 0.0),
                    Vector2::new(40.0, 24.0),
                    button("menu").message(DemoMessage::Activate).fill(),
                    true,
                ),
                DemoMessage::Dismiss,
            )
        })
        .update(|state, message| match message {
            DemoMessage::Activate => state.activated = true,
            DemoMessage::Dismiss => state.dismissed = true,
        })
        .into_bridge();
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(160.0, 60.0));

    runtime.dispatch_primary_click(Point::new(12.0, 8.0));

    assert_eq!(
        runtime
            .surface()
            .find_widget(93)
            .and_then(|widget| widget.widget_object().as_any().downcast_ref::<TextWidget>())
            .map(|widget| widget.text.as_str()),
        Some("activated")
    );
}

#[test]
fn dismissible_overlay_with_interactive_base_dismisses_noninteractive_base_space() {
    let bridge = app(DemoState::default())
        .view(|state| {
            let status = if state.dismissed { "dismissed" } else { "open" };
            let base = row([
                button("base").message(DemoMessage::Activate).width(60.0),
                text(status).id(94).width(80.0),
            ])
            .height(24.0);
            dismissible_overlay_with_interactive_base(
                base,
                floating_layer_with_input(
                    Point::new(80.0, 0.0),
                    Vector2::new(40.0, 24.0),
                    button("menu").message(DemoMessage::Activate).fill(),
                    true,
                ),
                DemoMessage::Dismiss,
            )
        })
        .update(|state, message| match message {
            DemoMessage::Activate => state.activated = true,
            DemoMessage::Dismiss => state.dismissed = true,
        })
        .into_bridge();
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(160.0, 60.0));

    runtime.dispatch_primary_click(Point::new(150.0, 8.0));

    assert_eq!(
        runtime
            .surface()
            .find_widget(94)
            .and_then(|widget| widget.widget_object().as_any().downcast_ref::<TextWidget>())
            .map(|widget| widget.text.as_str()),
        Some("dismissed")
    );
}
