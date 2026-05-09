use super::file_view::{column_context_menu, file_context_menu, file_view};
use super::tree::{folder_tree, splitter};
use super::*;

pub(super) fn project_surface(state: &mut BrowserState) -> ui::StateView<BrowserState> {
    let page = ui::column([
        header(state),
        ui::row([folder_tree(state), splitter(), file_view(state)])
            .style(ui::WidgetStyle::default())
            .fill_width()
            .fill_height()
            .padding(8.0)
            .spacing(8.0),
    ])
    .fill_width()
    .fill_height()
    .padding(12.0)
    .spacing(8.0);
    if has_context_menu(state) {
        ui::stack([page, context_menu_overlay(state)])
            .fill_width()
            .fill_height()
    } else {
        page
    }
}

fn has_context_menu(state: &BrowserState) -> bool {
    state.context_folder.is_some() || state.context_file.is_some() || state.context_column.is_some()
}

fn header(state: &BrowserState) -> ui::StateView<BrowserState> {
    ui::row([
        ui::text("Folder browser").size(170.0, 24.0),
        ui::text(format!(
            "{} items in {}",
            state.selected_folder().files.len(),
            state.selected_folder().name
        ))
        .size(220.0, 24.0),
        ui::text(state.selected_file_label()).size(180.0, 24.0),
        ui::text(state.status.clone()).fill_width().height(24.0),
    ])
    .fill_width()
    .spacing(12.0)
}

fn context_menu_overlay(state: &BrowserState) -> ui::StateView<BrowserState> {
    let (width, height, menu) = if let Some(folder_id) = state.context_folder.as_ref() {
        (
            FOLDER_MENU_WIDTH,
            FOLDER_MENU_HEIGHT,
            context_menu(state, folder_id),
        )
    } else if let Some(column_id) = state.context_column.as_ref() {
        (
            COLUMN_MENU_WIDTH,
            COLUMN_MENU_HEIGHT,
            column_context_menu(state, column_id),
        )
    } else if let Some(file_id) = state.context_file.as_ref() {
        (
            FILE_MENU_WIDTH,
            FILE_MENU_HEIGHT,
            file_context_menu(state, file_id),
        )
    } else {
        (0.0, 0.0, ui::text(""))
    };
    let (left, top) = anchored_context_menu_position(state.context_position, width, height);
    ui::column([
        ui::text("").fill_width().height(top),
        ui::row([
            ui::text("").size(left, 1.0),
            menu,
            ui::text("").fill_width().height(1.0),
        ])
        .fill_width()
        .height(height),
        ui::text("").fill_width().fill_height(),
    ])
    .fill_width()
    .fill_height()
    .key("context-menu-overlay")
}

fn context_menu(state: &BrowserState, folder_id: &str) -> ui::StateView<BrowserState> {
    let folder_name = state
        .find_folder(folder_id)
        .map(|folder| folder.name.clone())
        .unwrap_or_else(|| String::from("folder"));
    ui::column([
        ui::text(format!("Actions for {folder_name}"))
            .fill_width()
            .height(22.0),
        ui::button("Rename")
            .primary()
            .on_click(BrowserState::begin_rename_from_context)
            .fill_width()
            .height(28.0),
        ui::button("New Folder")
            .subtle()
            .on_click(BrowserState::create_folder_from_context)
            .fill_width()
            .height(28.0),
        ui::button("Cancel")
            .subtle()
            .on_click(BrowserState::close_context_menu)
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

pub(super) fn panel(
    title: impl Into<String>,
    content: ui::StateView<BrowserState>,
) -> ui::StateView<BrowserState> {
    ui::column([ui::text(title).fill_width().height(22.0), content])
        .style(ui::WidgetStyle::default())
        .fill_width()
        .fill_height()
        .padding(8.0)
        .spacing(6.0)
}
