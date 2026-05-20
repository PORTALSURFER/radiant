use crate::{BrowserState, ui};

pub(crate) fn column_context_menu(
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

pub(crate) fn file_context_menu(
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
