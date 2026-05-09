//! Folder tree pane projection for the folder browser example.

use super::*;

pub(super) fn folder_tree(state: &BrowserState) -> ui::StateView<BrowserState> {
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
        .width(state.tree_width)
        .fill_height()
}

fn folder_row(state: &BrowserState, folder: VisibleFolder) -> ui::StateView<BrowserState> {
    let id = folder.id.clone();
    let key = folder.id.clone();
    let toggle_id = folder.id.clone();
    let drag_id = folder.id.clone();
    let editing = state.rename_folder.as_deref() == Some(folder.id.as_str());
    let expander = if folder.expanded { "[-]" } else { "[+]" };
    let label = if editing {
        ui::row([
            ui::text_input(state.rename_draft.clone())
                .placeholder("Folder name")
                .bind_submit(
                    |state: &mut BrowserState| &mut state.rename_draft,
                    BrowserState::commit_rename,
                )
                .key(format!("folder-rename-input-{key}"))
                .fill_width()
                .height(22.0),
            ui::button("OK")
                .primary()
                .on_click(BrowserState::commit_rename)
                .key(format!("folder-rename-ok-{key}"))
                .size(36.0, 22.0),
            ui::button("X")
                .subtle()
                .on_click(BrowserState::cancel_folder_rename)
                .key(format!("folder-rename-cancel-{key}"))
                .size(28.0, 22.0),
        ])
        .fill_width()
        .height(22.0)
        .spacing(3.0)
    } else {
        let select_id = id.clone();
        let context_id = id.clone();
        let drag_id = drag_id.clone();
        let mut label = if folder.draggable {
            ui::button(folder.name).on_click_secondary_at_or_drag(
                move |state: &mut BrowserState| state.activate_folder(select_id.clone()),
                move |state: &mut BrowserState, position| {
                    state.open_context_menu_at(context_id.clone(), position);
                },
                move |state: &mut BrowserState, message| {
                    state.handle_folder_drag(drag_id.clone(), message);
                },
            )
        } else {
            ui::button(folder.name).on_click_or_secondary_at(
                move |state: &mut BrowserState| state.activate_folder(select_id.clone()),
                move |state: &mut BrowserState, position| {
                    state.open_context_menu_at(context_id.clone(), position);
                },
            )
        }
        .key(format!("folder-label-{key}"))
        .fill_width()
        .height(22.0);
        if folder.selected || folder.drop_target {
            label = label.primary();
        } else {
            label = label.subtle();
        }
        label
    };

    ui::row([
        ui::text("").size((folder.depth as f32) * 12.0, 22.0),
        if folder.has_children {
            ui::button(expander)
                .on_click(move |state: &mut BrowserState| state.toggle_folder(toggle_id.clone()))
                .key(format!("folder-toggle-{id}"))
                .size(32.0, 22.0)
                .subtle()
        } else {
            ui::text("")
                .key(format!("folder-toggle-spacer-{id}"))
                .size(32.0, 22.0)
        },
        label,
    ])
    .key(format!("folder-row-{id}"))
    .style(if folder.drop_target {
        ui::WidgetStyle {
            tone: ui::WidgetTone::Accent,
            prominence: ui::WidgetProminence::Subtle,
        }
    } else {
        ui::WidgetStyle::default()
    })
    .fill_width()
    .height(TREE_ROW_HEIGHT)
    .spacing(1.0)
    .hoverable()
}

pub(super) fn splitter() -> ui::StateView<BrowserState> {
    ui::column([
        ui::text("").fill_width().fill_height(),
        ui::drag_handle()
            .on_drag(|state: &mut BrowserState, message| state.resize_tree(message))
            .key("splitter-handle")
            .size(5.0, 28.0),
        ui::text("").fill_width().fill_height(),
    ])
    .style(ui::WidgetStyle {
        tone: ui::WidgetTone::Accent,
        prominence: ui::WidgetProminence::Subtle,
    })
    .width(11.0)
    .fill_height()
    .padding(2.0)
    .spacing(4.0)
}
