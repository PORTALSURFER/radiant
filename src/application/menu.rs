use crate::{
    application::{StateCallback, StateView, button, column, row, text},
    gui::types::{Point, Rect},
    layout::Vector2,
    widgets::{WidgetProminence, WidgetStyle, WidgetTone},
};
use std::sync::Arc;

/// One clickable item in a generic menu or context menu.
pub struct MenuItem<State> {
    label: String,
    style: WidgetStyle,
    on_select: StateCallback<State>,
}

impl<State> MenuItem<State> {
    /// Build a menu item that runs the supplied state callback when activated.
    pub fn new(
        label: impl Into<String>,
        on_select: impl Fn(&mut State) + Send + Sync + 'static,
    ) -> Self {
        Self {
            label: label.into(),
            style: WidgetStyle::default(),
            on_select: Arc::new(on_select),
        }
    }

    /// Use accent styling for a primary menu action.
    pub fn primary(mut self) -> Self {
        self.style = WidgetStyle {
            tone: WidgetTone::Accent,
            prominence: WidgetProminence::Strong,
        };
        self
    }

    /// Use danger styling for a destructive menu action.
    pub fn danger(mut self) -> Self {
        self.style = WidgetStyle {
            tone: WidgetTone::Danger,
            prominence: WidgetProminence::Strong,
        };
        self
    }

    /// Use lower-prominence styling for a secondary menu action.
    pub fn subtle(mut self) -> Self {
        self.style.prominence = WidgetProminence::Subtle;
        self
    }
}

/// Build a compact vertical menu.
pub fn menu<State: 'static>(
    title: impl Into<String>,
    items: impl IntoIterator<Item = MenuItem<State>>,
) -> StateView<State> {
    column([
        text(title.into()).fill_width().height(22.0),
        column(
            items
                .into_iter()
                .enumerate()
                .map(|(index, item)| menu_item_button(index, item))
                .collect::<Vec<_>>(),
        )
        .fill_width()
        .spacing(4.0),
    ])
    .style(WidgetStyle {
        tone: WidgetTone::Accent,
        prominence: WidgetProminence::Strong,
    })
    .fill_width()
    .padding(8.0)
    .spacing(6.0)
}

/// Build a context menu overlaid at an anchored surface position.
pub fn context_menu_overlay<State: 'static>(
    bounds: Rect,
    anchor: Point,
    size: Vector2,
    title: impl Into<String>,
    items: impl IntoIterator<Item = MenuItem<State>>,
) -> StateView<State> {
    let rect = crate::gui::panel::anchored_panel_rect(bounds, anchor, size, 0.0);
    let top = (rect.min.y - bounds.min.y).max(0.0);
    let left = (rect.min.x - bounds.min.x).max(0.0);
    column([
        text("").fill_width().height(top),
        row([
            text("").size(left, 1.0),
            menu(title, items).size(size.x, size.y),
            text("").fill_width().height(1.0),
        ])
        .fill_width()
        .height(size.y),
        text("").fill_width().fill_height(),
    ])
    .fill_width()
    .fill_height()
}

fn menu_item_button<State: 'static>(index: usize, item: MenuItem<State>) -> StateView<State> {
    let on_select = Arc::clone(&item.on_select);
    button(item.label)
        .on_click(move |state: &mut State| on_select(state))
        .key(format!("menu-item-{index}"))
        .style(item.style)
        .fill_width()
        .height(28.0)
}
