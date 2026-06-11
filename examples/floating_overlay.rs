//! Floating overlay application-builder example.

use radiant::prelude::*;

#[derive(Clone, Debug, PartialEq, Eq)]
enum OverlayMessage {
    CountClick,
    ToggleMenu,
}

#[derive(Default)]
struct OverlayExampleState {
    clicks: usize,
    menu_open: bool,
}

fn main() -> radiant::Result {
    radiant::app(OverlayExampleState::default())
        .title("Radiant Floating Overlay")
        .size(420, 220)
        .min_size(360, 180)
        .view(|state| {
            let page = column([
                text("Floating Overlay").height(28.0).fill_width(),
                text(format!("Underlying button clicks: {}", state.clicks))
                    .height(24.0)
                    .fill_width(),
                button("Button under the floating layer")
                    .message(OverlayMessage::CountClick)
                    .height(32.0)
                    .fill_width(),
                button(if state.menu_open {
                    "Hide Overlay"
                } else {
                    "Show Overlay"
                })
                .primary()
                .message(OverlayMessage::ToggleMenu)
                .height(32.0)
                .fill_width(),
            ])
            .padding(16.0)
            .spacing(8.0)
            .fill_width()
            .fill_height();

            if state.menu_open {
                stack([
                    page,
                    floating_layer(
                        Point::new(18.0, 70.0),
                        Vector2::new(260.0, 92.0),
                        overlay_menu(),
                    )
                    .key("floating-overlay-layer")
                    .fill(),
                ])
                .fill_width()
                .fill_height()
            } else {
                page
            }
        })
        .update(update)
        .run()
}

fn update(state: &mut OverlayExampleState, message: OverlayMessage) {
    match message {
        OverlayMessage::CountClick => state.clicks += 1,
        OverlayMessage::ToggleMenu => state.menu_open = !state.menu_open,
    }
}

fn overlay_menu<Message: 'static>() -> ViewNode<Message> {
    column([
        overlay_row("Bass", "Sounds"),
        overlay_row("Drums", "Sounds"),
        overlay_row("Loop", "Clips"),
        overlay_row("One Shot", "Clips"),
    ])
    .style(WidgetStyle {
        tone: WidgetTone::Neutral,
        prominence: WidgetProminence::Strong,
    })
    .padding(4.0)
    .spacing(2.0)
    .fill_width()
    .height(92.0)
}

fn overlay_row<Message: 'static>(tag: &'static str, group: &'static str) -> ViewNode<Message> {
    row([
        text(tag).height(20.0).width(108.0).truncate(),
        text(group).height(20.0).fill_width().truncate(),
    ])
    .style(WidgetStyle {
        tone: WidgetTone::Accent,
        prominence: WidgetProminence::Subtle,
    })
    .height(20.0)
    .fill_width()
    .spacing(8.0)
}
