use super::model::{DetailsColumn, DetailsRow, DetailsSort};
use crate::{
    application::{
        StateStringCallback, StateView, View, button, column, drag_handle, input_overlay, row,
        scroll, text,
    },
    widgets::{DragHandleMessage, WidgetProminence, WidgetStyle, WidgetTone},
};
use std::sync::Arc;

/// Build a compact details list with clickable sort columns.
pub fn sortable_details_list<State: 'static>(
    columns: impl IntoIterator<Item = DetailsColumn>,
    rows: impl IntoIterator<Item = DetailsRow>,
    sort: Option<DetailsSort>,
    on_sort: impl Fn(&mut State, String) + Send + Sync + 'static,
) -> StateView<State> {
    selectable_sortable_details_list(columns, rows, sort, on_sort, None::<fn(&mut State, String)>)
}

/// Build a compact details list with clickable sort columns and selectable rows.
pub fn selectable_sortable_details_list<State: 'static>(
    columns: impl IntoIterator<Item = DetailsColumn>,
    rows: impl IntoIterator<Item = DetailsRow>,
    sort: Option<DetailsSort>,
    on_sort: impl Fn(&mut State, String) + Send + Sync + 'static,
    on_select: Option<impl Fn(&mut State, String) + Send + Sync + 'static>,
) -> StateView<State> {
    let columns = columns.into_iter().collect::<Vec<_>>();
    let on_sort: StateStringCallback<State> = Arc::new(on_sort);
    let on_select: Option<StateStringCallback<State>> =
        on_select.map(|on_select| Arc::new(on_select) as StateStringCallback<State>);

    column([
        details_header(&columns, sort.as_ref(), Arc::clone(&on_sort)),
        scroll(
            column(
                rows.into_iter()
                    .map(|row| details_row(&columns, row, on_select.as_ref().map(Arc::clone))),
            )
            .fill_width()
            .spacing(1.0),
        )
        .fill_height(),
    ])
    .fill_width()
    .fill_height()
    .spacing(3.0)
}

fn details_header<State: 'static>(
    columns: &[DetailsColumn],
    sort: Option<&DetailsSort>,
    on_sort: StateStringCallback<State>,
) -> StateView<State> {
    compact_details_row(columns.iter().map(|column| {
        let id = column.id.clone();
        let on_sort = Arc::clone(&on_sort);
        let marker = sort
            .filter(|sort| sort.column_id == column.id)
            .map(|sort| sort.direction.marker())
            .unwrap_or("");
        let label = format!("{}{}", column.label, marker);
        sized_cell(
            column,
            button(label)
                .on_click(move |state: &mut State| on_sort(state, id.clone()))
                .key(format!("details-sort-{}", column.id))
                .subtle(),
        )
    }))
}

fn details_row<State: 'static>(
    columns: &[DetailsColumn],
    row_data: DetailsRow,
    on_select: Option<StateStringCallback<State>>,
) -> StateView<State> {
    let row_id = row_data.id.clone();
    let selectable = on_select.is_some();
    let mut row = compact_details_row(columns.iter().enumerate().map(|(index, column)| {
        let value = row_data.cells.get(index).cloned().unwrap_or_default();
        let cell = if let Some(on_select) = on_select.as_ref() {
            let row_id = row_id.clone();
            let on_select = Arc::clone(on_select);
            let mut button = button(value)
                .on_click(move |state: &mut State| on_select(state, row_id.clone()))
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
        sized_cell(column, cell)
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

fn sized_cell<State: 'static>(column: &DetailsColumn, cell: StateView<State>) -> StateView<State> {
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
        Some(width) => cell.size(width, 20.0),
        None => cell.fill_width().height(20.0),
    }
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
    let key = key.into();
    let label = label.into();
    row([
        input_overlay(
            text(label.clone())
                .key(format!("{key}-label"))
                .align_text(crate::widgets::TextAlign::Left)
                .fill_width()
                .height(20.0)
                .truncate(),
            button(label)
                .click_or_drag(sort_message, drag_message)
                .key(format!("{key}-sort-drag"))
                .fill_width()
                .height(20.0),
        )
        .fill_width()
        .height(20.0),
        drag_handle()
            .mapped(resize_message)
            .key(format!("{key}-resize"))
            .size(4.0, 20.0),
    ])
    .key(key)
    .width(width)
    .height(20.0)
    .spacing(1.0)
}
