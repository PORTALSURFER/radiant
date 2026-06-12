use super::model::{DetailsColumn, DetailsRow, DetailsSort};
use crate::{
    application::{
        LayerHorizontalAnchor, LayerVerticalAnchor, View, anchored_layer, button, column,
        drag_handle, input_underlay, row, scroll, text,
    },
    layout::Vector2,
    widgets::{DragHandleMessage, WidgetId, WidgetProminence, WidgetStyle, WidgetTone},
};
use std::sync::Arc;

/// Stable widget ids for a compact resizable details header cell.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct CompactDetailsHeaderCellIds {
    /// Widget id for the sort/reorder input surface.
    pub sort_drag: Option<WidgetId>,
    /// Widget id for the trailing resize handle.
    pub resize: Option<WidgetId>,
}

/// Named inputs for a compact details-list cell with a fixed-size anchored child.
pub struct CompactDetailsAnchoredCellParts<Message> {
    child: View<Message>,
    width: Option<f32>,
    size: Vector2,
    horizontal: LayerHorizontalAnchor,
    vertical: LayerVerticalAnchor,
    inset_x: f32,
    inset_y: f32,
}

impl<Message> CompactDetailsAnchoredCellParts<Message> {
    /// Create compact anchored-cell inputs with centered placement and no inset.
    pub fn new(child: View<Message>, size: Vector2) -> Self {
        Self {
            child,
            width: None,
            size,
            horizontal: LayerHorizontalAnchor::Center,
            vertical: LayerVerticalAnchor::Center,
            inset_x: 0.0,
            inset_y: 0.0,
        }
    }

    /// Use a fixed width, or fill the remaining details-row width when `None`.
    pub fn width(mut self, width: Option<f32>) -> Self {
        self.width = width;
        self
    }

    /// Place the fixed-size child along the compact cell's horizontal axis.
    pub fn horizontal(mut self, horizontal: LayerHorizontalAnchor) -> Self {
        self.horizontal = horizontal;
        self
    }

    /// Place the fixed-size child along the compact cell's vertical axis.
    pub fn vertical(mut self, vertical: LayerVerticalAnchor) -> Self {
        self.vertical = vertical;
        self
    }

    /// Offset the child from its anchor by the given x/y inset.
    pub fn inset(mut self, x: f32, y: f32) -> Self {
        self.inset_x = x;
        self.inset_y = y;
        self
    }
}

impl CompactDetailsHeaderCellIds {
    /// Build explicit ids for the sort/reorder and resize surfaces.
    pub const fn new(sort_drag: Option<WidgetId>, resize: Option<WidgetId>) -> Self {
        Self { sort_drag, resize }
    }
}

/// Build a compact details list that emits sort messages.
pub fn message_sortable_details_list<Message: 'static>(
    columns: impl IntoIterator<Item = DetailsColumn>,
    rows: impl IntoIterator<Item = DetailsRow>,
    sort: Option<DetailsSort>,
    sort_message: impl Fn(String) -> Message + Send + Sync + 'static,
) -> View<Message>
where
    Message: Clone + Send + Sync + 'static,
{
    message_selectable_sortable_details_list(
        columns,
        rows,
        sort,
        sort_message,
        None::<fn(String) -> Message>,
    )
}

/// Build a compact details list that emits sort and row-selection messages.
pub fn message_selectable_sortable_details_list<Message: 'static>(
    columns: impl IntoIterator<Item = DetailsColumn>,
    rows: impl IntoIterator<Item = DetailsRow>,
    sort: Option<DetailsSort>,
    sort_message: impl Fn(String) -> Message + Send + Sync + 'static,
    select_message: Option<impl Fn(String) -> Message + Send + Sync + 'static>,
) -> View<Message>
where
    Message: Clone + Send + Sync + 'static,
{
    let columns = columns.into_iter().collect::<Vec<_>>();
    let sort_message = Arc::new(sort_message) as Arc<dyn Fn(String) -> Message + Send + Sync>;
    let select_message = select_message.map(|select_message| {
        Arc::new(select_message) as Arc<dyn Fn(String) -> Message + Send + Sync>
    });

    column([
        message_details_header(&columns, sort.as_ref(), Arc::clone(&sort_message)),
        scroll(
            column(rows.into_iter().map(|row| {
                message_details_row(&columns, row, select_message.as_ref().map(Arc::clone))
            }))
            .fill_width()
            .spacing(1.0),
        )
        .fill_height(),
    ])
    .fill_width()
    .fill_height()
    .spacing(3.0)
}

