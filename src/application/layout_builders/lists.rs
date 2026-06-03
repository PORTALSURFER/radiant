//! List-oriented layout builders.

use super::containers::{column, row, row_key};
use super::scroll::{scroll, virtual_scroll};
use crate::application::{ViewNode, empty, spacer};
use crate::gui::list::{VirtualListWindow, bounded_list_height};
use crate::widgets::WidgetStyle;

#[cfg(test)]
#[path = "lists/tests.rs"]
mod tests;

/// Named construction fields for a fixed-row scroll column capped to a maximum
/// visible row count.
pub struct BoundedScrollColumnParts<Message> {
    /// Fixed-height rows projected into the scroll column.
    pub rows: Vec<ViewNode<Message>>,
    /// Maximum number of rows visible before scrolling.
    pub max_visible_rows: usize,
    /// Fixed row height used for height capping.
    pub row_height: f32,
    /// Vertical chrome included in the capped scroll height.
    pub vertical_chrome: f32,
    /// Style applied to the scroll viewport.
    pub style: WidgetStyle,
    /// Padding applied inside the scroll viewport.
    pub padding: f32,
}

impl<Message> BoundedScrollColumnParts<Message> {
    /// Build bounded scroll-column parts from rows and fixed-row metrics.
    pub fn new(
        rows: Vec<ViewNode<Message>>,
        max_visible_rows: usize,
        row_height: f32,
        vertical_chrome: f32,
    ) -> Self {
        Self {
            rows,
            max_visible_rows,
            row_height,
            vertical_chrome,
            style: WidgetStyle::default(),
            padding: 0.0,
        }
    }

    /// Set the style applied to the scroll viewport.
    pub fn style(mut self, style: WidgetStyle) -> Self {
        self.style = style;
        self
    }

    /// Set uniform padding inside the scroll viewport.
    pub fn padding(mut self, padding: f32) -> Self {
        self.padding = padding;
        self
    }
}

/// Build a scroll viewport containing a column projected from an iterator.
pub fn scroll_column<Message, Item>(
    items: impl IntoIterator<Item = Item>,
    project: impl FnMut(Item) -> ViewNode<Message>,
) -> ViewNode<Message> {
    scroll(column(items.into_iter().map(project)))
}

/// Build a fixed-row scroll column capped to a maximum visible row count.
///
/// This is useful for transient autocomplete popups, command-palette results,
/// compact menu bodies, and other small bounded lists where application code
/// should project rows but not repeat scroll-height capping and empty-list
/// handling.
pub fn bounded_scroll_column<Message: 'static>(
    rows: Vec<ViewNode<Message>>,
    max_visible_rows: usize,
    row_height: f32,
    vertical_chrome: f32,
) -> ViewNode<Message> {
    bounded_scroll_column_from_parts(BoundedScrollColumnParts::new(
        rows,
        max_visible_rows,
        row_height,
        vertical_chrome,
    ))
}

/// Build a fixed-row scroll column from named parts.
pub fn bounded_scroll_column_from_parts<Message: 'static>(
    parts: BoundedScrollColumnParts<Message>,
) -> ViewNode<Message> {
    let height = bounded_list_height(
        parts.rows.len(),
        parts.max_visible_rows,
        parts.row_height,
        parts.vertical_chrome,
    );
    if height <= 0.0 {
        return empty().fill_width().height(0.0);
    }
    scroll(column(parts.rows).spacing(0.0).fill_width())
        .style(parts.style)
        .padding(parts.padding)
        .fill_width()
        .height(height)
}

/// Build a scrollable vertical list with stable intrinsic-height rows.
pub fn list<Message, Item>(
    items: impl IntoIterator<Item = Item>,
    project: impl FnMut(Item) -> ViewNode<Message>,
) -> ViewNode<Message> {
    scroll_column(items, project)
        .style(WidgetStyle::default())
        .fill_height()
}

/// Build a vertically virtualized list with stable intrinsic-height rows.
///
/// This helper still projects every item before the layout engine virtualizes
/// the scroll viewport. Prefer [`virtual_list_window`] for large fixed-height
/// lists when the host can resolve a [`VirtualListWindow`] from logical scroll
/// state.
pub fn virtual_list<Message, Item>(
    items: impl IntoIterator<Item = Item>,
    project: impl FnMut(Item) -> ViewNode<Message>,
    overscan_px: f32,
) -> ViewNode<Message> {
    virtual_scroll(
        column(items.into_iter().map(project)).spacing(0.0),
        overscan_px,
    )
    .style(WidgetStyle::default())
    .fill_height()
}

/// Build a vertically virtualized fixed-row list from a pre-resolved logical window.
///
/// Unlike [`virtual_list`], this helper only calls `project` for
/// `window.window_start..window.window_end`. It is intended for large
/// item-indexed lists whose host state already owns logical scroll position or
/// focus-follow navigation through [`VirtualListWindow`].
pub fn virtual_list_window<Message: 'static>(
    window: VirtualListWindow,
    row_height: f32,
    mut project: impl FnMut(usize) -> ViewNode<Message>,
    overscan_px: f32,
) -> ViewNode<Message> {
    let row_height = row_height.max(0.0);
    let projected_len = window.window_len();
    let mut children = Vec::with_capacity(projected_len + 2);

    let top_spacer_height = row_height * window.window_start as f32;
    if top_spacer_height > 0.0 {
        children.push(spacer().height(top_spacer_height).fill_width());
    }

    children.extend(
        (window.window_start..window.window_end)
            .map(|index| project(index).height(row_height).fill_width()),
    );

    let bottom_items = window.total_items.saturating_sub(window.window_end);
    let bottom_spacer_height = row_height * bottom_items as f32;
    if bottom_spacer_height > 0.0 {
        children.push(spacer().height(bottom_spacer_height).fill_width());
    }

    virtual_scroll(column(children).spacing(0.0), overscan_px)
        .style(WidgetStyle::default())
        .fill_height()
}

/// Build a keyed list row with full-width, fixed-height defaults.
pub fn list_row<Message>(
    key: impl ToString,
    children: impl IntoIterator<Item = ViewNode<Message>>,
) -> ViewNode<Message> {
    apply_list_row_chrome(row_key(key, children))
}

/// Build a list row with a direct numeric id instead of a string key.
///
/// Prefer this for large numeric collections when the caller already owns
/// stable item ids; it avoids per-row key string allocation during projection.
pub fn list_row_id<Message>(
    id: u64,
    children: impl IntoIterator<Item = ViewNode<Message>>,
) -> ViewNode<Message> {
    apply_list_row_chrome(row(children).id(id))
}

fn apply_list_row_chrome<Message>(row: ViewNode<Message>) -> ViewNode<Message> {
    row.style(WidgetStyle::default())
        .hoverable()
        .fill_width()
        .height(44.0)
        .padding_x(12.0)
        .padding_y(7.0)
        .spacing(10.0)
}
