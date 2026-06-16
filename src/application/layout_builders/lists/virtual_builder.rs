use super::scroll_update::resolve_virtual_list_window_change;
use super::virtual_window::virtual_list_window_body;
use crate::application::ViewNode;
use crate::application::layout_builders::column;
use crate::gui::list::{VirtualListWindow, VirtualListWindowChange};

/// Builder for fixed-row virtual lists whose logical window is app-owned.
///
/// The builder attaches list-window change messages to the scroll container so
/// applications can keep virtualization state in their normal reducer instead
/// of wiring app-launch scroll lifecycle hooks.
pub struct VirtualListBuilder<Message, Project> {
    window: VirtualListWindow,
    row_height: f32,
    overscan_px: f32,
    project: Project,
    on_window_changed: Option<Box<dyn Fn(VirtualListWindowChange) -> Message + Send + Sync>>,
}

impl<Message, Project> VirtualListBuilder<Message, Project> {
    /// Use the current logical list window.
    pub fn window(mut self, window: VirtualListWindow) -> Self {
        self.window = window;
        self
    }

    /// Use a fixed row height for layout and scroll-to-window projection.
    pub fn row_height(mut self, row_height: f32) -> Self {
        self.row_height = row_height;
        self
    }

    /// Use a pixel overscan distance for runtime virtualization.
    pub fn overscan_px(mut self, overscan_px: f32) -> Self {
        self.overscan_px = overscan_px;
        self
    }

    /// Emit a message when runtime scrolling resolves a different list window.
    pub fn on_window_changed(
        mut self,
        message: impl Fn(VirtualListWindowChange) -> Message + Send + Sync + 'static,
    ) -> Self {
        self.on_window_changed = Some(Box::new(message));
        self
    }
}

impl<Message: 'static, Project> VirtualListBuilder<Message, Project>
where
    Project: FnMut(usize) -> ViewNode<Message>,
{
    /// Build the list view.
    pub fn view(mut self) -> ViewNode<Message> {
        let row_height = self.row_height;
        let window = self.window;
        let overscan_px = self.overscan_px;
        let on_window_changed = self.on_window_changed.take();
        let mut view = virtual_list_window_body(
            window,
            row_height,
            |window| {
                column(
                    (window.window_start..window.window_end)
                        .map(|index| (self.project)(index).height(row_height).fill_width()),
                )
                .spacing(0.0)
                .fill_width()
                .height(row_height.max(0.0) * window.window_len() as f32)
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

/// Build a fixed-row virtual-list builder.
///
/// Use this for large item-indexed lists where application state owns the
/// current [`VirtualListWindow`] and wants scroll changes delivered as ordinary
/// messages through [`VirtualListBuilder::on_window_changed`].
pub fn virtual_list_windowed<Message, Project>(
    project: Project,
) -> VirtualListBuilder<Message, Project> {
    VirtualListBuilder {
        window: VirtualListWindow::default(),
        row_height: 0.0,
        overscan_px: 0.0,
        project,
        on_window_changed: None,
    }
}
