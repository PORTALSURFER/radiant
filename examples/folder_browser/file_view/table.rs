use std::path::Path;

use super::super::*;
use radiant::application::{compact_details_header_row, compact_details_row};

pub(super) fn details_header(state: &BrowserState) -> ui::View<BrowserMessage> {
    compact_details_header_row(
        state
            .visible_file_columns()
            .iter()
            .map(|column| {
                let id = column.id.clone();
                let label = state
                    .columns
                    .sort
                    .label_for(column.id.as_str(), column.label.as_str());
                sized_cell(
                    column,
                    ui::row([
                        ui::button(label)
                            .secondary_clicks()
                            .mapped(move |event| BrowserMessage::ColumnHeader {
                                column_id: id.clone(),
                                event,
                            })
                            .key(format!("file-sort-{}", column.id))
                            .fill_width()
                            .height(20.0)
                            .input_only(),
                        {
                            let id = column.id.clone();
                            ui::drag_handle()
                                .mapped(move |event| BrowserMessage::ResizeFileColumn {
                                    column_id: id.clone(),
                                    event,
                                })
                                .key(format!("file-column-resize-{}", column.id))
                                .size(4.0, 20.0)
                        },
                    ])
                    .fill_width()
                    .height(20.0)
                    .spacing(1.0),
                )
            })
            .collect::<Vec<_>>(),
    )
}

pub(super) fn file_details_row(state: &BrowserState, file: &FileEntry) -> ui::View<BrowserMessage> {
    let selected = state.selection.selected_file.as_deref() == Some(file.id.as_str());
    let editing = state.rename.file.as_deref() == Some(file.id.as_str());
    let cells = state
        .visible_file_columns()
        .into_iter()
        .map(|column| {
            let cell = if column.id == "name" && editing {
                file_name_editor(state, file)
            } else if column.id == "name" {
                file_name_cell(file)
            } else {
                file_cell(
                    file.id.clone(),
                    column.id.clone(),
                    file_column_value(file, column.id.as_str()),
                )
            };
            sized_cell(column, cell)
        })
        .collect::<Vec<_>>();
    compact_details_row(cells)
        .key(format!("file-row-{}", file.id))
        .style(if selected {
            ui::WidgetStyle::subtle(ui::WidgetTone::Accent)
        } else {
            ui::WidgetStyle::default()
        })
        .hoverable()
}

fn file_name_editor(state: &BrowserState, file: &FileEntry) -> ui::View<BrowserMessage> {
    ui::row([
        ui::text_input(state.rename.file_draft.clone())
            .placeholder("File name")
            .message_event(BrowserMessage::FileRenameInput)
            .key(format!("file-rename-input-{}", file.id))
            .fill_width()
            .height(20.0),
        ui::button("OK")
            .primary()
            .message(BrowserMessage::CommitFileRename)
            .key(format!("file-rename-ok-{}", file.id))
            .size(36.0, 20.0),
        ui::button("X")
            .subtle()
            .message(BrowserMessage::CancelFileRename)
            .key(format!("file-rename-cancel-{}", file.id))
            .size(28.0, 20.0),
    ])
    .fill_width()
    .height(20.0)
    .spacing(3.0)
}

fn file_name_cell(file: &FileEntry) -> ui::View<BrowserMessage> {
    let file_id = file.id.clone();
    ui::button(file.name.clone())
        .secondary_clicks()
        .mapped(move |event| BrowserMessage::FileButton {
            file_id: file_id.clone(),
            event,
        })
        .key(format!("file-name-{}", file.id))
        .fill_width()
        .height(20.0)
        .input_only()
}

fn file_column_value(file: &FileEntry, column_id: &str) -> String {
    match column_id {
        "size" => file.size.clone(),
        "kind" => file.kind.clone(),
        "modified" => file.modified.clone(),
        "extension" => file_extension(Path::new(&file.id)),
        "path" => file.id.clone(),
        _ => file.name.clone(),
    }
}

fn file_cell(file_id: String, column_id: String, value: String) -> ui::View<BrowserMessage> {
    let button_file_id = file_id.clone();
    ui::button(value)
        .secondary_clicks()
        .mapped(move |event| BrowserMessage::FileButton {
            file_id: button_file_id.clone(),
            event,
        })
        .key(format!("{file_id}-{column_id}"))
        .fill_width()
        .height(20.0)
        .input_only()
}

fn sized_cell(column: &FileColumn, cell: ui::View<BrowserMessage>) -> ui::View<BrowserMessage> {
    cell.size(column.width, 20.0)
}
