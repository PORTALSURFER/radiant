use super::super::containers::{column, stack};
use super::scroll_update::resolve_virtual_list_window_change;
use super::virtual_window::virtual_list_window_body;
use crate::application::ViewNode;
use crate::gui::list::{
    TreeGuideOverlayStyle, TreeGuideRow, VirtualListWindow, VirtualListWindowChange,
    tree_guide_overlay,
};

/// Builder for fixed-row virtual tree lists whose logical window is app-owned.
///
/// This is the tree-guide companion to `virtual_list_windowed(...)`: the host
/// owns tree rows and reducer state, while Radiant owns the fixed-row spacer,
/// guide overlay composition, and scroll-to-window change mapping.
pub struct VirtualTreeListBuilder<'a, Message, Project> {
    window: VirtualListWindow,
    row_height: f32,
    guide_rows: &'a [TreeGuideRow],
    guide_style: TreeGuideOverlayStyle,
    project: Project,
    overscan_px: f32,
    on_window_changed: Option<Box<dyn Fn(VirtualListWindowChange) -> Message + Send + Sync>>,
}

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
    guide_style: impl Into<TreeGuideOverlayStyle>,
    mut project: impl FnMut(usize) -> ViewNode<Message>,
    overscan_px: f32,
) -> ViewNode<Message> {
    virtual_tree_list_windowed(window, row_height, guide_rows, guide_style, move |index| {
        project(index)
    })
    .overscan_px(overscan_px)
    .view()
}

impl<'a, Message, Project> VirtualTreeListBuilder<'a, Message, Project> {
    /// Use a pixel overscan distance for runtime virtualization.
    pub fn overscan_px(mut self, overscan_px: f32) -> Self {
        self.overscan_px = overscan_px;
        self
    }

    /// Emit a message when runtime scrolling resolves a different tree window.
    pub fn on_window_changed(
        mut self,
        message: impl Fn(VirtualListWindowChange) -> Message + Send + Sync + 'static,
    ) -> Self {
        self.on_window_changed = Some(Box::new(message));
        self
    }
}

impl<Message: 'static, Project> VirtualTreeListBuilder<'_, Message, Project>
where
    Project: FnMut(usize) -> ViewNode<Message>,
{
    /// Build the tree-list view.
    pub fn view(mut self) -> ViewNode<Message> {
        let window = self.window;
        let row_height = self.row_height;
        let guide_rows = self.guide_rows;
        let guide_style = self.guide_style;
        let overscan_px = self.overscan_px;
        let on_window_changed = self.on_window_changed.take();
        let mut project = self.project;
        let mut view = virtual_list_window_body(
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
        );

        if let Some(message) = on_window_changed {
            view = view.on_scroll_update_opt(move |update| {
                let change = resolve_virtual_list_window_change(
                    update.offset.y,
                    row_height,
                    update.viewport.y,
                    window,
                    overscan_px,
                );
                (change.window != window).then(|| message(change))
            });
        }
        view
    }
}

/// Build a virtual tree-list builder with reducer-friendly window-change messages.
pub fn virtual_tree_list_windowed<'a, Message, Project>(
    window: VirtualListWindow,
    row_height: f32,
    guide_rows: &'a [TreeGuideRow],
    guide_style: impl Into<TreeGuideOverlayStyle>,
    project: Project,
) -> VirtualTreeListBuilder<'a, Message, Project> {
    VirtualTreeListBuilder {
        window,
        row_height,
        guide_rows,
        guide_style: guide_style.into(),
        project,
        overscan_px: 0.0,
        on_window_changed: None,
    }
}