fn message_details_header<Message>(
    columns: &[DetailsColumn],
    sort: Option<&DetailsSort>,
    sort_message: Arc<dyn Fn(String) -> Message + Send + Sync>,
) -> View<Message>
where
    Message: Clone + Send + Sync + 'static,
{
    compact_details_row(columns.iter().map(|column| {
        let id = column.id.clone();
        let marker = sort
            .filter(|sort| sort.column_id == column.id)
            .map(|sort| sort.direction.marker())
            .unwrap_or("");
        let label = format!("{}{}", column.label, marker);
        sized_message_cell(
            column,
            button(label)
                .message(sort_message(id))
                .key(format!("details-sort-{}", column.id))
                .subtle(),
        )
    }))
}

fn message_details_row<Message>(
    columns: &[DetailsColumn],
    row_data: DetailsRow,
    select_message: Option<Arc<dyn Fn(String) -> Message + Send + Sync>>,
) -> View<Message>
where
    Message: Clone + Send + Sync + 'static,
{
    let row_id = row_data.id.clone();
    let selectable = select_message.is_some();
    let mut row = compact_details_row(columns.iter().enumerate().map(|(index, column)| {
        let value = row_data.cells.get(index).cloned().unwrap_or_default();
        let cell = if let Some(select_message) = select_message.as_ref() {
            let mut button = button(value)
                .message(select_message(row_id.clone()))
                .key(format!("{}-{index}", row_data.id))
                .fill_width()
                .height(20.0);
            if row_data.selected {
                button = button.primary();
            } else {
                button = button.subtle();
            };
            button
        } else {
            text(value).key(format!("{}-{index}", row_data.id))
        };
        sized_message_cell(column, cell)
    }))
    .key(format!("details-row-{}", row_data.id))
    .style(if row_data.selected {
        WidgetStyle::new(WidgetTone::Accent, WidgetProminence::Subtle)
    } else {
        WidgetStyle::default()
    })
    .hoverable();
    if row_data.selected && !selectable {
        row = row.primary();
    }
    row
}

fn sized_message_cell<Message: 'static>(
    column: &DetailsColumn,
    cell: View<Message>,
) -> View<Message> {
    compact_details_cell(cell, column.width)
}

/// Build a compact horizontal details-row layout.
///
/// This is the same dense row frame used by Radiant's built-in details list:
/// fixed 22px row height, small vertical padding, left/right chrome, and
/// compact cell spacing. Host apps can reuse it when they need custom row
/// content but still want details-list density and alignment.
pub fn compact_details_row<Message>(
    children: impl IntoIterator<Item = View<Message>>,
) -> View<Message> {
    row(children)
        .fill_width()
        .height(22.0)
        .padding_x(8.0)
        .padding_y(1.0)
        .spacing(10.0)
}

/// Size one compact details-list cell.
///
/// This matches the cell sizing used by Radiant's built-in details lists:
/// fixed-width columns get a 20px-tall fixed cell, while flexible columns fill
/// the remaining row width at the same height. Host apps can use it for custom
/// cell content without repeating details-list sizing policy.
pub fn compact_details_cell<Message>(cell: View<Message>, width: Option<f32>) -> View<Message> {
    match width {
        Some(width) => cell.width(width).height(20.0),
        None => cell.fill_width().height(20.0),
    }
}

/// Build a compact details-list cell with a fixed-size anchored child.
///
/// This preserves the standard compact cell sizing policy while letting hosts
/// place badges, status markers, compact actions, or other fixed-size content
/// inside the cell without rebuilding the full-size anchored layer and then
/// applying details-cell sizing separately.
pub fn compact_details_anchored_cell_from_parts<Message>(
    parts: CompactDetailsAnchoredCellParts<Message>,
) -> View<Message>
where
    Message: 'static,
{
    let CompactDetailsAnchoredCellParts {
        child,
        width,
        size,
        horizontal,
        vertical,
        inset_x,
        inset_y,
    } = parts;
    compact_details_cell(
        anchored_layer(child, size, horizontal, vertical, inset_x, inset_y),
        width,
    )
}

