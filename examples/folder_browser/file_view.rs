//! File details pane projection for the folder browser example.

#[path = "file_view/table.rs"]
mod table;

use super::*;
use table::{details_header, file_details_row};

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
