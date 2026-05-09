//! File details pane projection for the folder browser example.

use std::path::Path;

use super::*;

pub(super) fn file_view(state: &BrowserState) -> ui::StateView<BrowserState> {
    let folder = state.selected_folder();
    let file_rows = ui::scroll(
        ui::column(
            state
                .sorted_files()
                .into_iter()
                .map(|file| file_details_row(state, file))
                .collect::<Vec<_>>(),
        )
        .fill_width()
        .spacing(1.0),
    )
    .fill_height();
    let details = ui::column([details_header(state), file_rows])
        .fill_width()
        .fill_height()
        .spacing(3.0);
    let file_actions = ui::row([
        ui::text("Files").fill_width().height(28.0),
        ui::button("New File")
            .primary()
            .on_click(BrowserState::create_file_in_selected_folder)
            .size(104.0, 28.0),
    ])
    .fill_width()
    .height(32.0)
    .spacing(8.0);
    let content = ui::column([file_actions, details])
        .fill_width()
        .fill_height()
        .spacing(6.0);
    view::panel(folder.name.clone(), content)
        .fill_width()
        .fill_height()
}

fn details_header(state: &BrowserState) -> ui::StateView<BrowserState> {
    details_header_row(
        state
            .visible_file_columns()
            .iter()
            .map(|column| {
                let id = column.id.clone();
                let marker = if state.sort.column_id == column.id {
                    match state.sort.direction {
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

pub(super) fn column_context_menu(
    state: &BrowserState,
    column_id: &str,
) -> ui::StateView<BrowserState> {
    let column_name = state
        .file_columns
        .iter()
        .find(|column| column.id == column_id)
        .map(|column| column.label.clone())
        .unwrap_or_else(|| String::from("columns"));
    let mut actions = vec![
        ui::text(format!("Columns from {column_name}"))
            .fill_width()
            .height(22.0),
    ];
    actions.extend(state.file_columns.iter().map(|column| {
        let id = column.id.clone();
        let marker = if column.visible { "[x]" } else { "[ ]" };
        let label = if column.id == "name" {
            format!("{marker} {} locked", column.label)
        } else {
            format!("{marker} {}", column.label)
        };
        ui::button(label)
            .subtle()
            .on_click(move |state: &mut BrowserState| state.toggle_file_column(id.clone()))
            .fill_width()
            .height(26.0)
    }));
    actions.push(
        ui::button("Reset Defaults")
            .primary()
            .on_click(BrowserState::reset_file_columns)
            .fill_width()
            .height(28.0),
    );
    actions.push(
        ui::button("Close")
            .subtle()
            .on_click(BrowserState::close_column_context_menu)
            .fill_width()
            .height(28.0),
    );
    ui::column(actions)
        .style(ui::WidgetStyle {
            tone: ui::WidgetTone::Accent,
            prominence: ui::WidgetProminence::Strong,
        })
        .width(210.0)
        .height(250.0)
        .padding(8.0)
        .spacing(5.0)
}

pub(super) fn file_context_menu(
    state: &BrowserState,
    file_id: &str,
) -> ui::StateView<BrowserState> {
    let file_name = state
        .selected_folder()
        .files
        .iter()
        .find(|file| file.id == file_id)
        .map(|file| file.name.clone())
        .unwrap_or_else(|| String::from("item"));
    ui::column([
        ui::text(format!("Actions for {file_name}"))
            .fill_width()
            .height(22.0),
        ui::button("Rename")
            .primary()
            .on_click(BrowserState::begin_file_rename_from_context)
            .fill_width()
            .height(28.0),
        ui::button("Delete")
            .subtle()
            .on_click(BrowserState::delete_file_from_context)
            .fill_width()
            .height(28.0),
        ui::button("Cancel")
            .subtle()
            .on_click(BrowserState::close_file_context_menu)
            .fill_width()
            .height(28.0),
    ])
    .style(ui::WidgetStyle {
        tone: ui::WidgetTone::Accent,
        prominence: ui::WidgetProminence::Strong,
    })
    .width(190.0)
    .height(126.0)
    .padding(8.0)
    .spacing(6.0)
}

fn file_details_row(state: &BrowserState, file: &FileEntry) -> ui::StateView<BrowserState> {
    let selected = state.selected_file.as_deref() == Some(file.id.as_str());
    let editing = state.rename_file.as_deref() == Some(file.id.as_str());
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
        ui::text_input(state.file_rename_draft.clone())
            .placeholder("File name")
            .bind_submit(
                |state: &mut BrowserState| &mut state.file_rename_draft,
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
