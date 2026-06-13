use super::compact::{compact_details_cell, compact_details_row};
use crate::{
    application::{View, button, column, scroll, text},
    widgets::{WidgetProminence, WidgetStyle, WidgetTone},
};
use std::sync::Arc;

use super::super::model::{DetailsColumn, DetailsRow, DetailsSort};

/// Build a compact details list that emits sort messages.
pub fn message_sortable_details_list<Message>(
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
pub fn message_selectable_sortable_details_list<Message>(
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
