use radiant::{
    prelude as ui,
    runtime::{Event, PaintPrimitive, SurfaceRuntime},
    theme::ThemeTokens,
    widgets::PointerButton,
};
use std::sync::{Arc, Mutex};

#[derive(Clone, Debug, PartialEq, Eq)]
enum OverlayMessage {
    Dismiss,
    Pick,
}

#[test]
fn application_builder_dropdown_overlay_routes_above_dismiss_layer() {
    let events = Arc::new(Mutex::new(Vec::<OverlayMessage>::new()));
    let captured_events = Arc::clone(&events);
    let bridge = ui::app(())
        .view(|_| {
            ui::stack([
                ui::button("Base")
                    .message(OverlayMessage::Dismiss)
                    .id(1000)
                    .fill(),
                ui::button("")
                    .message(OverlayMessage::Dismiss)
                    .id(1001)
                    .input_only()
                    .fill(),
                ui::dropdown_menu_overlay(
                    20.0,
                    20.0,
                    Some(100.0),
                    vec![ui::DropdownOption::from_parts(ui::DropdownOptionParts {
                        label: "WASAPI".into(),
                        selected: false,
                        message: OverlayMessage::Pick,
                    })],
                ),
            ])
            .fill()
        })
        .update(move |(), message| {
            captured_events.lock().expect("events lock").push(message);
        })
        .into_bridge();
    let mut runtime = SurfaceRuntime::new(bridge, ui::Vector2::new(180.0, 120.0));

    assert_eq!(runtime.widget_at(ui::Point::new(5.0, 5.0)), Some(1001));
    assert_eq!(
        runtime.dispatch_event(Event::PointerPress {
            position: ui::Point::new(5.0, 5.0),
            button: PointerButton::Primary,
            modifiers: Default::default(),
        }),
        Some(1001)
    );
    assert_eq!(
        runtime.dispatch_event(Event::PointerRelease {
            position: ui::Point::new(5.0, 5.0),
            button: PointerButton::Primary,
            modifiers: Default::default(),
        }),
        Some(1001)
    );
    assert_eq!(
        events.lock().expect("events lock").as_slice(),
        &[OverlayMessage::Dismiss]
    );

    let frame = runtime.frame(&ThemeTokens::default());
    let option_rect = frame
        .paint_plan
        .primitives
        .iter()
        .find_map(|primitive| match primitive {
            PaintPrimitive::Text(text) if text.text.as_str() == "WASAPI" => Some(text.rect),
            _ => None,
        })
        .expect("dropdown option should paint");
    let option_point = ui::Point::new(
        option_rect.min.x + option_rect.width() * 0.5,
        option_rect.min.y + option_rect.height() * 0.5,
    );

    assert_ne!(runtime.widget_at(option_point), Some(1001));
    assert!(
        runtime
            .dispatch_event(Event::PointerPress {
                position: option_point,
                button: PointerButton::Primary,
                modifiers: Default::default(),
            })
            .is_some()
    );
    assert!(
        runtime
            .dispatch_event(Event::PointerRelease {
                position: option_point,
                button: PointerButton::Primary,
                modifiers: Default::default(),
            })
            .is_some()
    );
    assert_eq!(
        events.lock().expect("events lock").as_slice(),
        &[OverlayMessage::Dismiss, OverlayMessage::Pick]
    );
}
