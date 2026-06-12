use super::super::containers::{column, stack};
use super::virtual_window::virtual_list_window_body;
use crate::application::ViewNode;
use crate::gui::list::{TreeGuideRow, TreeGuideStyle, VirtualListWindow, tree_guide_overlay};

/// Build a vertically virtualized fixed-row tree list with guide overlays.
///
/// This composes [`virtual_list_window_body`] with Radiant's generic tree-guide
/// overlay so host applications can keep tree projection, row identity, and
/// domain actions in the app while Radiant owns the fixed-row virtual scroll and
/// guide-layer composition.
pub fn virtual_tree_list_window<Message: 'static>(
    window: VirtualListWindow,
    row_height: f32,
    guide_rows: &[TreeGuideRow],
    guide_style: TreeGuideStyle,
    mut project: impl FnMut(usize) -> ViewNode<Message>,
    overscan_px: f32,
) -> ViewNode<Message> {
    virtual_list_window_body(
        window,
        row_height,
        |window| {
            let rows = column(
                (window.window_start..window.window_end)
                    .map(|index| project(index).height(row_height).fill_width()),
            )
            .spacing(0.0)
            .fill_width();
            stack([
                rows,
                tree_guide_overlay(
                    guide_rows,
                    window.window_start,
                    window.window_end,
                    guide_style,
                ),
            ])
        },
        overscan_px,
    )
}
