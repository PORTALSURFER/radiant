//! Context-menu application-builder helper.

use radiant::prelude::*;
use radiant::{
    gui::types::Rect,
    layout::{Point, Vector2},
};

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

impl ContextMenuState {
    fn open_menu(&mut self) {
        self.menu_open = true;
        self.status = "Context menu opened".to_string();
    }

    fn close_menu(&mut self) {
        self.menu_open = false;
        self.status = "Context menu closed".to_string();
    }

    fn apply_action(&mut self, label: &'static str) {
        self.menu_open = false;
        self.status = format!("Selected: {label}");
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
                    .on_click(ContextMenuState::open_menu)
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
                    context_menu_overlay(
                        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(520.0, 260.0)),
                        state.anchor,
                        Vector2::new(180.0, 144.0),
                        "Actions",
                        [
                            MenuItem::new("Inspect", |state: &mut ContextMenuState| {
                                state.apply_action("Inspect")
                            })
                            .primary(),
                            MenuItem::new("Duplicate", |state: &mut ContextMenuState| {
                                state.apply_action("Duplicate")
                            })
                            .subtle(),
                            MenuItem::new("Delete", |state: &mut ContextMenuState| {
                                state.apply_action("Delete")
                            })
                            .danger(),
                            MenuItem::new("Cancel", ContextMenuState::close_menu).subtle(),
                        ],
                    )
                    .key("context-menu-overlay"),
                ])
                .fill_width()
                .fill_height()
            } else {
                page
            }
        })
        .run()
}
