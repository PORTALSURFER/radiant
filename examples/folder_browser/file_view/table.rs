use std::path::Path;

use super::super::*;

pub(super) fn details_header(state: &BrowserState) -> ui::StateView<BrowserState> {
    details_header_row(
        state
            .visible_file_columns()
            .iter()
            .map(|column| {
                let id = column.id.clone();
                let marker = if state.columns.sort.column_id == column.id {
                    match state.columns.sort.direction {
                        ui::SortDirection::Ascending => " ^",
                        ui::SortDirection::Descending => " v",
                    }
                } else {
                    ""
                };
                let label = format!("{}{}", column.label, marker);
                sized_cell(
                    column,
                    ui::row([
                        ui::button(label)
                            .on_click_or_secondary_at(
                                move |state: &mut BrowserState| state.sort_by(id.clone()),
                                {
                                    let id = column.id.clone();
                                    move |state: &mut BrowserState, position| {
                                        state.open_column_context_menu_at(id.clone(), position);
                                    }
                                },
                            )
                            .key(format!("file-sort-{}", column.id))
                            .fill_width()
                            .height(20.0)
                            .input_only(),
                        {
                            let id = column.id.clone();
                            ui::drag_handle()
                                .on_drag(move |state: &mut BrowserState, message| {
                                    state.resize_file_column(id.clone(), message);
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

pub(super) fn file_details_row(
    state: &BrowserState,
    file: &FileEntry,
) -> ui::StateView<BrowserState> {
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
            ui::WidgetStyle {
                tone: ui::WidgetTone::Accent,
                prominence: ui::WidgetProminence::Subtle,
            }
        } else {
            ui::WidgetStyle::default()
        })
        .hoverable()
}

fn file_name_editor(state: &BrowserState, file: &FileEntry) -> ui::StateView<BrowserState> {
    ui::row([
        ui::text_input(state.rename.file_draft.clone())
            .placeholder("File name")
            .bind_submit(
                |state: &mut BrowserState| &mut state.rename.file_draft,
                BrowserState::commit_file_rename,
            )
            .key(format!("file-rename-input-{}", file.id))
            .fill_width()
            .height(20.0),
        ui::button("OK")
            .primary()
            .on_click(BrowserState::commit_file_rename)
            .key(format!("file-rename-ok-{}", file.id))
            .size(36.0, 20.0),
        ui::button("X")
            .subtle()
            .on_click(BrowserState::cancel_file_rename)
            .key(format!("file-rename-cancel-{}", file.id))
            .size(28.0, 20.0),
    ])
    .fill_width()
    .height(20.0)
    .spacing(3.0)
}

fn file_name_cell(file: &FileEntry) -> ui::StateView<BrowserState> {
    let select_id = file.id.clone();
    let context_id = file.id.clone();
    ui::button(file.name.clone())
        .on_click_or_secondary_at(
            move |state: &mut BrowserState| state.select_file_id(select_id.clone()),
            move |state: &mut BrowserState, position| {
                state.open_file_context_menu_at(context_id.clone(), position);
            },
        )
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

fn file_cell(file_id: String, column_id: String, value: String) -> ui::StateView<BrowserState> {
    let select_id = file_id.clone();
    let context_id = file_id.clone();
    ui::button(value)
        .on_click_or_secondary_at(
            move |state: &mut BrowserState| state.select_file_id(select_id.clone()),
            move |state: &mut BrowserState, position| {
                state.open_file_context_menu_at(context_id.clone(), position);
            },
        )
        .key(format!("{file_id}-{column_id}"))
        .fill_width()
        .height(20.0)
        .input_only()
}

fn sized_cell(
    column: &FileColumn,
    cell: ui::StateView<BrowserState>,
) -> ui::StateView<BrowserState> {
    cell.size(column.width, 20.0)
}

fn compact_details_row(
    children: impl IntoIterator<Item = ui::StateView<BrowserState>>,
) -> ui::StateView<BrowserState> {
    ui::row(children)
        .fill_width()
        .height(22.0)
        .padding_x(8.0)
        .padding_y(1.0)
        .spacing(10.0)
}

fn details_header_row(
    children: impl IntoIterator<Item = ui::StateView<BrowserState>>,
) -> ui::StateView<BrowserState> {
    ui::row(children)
        .style(ui::WidgetStyle {
            tone: ui::WidgetTone::Accent,
            prominence: ui::WidgetProminence::Subtle,
        })
        .fill_width()
        .height(24.0)
        .padding_x(8.0)
        .padding_y(2.0)
        .spacing(10.0)
}
