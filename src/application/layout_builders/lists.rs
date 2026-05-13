//! List-oriented layout builders.

use super::containers::{column, row, row_key};
use super::scroll::{scroll, virtual_scroll};
use crate::application::{ViewNode, spacer};
use crate::gui::list::VirtualListWindow;
use crate::widgets::WidgetStyle;

/// Build a scroll viewport containing a column projected from an iterator.
pub fn scroll_column<Message, Item>(
    items: impl IntoIterator<Item = Item>,
    project: impl FnMut(Item) -> ViewNode<Message>,
) -> ViewNode<Message> {
    scroll(column(items.into_iter().map(project)))
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
    virtual_scroll(column(items.into_iter().map(project)), overscan_px)
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        application::{IntoView, button},
        layout::{LayoutNode, NodeId},
    };

    #[test]
    fn virtual_list_window_projects_only_materialized_range() {
        let window = VirtualListWindow {
            total_items: 10_000,
            viewport_start: 120,
            viewport_end: 128,
            window_start: 116,
            window_end: 132,
        };
        let mut projected = Vec::new();

        let view: ViewNode<()> = virtual_list_window(
            window,
            32.0,
            |index| {
                projected.push(index);
                list_row_id(
                    10_000 + index as NodeId,
                    [button(format!("Row {index:05}"))
                        .message(())
                        .id(20_000 + index as NodeId)],
                )
            },
            64.0,
        );

        assert_eq!(projected, (116..132).collect::<Vec<_>>());
        let layout = view.into_surface().layout_node();
        assert!(
            count_layout_nodes(&layout) < 64,
            "windowed projection should stay bounded to materialized rows"
        );
    }

    fn count_layout_nodes(node: &LayoutNode) -> usize {
        match node {
            LayoutNode::Widget(_) => 1,
            LayoutNode::Container(container) => {
                1 + container
                    .children
                    .iter()
                    .map(|child| count_layout_nodes(&child.child))
                    .sum::<usize>()
            }
        }
    }
}
