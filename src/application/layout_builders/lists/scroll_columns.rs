use super::super::containers::column;
use super::super::scroll::scroll;
use crate::application::{ViewNode, empty};
use crate::gui::list::bounded_list_height;
use crate::widgets::WidgetStyle;

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
