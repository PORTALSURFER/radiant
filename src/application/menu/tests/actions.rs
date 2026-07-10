use super::{MenuMessage, click};
use crate::{
    application::{IntoView, MenuCommand, message_menu},
    gui::types::{Point, Rect},
    layout::Vector2,
    runtime::{DeclarativeOwnedRuntimeBridge, PaintPrimitive, SurfaceRuntime, UiSurface},
    widgets::WidgetTone,
};

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

    click(&mut runtime, Point::new(20.0, 70.0));

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
fn menu_command_style_helpers_are_generic() {
    let command = MenuCommand::new("Open", MenuMessage::Open).primary();
    assert_eq!(command.style.tone, WidgetTone::Accent);
}
