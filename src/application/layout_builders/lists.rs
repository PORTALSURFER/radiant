//! List-oriented layout builders.

use super::containers::{column, row, row_key, stack};
use super::scroll::{scroll, virtual_scroll};
use crate::application::{ViewNode, empty, spacer};
use crate::gui::list::{
    TreeGuideRow, TreeGuideStyle, VirtualListWindow, VirtualListWindowChange,
    VirtualListWindowRequest, bounded_list_height, resolve_virtual_list_window, tree_guide_overlay,
    virtual_list_view_start_for_scroll_offset,
};
use crate::runtime::ScrollUpdate;
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
            view = view.on_scroll_update(move |update| {
                message(resolve_virtual_list_window_change(
                    update.offset.y,
                    row_height,
                    window,
                    overscan_px,
                ))
            });
        }
        view
    }
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

fn resolve_virtual_list_window_change(
    offset_y: f32,
    row_height: f32,
    current: VirtualListWindow,
    overscan_px: f32,
) -> VirtualListWindowChange {
    let row_height = row_height.max(1.0);
    let requested_start =
        virtual_list_view_start_for_scroll_offset(offset_y, row_height, current.total_items);
    let overscan = (overscan_px.max(0.0) / row_height).ceil() as usize;
    let window = resolve_virtual_list_window(VirtualListWindowRequest {
        total_items: current.total_items,
        viewport_len: current.viewport_len(),
        requested_start,
        overscan,
        focused_index: None,
        previous_start: None,
        guard_band: 0,
    });
    VirtualListWindowChange {
        offset_y,
        row_height,
        window,
    }
}

/// Resolve a fixed-row virtual-list window change from a runtime scroll update.
pub fn virtual_list_window_change_for_scroll(
    update: ScrollUpdate,
    row_height: f32,
    current: VirtualListWindow,
    overscan_rows: usize,
) -> VirtualListWindowChange {
    resolve_virtual_list_window_change(
        update.offset.y,
        row_height,
        current,
        row_height.max(0.0) * overscan_rows as f32,
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
