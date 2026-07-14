//! Context-menu application-builder helper.

use radiant::layout::{Point, Vector2};
use radiant::prelude::*;

#[derive(Clone, Debug, PartialEq, Eq)]
enum ContextMenuMessage {
    OpenMenu,
    CloseMenu,
    ApplyAction(&'static str),
}

#[derive(Clone, Debug)]
struct ContextMenuState {
    menu_open: bool,
    anchor: Point,
    status: String,
}

impl Default for ContextMenuState {
    fn default() -> Self {
        Self {
            menu_open: false,
            anchor: Point::new(312.0, 120.0),
            status: "Open the context menu".to_string(),
        }
    }
}

fn main() -> radiant::Result {
    radiant::app(ContextMenuState::default())
        .title("Radiant Context Menu")
        .size(520, 260)
        .min_size(420, 220)
        .view(|state| {
            let page = column([
                text("Context Menu").height(26.0).fill_width(),
                text(state.status.clone()).height(28.0).fill_width(),
                button("Open Menu")
                    .primary()
                    .message(ContextMenuMessage::OpenMenu)
                    .width(140.0)
                    .height(32.0),
            ])
            .style(WidgetStyle::default())
            .fill_width()
            .fill_height()
            .padding(16.0)
            .spacing(10.0);

            if state.menu_open {
                stack([
                    page,
                    context_menu(
                        "Actions",
                        [
                            MenuCommand::new("Inspect", ContextMenuMessage::ApplyAction("Inspect"))
                                .primary(),
                            MenuCommand::new(
                                "Duplicate",
                                ContextMenuMessage::ApplyAction("Duplicate"),
                            )
                            .subtle(),
                            MenuCommand::new("Delete", ContextMenuMessage::ApplyAction("Delete"))
                                .danger(),
                            MenuCommand::new("Cancel", ContextMenuMessage::CloseMenu).subtle(),
                        ],
                    )
                    .anchor(state.anchor)
                    .size(Vector2::new(180.0, 144.0))
                    .view()
                    .key("context-menu-overlay"),
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

fn update(state: &mut ContextMenuState, message: ContextMenuMessage) {
    match message {
        ContextMenuMessage::OpenMenu => {
            state.menu_open = true;
            state.status = "Context menu opened".to_string();
        }
        ContextMenuMessage::CloseMenu => {
            state.menu_open = false;
            state.status = "Context menu closed".to_string();
        }
        ContextMenuMessage::ApplyAction(label) => {
            state.menu_open = false;
            state.status = format!("Selected: {label}");
        }
    }
}