/// Build a compact details-list header row.
///
/// This matches Radiant's dense details-list header chrome: fixed 24px height,
/// accent subtle background, small vertical padding, left/right chrome, and
/// compact cell spacing. Host apps can use it when they need custom sortable,
/// resizable, or reorderable header cells but should not repeat details-list
/// header styling.
pub fn compact_details_header_row<Message>(
    children: impl IntoIterator<Item = View<Message>>,
) -> View<Message> {
    row(children)
        .style(WidgetStyle::new(
            WidgetTone::Accent,
            WidgetProminence::Subtle,
        ))
        .fill_width()
        .height(24.0)
        .padding_x(8.0)
        .padding_y(2.0)
        .spacing(10.0)
}

/// Build a compact sortable, reorderable, and resizable details-list header cell.
///
/// This provides the standard composition used by dense details-list headers:
/// visible truncated label text, a transparent click-or-drag input layer for
/// sort and column reorder gestures, and a trailing resize handle.
pub fn compact_resizable_details_header_cell<Message>(
    key: impl Into<String>,
    label: impl Into<String>,
    width: f32,
    sort_message: Message,
    drag_message: impl Fn(DragHandleMessage) -> Message + Send + Sync + 'static,
    resize_message: impl Fn(DragHandleMessage) -> Message + Send + Sync + 'static,
) -> View<Message>
where
    Message: Clone + Send + Sync + 'static,
{
    compact_resizable_details_header_cell_with_ids(
        key,
        label,
        width,
        CompactDetailsHeaderCellIds::default(),
        sort_message,
        drag_message,
        resize_message,
    )
}

/// Build a compact sortable, reorderable, and resizable details-list header
/// cell with explicit stable widget ids for the interactive surfaces.
#[allow(
    clippy::too_many_arguments,
    reason = "compatibility API with two optional stable widget ids"
)]
pub fn compact_resizable_details_header_cell_with_ids<Message>(
    key: impl Into<String>,
    label: impl Into<String>,
    width: f32,
    ids: CompactDetailsHeaderCellIds,
    sort_message: Message,
    drag_message: impl Fn(DragHandleMessage) -> Message + Send + Sync + 'static,
    resize_message: impl Fn(DragHandleMessage) -> Message + Send + Sync + 'static,
) -> View<Message>
where
    Message: Clone + Send + Sync + 'static,
{
    let key = key.into();
    let label = label.into();
    let mut sort_drag = button("")
        .hover_chrome_only()
        .click_or_drag(sort_message, drag_message)
        .key(format!("{key}-sort-drag"))
        .fill_width()
        .height(20.0);
    if let Some(id) = ids.sort_drag {
        sort_drag = sort_drag.id(id);
    }
    let mut resize = drag_handle()
        .mapped(resize_message)
        .key(format!("{key}-resize"))
        .size(4.0, 20.0);
    if let Some(id) = ids.resize {
        resize = resize.id(id);
    }
    row([
        input_underlay(
            text(label.clone())
                .key(format!("{key}-label"))
                .align_text(crate::widgets::TextAlign::Left)
                .fill_width()
                .height(20.0)
                .truncate(),
            sort_drag,
        )
        .fill_width()
        .height(20.0),
        resize,
    ])
    .key(key)
    .width(width)
    .height(20.0)
    .spacing(1.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::IntoView;

    #[test]
    fn compact_details_anchored_cell_preserves_cell_size_and_places_child() {
        let frame = compact_details_anchored_cell_from_parts::<()>(
            CompactDetailsAnchoredCellParts::new(
                text("K").style(WidgetStyle::subtle(WidgetTone::Warning)),
                Vector2::new(24.0, 14.0),
            )
            .width(Some(64.0))
            .horizontal(LayerHorizontalAnchor::End)
            .vertical(LayerVerticalAnchor::Start)
            .inset(2.0, 3.0),
        )
        .view_frame_at_size_with_default_theme(Vector2::new(64.0, 20.0));

        let text_rect = frame
            .paint_plan
            .first_text_run("K")
            .expect("anchored child text should paint")
            .rect;

        assert!(text_rect.min.x >= 38.0, "{text_rect:?}");
        assert!(text_rect.min.y >= 3.0, "{text_rect:?}");
        assert!(text_rect.max.x <= 64.0, "{text_rect:?}");
        assert!(text_rect.max.y <= 20.0, "{text_rect:?}");
    }
}
