//! Folder tree pane projection for the folder browser example.

use super::*;

pub(super) fn folder_tree(state: &BrowserState) -> ui::View<BrowserMessage> {
    let tree = ui::scroll(
        ui::column(
            state
                .visible_folders()
                .into_iter()
                .map(|folder| folder_row(state, folder))
                .collect::<Vec<_>>(),
        )
        .fill_width()
        .spacing(1.0),
    )
    .fill_height();
    view::panel("Folder Tree", tree)
        .width(state.tree.tree_width)
        .fill_height()
}

fn folder_row(state: &BrowserState, folder: VisibleFolder) -> ui::View<BrowserMessage> {
    let id = folder.id.clone();
    let key = folder.id.clone();
    let editing = state.rename.folder.as_deref() == Some(folder.id.as_str());
    let label = if editing {
        ui::row([
            ui::text_input(state.rename.folder_draft.clone())
                .placeholder("Folder name")
                .message_event(BrowserMessage::FolderRenameInput)
                .key(format!("folder-rename-input-{key}"))
                .fill_width()
                .height(22.0),
            ui::button("OK")
                .primary()
                .message(BrowserMessage::CommitFolderRename)
                .key(format!("folder-rename-ok-{key}"))
                .size(36.0, 22.0),
            ui::close_button()
                .subtle()
                .message(BrowserMessage::CancelFolderRename)
                .key(format!("folder-rename-cancel-{key}"))
                .size(28.0, 22.0),
        ])
        .fill_width()
        .height(22.0)
        .spacing(3.0)
    } else {
        let folder_id = id.clone();
        let mut label = if folder.draggable {
            ui::button(folder.name.clone())
                .secondary_clicks()
                .draggable()
                .mapped(move |event| BrowserMessage::FolderLabel {
                    folder_id: folder_id.clone(),
                    event,
                })
        } else {
            ui::button(folder.name.clone())
                .secondary_clicks()
                .mapped(move |event| BrowserMessage::FolderLabel {
                    folder_id: folder_id.clone(),
                    event,
                })
        }
        .key(format!("folder-label-{key}"))
        .align_text(ui::TextAlign::Left)
        .fill_width()
        .height(22.0);
        if folder.selected || folder.drop_target {
            label = label.primary();
        } else {
            label = label.subtle();
        }
        label
    };
    let toggle_id = folder.id.clone();
    let branch_control = if folder.has_children {
        ui::disclosure_button(folder.expanded)
            .subtle()
            .message(BrowserMessage::ToggleFolder(toggle_id))
            .key(format!("folder-toggle-{id}"))
            .size(24.0, 22.0)
    } else {
        ui::spacer()
            .key(format!("folder-toggle-spacer-{id}"))
            .size(24.0, 22.0)
    };

    ui::row([
        ui::spacer().size((folder.depth as f32) * 4.0, 22.0),
        branch_control,
        label,
    ])
    .key(format!("folder-row-{id}"))
    .style(if folder.drop_target {
        ui::WidgetStyle::subtle(ui::WidgetTone::Accent)
    } else {
        ui::WidgetStyle::default()
    })
    .fill_width()
    .height(TREE_ROW_HEIGHT)
    .spacing(1.0)
    .hoverable()
}

pub(super) fn splitter() -> ui::View<BrowserMessage> {
    ui::column([
        ui::text("").fill_width().fill_height(),
        ui::drag_handle()
            .mapped(BrowserMessage::ResizeTree)
            .key("splitter-handle")
            .size(5.0, 28.0),
        ui::text("").fill_width().fill_height(),
    ])
    .style(ui::WidgetStyle::subtle(ui::WidgetTone::Accent))
    .width(11.0)
    .fill_height()
    .padding(2.0)
    .spacing(4.0)
}
