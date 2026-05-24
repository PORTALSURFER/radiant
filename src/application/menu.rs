use crate::{
    application::{StateView, button, column, row, text},
    gui::types::{Point, Rect},
    layout::Vector2,
    widgets::{WidgetProminence, WidgetStyle, WidgetTone},
};
use std::sync::Arc;

mod model;

pub use model::{ContextMenuOverlayParts, MenuItem, MenuItemParts, MenuParts};

/// Build a compact vertical menu.
pub fn menu<State: 'static>(
    title: impl Into<String>,
    items: impl IntoIterator<Item = MenuItem<State>>,
) -> StateView<State> {
    menu_from_parts(MenuParts {
        title: title.into(),
        items: items.into_iter().collect(),
    })
}

/// Build a compact vertical menu from named parts.
pub fn menu_from_parts<State: 'static>(parts: MenuParts<State>) -> StateView<State> {
    column([
        text(parts.title).fill_width().height(22.0),
        column(
            parts
                .items
                .into_iter()
                .enumerate()
                .map(|(index, item)| menu_item_button(index, item)),
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
    context_menu_overlay_from_parts(ContextMenuOverlayParts {
        bounds,
        anchor,
        size,
        title: title.into(),
        items: items.into_iter().collect(),
    })
}

/// Build a context menu overlay from named parts.
pub fn context_menu_overlay_from_parts<State: 'static>(
    parts: ContextMenuOverlayParts<State>,
) -> StateView<State> {
    let rect = crate::gui::panel::anchored_panel_rect_from_parts(
        crate::gui::panel::AnchoredPanelRectParts {
            bounds: parts.bounds,
            anchor: parts.anchor,
            size: parts.size,
            inset: 0.0,
        },
    );
    let top = (rect.min.y - parts.bounds.min.y).max(0.0);
    let left = (rect.min.x - parts.bounds.min.x).max(0.0);
    column([
        text("").fill_width().height(top),
        row([
            text("").size(left, 1.0),
            menu_from_parts(MenuParts {
                title: parts.title,
                items: parts.items,
            })
            .size(parts.size.x, parts.size.y),
            text("").fill_width().height(1.0),
        ])
        .fill_width()
        .height(parts.size.y),
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
