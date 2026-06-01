use super::*;
use crate::{
    application::IntoView,
    gui::types::{Point, Rect},
    layout::Vector2,
    runtime::{DeclarativeOwnedRuntimeBridge, Event, PaintPrimitive, SurfaceRuntime, UiSurface},
    widgets::{PointerButton, PointerModifiers, WidgetTone},
};

#[derive(Clone, Debug, PartialEq)]
enum MenuMessage {
    Open,
    Delete,
    Close,
}

#[test]
fn message_menu_emits_host_messages() {
    let bridge = DeclarativeOwnedRuntimeBridge::new(
        Vec::<MenuMessage>::new(),
        |_| {
            UiSurface::new(
                message_menu(
                    "Actions",
                    [
                        MenuCommand::new("Open", MenuMessage::Open),
                        MenuCommand::new("Delete", MenuMessage::Delete).danger(),
                    ],
                )
                .into_node(),
            )
        },
        |messages: &mut Vec<MenuMessage>, message| messages.push(message),
    );
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(240.0, 120.0));
    let delete_command = Point::new(20.0, 70.0);

    runtime.dispatch_event(Event::PointerPress {
        position: delete_command,
        button: PointerButton::Primary,
        modifiers: PointerModifiers::default(),
    });
    runtime.dispatch_event(Event::PointerRelease {
        position: delete_command,
        button: PointerButton::Primary,
        modifiers: PointerModifiers::default(),
    });

    assert_eq!(runtime.bridge().state(), &[MenuMessage::Delete]);
}

#[test]
fn message_menu_applies_command_styles() {
    let frame = UiSurface::new(
        message_menu(
            "Actions",
            [MenuCommand::new("Delete", MenuMessage::Delete).danger()],
        )
        .into_node(),
    )
    .frame(
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(240.0, 120.0)),
        &Default::default(),
    );

    assert!(
        frame.paint_plan.primitives.iter().any(|primitive| {
            matches!(
                primitive,
                PaintPrimitive::FillRect(fill) if fill.color.r > fill.color.g
            )
        }),
        "danger commands should apply the danger-toned button style"
    );
}

#[test]
fn dismissible_context_menu_backing_emits_dismiss_message() {
    let bridge = DeclarativeOwnedRuntimeBridge::new(
        Vec::<MenuMessage>::new(),
        |_| {
            UiSurface::new(
                dismissible_context_menu(
                    Point::new(80.0, 90.0),
                    Vector2::new(200.0, 96.0),
                    "Actions",
                    [MenuCommand::new("Open", MenuMessage::Open)],
                    MenuMessage::Close,
                )
                .into_node(),
            )
        },
        |messages: &mut Vec<MenuMessage>, message| messages.push(message),
    );
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(640.0, 360.0));

    runtime.dispatch_event(Event::PointerPress {
        position: Point::new(12.0, 12.0),
        button: PointerButton::Primary,
        modifiers: PointerModifiers::default(),
    });
    runtime.dispatch_event(Event::PointerRelease {
        position: Point::new(12.0, 12.0),
        button: PointerButton::Primary,
        modifiers: PointerModifiers::default(),
    });

    assert_eq!(runtime.bridge().state(), &[MenuMessage::Close]);
}

#[test]
fn dismissible_context_menu_with_width_uses_standard_menu_height() {
    let frame = UiSurface::new(
        dismissible_context_menu_with_width(
            Point::new(80.0, 90.0),
            200.0,
            "Actions",
            [
                MenuCommand::new("Open", MenuMessage::Open),
                MenuCommand::new("Delete", MenuMessage::Delete).danger(),
            ],
            MenuMessage::Close,
        )
        .into_node(),
    )
    .frame(
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(640.0, 360.0)),
        &Default::default(),
    );

    let menu_rects = frame
        .paint_plan
        .primitives
        .iter()
        .filter_map(|primitive| match primitive {
            PaintPrimitive::FillRect(fill) => Some(fill.rect),
            _ => None,
        })
        .collect::<Vec<_>>();

    assert!(
        menu_rects.iter().any(|rect| {
            rect.min.x >= 80.0 && (rect.height() - message_menu_height(2)).abs() < 0.01
        }),
        "standard-width menu should paint at standard compact height: {menu_rects:?}"
    );
}

#[test]
fn menu_command_style_helpers_are_generic() {
    let command = MenuCommand::new("Open", MenuMessage::Open).primary();
    assert_eq!(command.style.tone, WidgetTone::Accent);
}
