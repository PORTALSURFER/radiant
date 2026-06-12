use super::super::containers::column;
use super::super::scroll::virtual_scroll;
use crate::application::{ViewNode, spacer};
use crate::gui::list::VirtualListWindow;
use crate::widgets::WidgetStyle;

/// Build a vertically virtualized fixed-row list from a pre-resolved logical window.
///
/// This helper only calls `project` for `window.window_start..window.window_end`.
/// It is intended for large item-indexed lists whose host state already owns
/// logical scroll position or focus-follow navigation through [`VirtualListWindow`].
pub fn virtual_list_window<Message: 'static>(
    window: VirtualListWindow,
    row_height: f32,
    mut project: impl FnMut(usize) -> ViewNode<Message>,
    overscan_px: f32,
) -> ViewNode<Message> {
    virtual_list_window_body(
        window,
        row_height,
        |window| {
            column(
                (window.window_start..window.window_end)
                    .map(|index| project(index).height(row_height).fill_width()),
            )
            .spacing(0.0)
            .fill_width()
            .height(row_height.max(0.0) * window.window_len() as f32)
        },
        overscan_px,
    )
}

/// Build a vertically virtualized fixed-row list from a materialized body view.
///
/// This is the lower-level companion to [`virtual_list_window`]. Use it when
/// the materialized range must be composed as one body, such as row groups,
/// table overlays, guide overlays, or other decoration that spans several
/// materialized rows. Radiant still owns the full-scroll spacer geometry while
/// the host projects only `window.window_start..window.window_end`.
pub fn virtual_list_window_body<Message: 'static>(
    window: VirtualListWindow,
    row_height: f32,
    body: impl FnOnce(VirtualListWindow) -> ViewNode<Message>,
    overscan_px: f32,
) -> ViewNode<Message> {
    let row_height = row_height.max(0.0);
    let projected_len = window.window_len();
    let mut children = Vec::with_capacity(projected_len + 2);

    let top_spacer_height = row_height * window.window_start as f32;
    if top_spacer_height > 0.0 {
        children.push(spacer().height(top_spacer_height).fill_width());
    }

    if projected_len > 0 {
        children.push(
            body(window)
                .fill_width()
                .height(row_height * projected_len as f32),
        );
    }

    let bottom_items = window.total_items.saturating_sub(window.window_end);
    let bottom_spacer_height = row_height * bottom_items as f32;
    if bottom_spacer_height > 0.0 {
        children.push(spacer().height(bottom_spacer_height).fill_width());
    }

    virtual_scroll(column(children).spacing(0.0), overscan_px)
        .style(WidgetStyle::default())
        .fill_height()
}
