use super::*;

pub(super) fn action_from_pointer_with_motion(
    layout: &ShellLayout,
    model: &AppModel,
    motion_model: Option<&NativeMotionModel>,
    shell_state: &mut NativeShellState,
    point: Point,
    modifiers: ModifiersState,
) -> Option<UiAction> {
    route_modal_and_chrome_actions(layout, model, motion_model, shell_state, point, modifiers)
        .or_else(|| route_browser_or_folder_row(layout, model, shell_state, point, modifiers))
        .or_else(|| route_shell_background(layout, model, shell_state, point, modifiers))
}

fn route_modal_and_chrome_actions(
    layout: &ShellLayout,
    model: &AppModel,
    motion_model: Option<&NativeMotionModel>,
    shell_state: &mut NativeShellState,
    point: Point,
    modifiers: ModifiersState,
) -> Option<UiAction> {
    if let Some(action) = shell_state.prompt_action_at_point(layout, model, point) {
        return Some(action);
    }
    if let Some(action) = shell_state.progress_action_at_point(layout, model, point) {
        return Some(action);
    }
    if let Some(action) = shell_state.options_panel_action_at_point(layout, model, point) {
        return Some(action);
    }
    if model.options_panel.visible {
        if shell_state.options_panel_contains_point_live(layout, model, point) {
            return None;
        }
        return Some(UiAction::CloseOptionsPanel);
    }
    if let Some(action) = shell_state.status_options_action_at_point(layout, model, point) {
        return Some(action);
    }
    if let Some(action) = shell_state.top_bar_volume_action_at_point(layout, point) {
        return Some(action);
    }
    if let Some(action) = shell_state.browser_tab_action_at_point(layout, point) {
        return Some(action);
    }
    if let Some(action) = shell_state.map_sample_action_at_point(layout, model, point) {
        return Some(action);
    }
    if let Some(action) =
        shell_state.browser_action_at_point(layout, model, point, modifiers.alt_key())
    {
        return Some(action);
    }
    if let Some(action) = shell_state.source_action_at_point(layout, model, point) {
        return Some(action);
    }
    if let Some(action) = shell_state.folder_header_action_at_point(layout, model, point) {
        return Some(action);
    }
    if let Some(action) = motion_model.and_then(|motion_model| {
        shell_state.waveform_toolbar_action_at_point_with_motion_and_modifiers(
            layout,
            motion_model,
            point,
            modifiers.shift_key(),
        )
    }) {
        return Some(action);
    }
    shell_state.waveform_toolbar_action_at_point_with_modifiers(
        layout,
        model,
        point,
        modifiers.shift_key(),
    )
}

fn route_browser_or_folder_row(
    layout: &ShellLayout,
    model: &AppModel,
    shell_state: &mut NativeShellState,
    point: Point,
    modifiers: ModifiersState,
) -> Option<UiAction> {
    if let Some(action) = shell_state.browser_row_similarity_action_at_point(layout, model, point) {
        return Some(action);
    }
    if let Some(visible_row) = shell_state.browser_row_at_point(layout, model, point) {
        let shift = modifiers.shift_key();
        let command = modifiers.control_key() || modifiers.super_key();
        return Some(if shift && command {
            UiAction::AddRangeBrowserSelection { visible_row }
        } else if shift {
            UiAction::ExtendBrowserSelectionToRow { visible_row }
        } else if command {
            UiAction::ToggleBrowserRowSelection { visible_row }
        } else {
            UiAction::FocusBrowserRow { visible_row }
        });
    }
    if let Some(index) = shell_state.folder_row_disclosure_at_point(layout, model, point) {
        return Some(folder_row_pointer_action(model, index));
    }
    shell_state
        .folder_row_at_point(layout, model, point)
        .map(|index| folder_row_pointer_action(model, index))
}

fn route_shell_background(
    layout: &ShellLayout,
    model: &AppModel,
    shell_state: &mut NativeShellState,
    point: Point,
    modifiers: ModifiersState,
) -> Option<UiAction> {
    let hit = layout.hit_test(point)?;
    match hit {
        ShellNodeKind::Sidebar => route_sidebar_background(layout, model, shell_state, point),
        ShellNodeKind::WaveformCard => {
            if layout.waveform_plot.contains(point) {
                Some(waveform_action_from_pointer(
                    layout, model, point, modifiers,
                ))
            } else {
                Some(UiAction::FocusWaveformPanel)
            }
        }
        ShellNodeKind::TopBar => Some(UiAction::ToggleTransport),
        ShellNodeKind::Content
        | ShellNodeKind::BrowserPanel
        | ShellNodeKind::BrowserTabs
        | ShellNodeKind::BrowserTable => Some(UiAction::FocusBrowserPanel),
        ShellNodeKind::StatusBar => Some(UiAction::FocusLoadedSampleInBrowser),
        _ => None,
    }
}

fn route_sidebar_background(
    layout: &ShellLayout,
    model: &AppModel,
    shell_state: &mut NativeShellState,
    point: Point,
) -> Option<UiAction> {
    if let Some(index) = shell_state.source_row_at_point(layout, model, point) {
        return Some(UiAction::FocusSourceRow { index });
    }
    if let Some(index) = shell_state.folder_row_disclosure_at_point(layout, model, point) {
        return Some(folder_row_pointer_action(model, index));
    }
    if let Some(index) = shell_state.folder_row_at_point(layout, model, point) {
        return Some(folder_row_pointer_action(model, index));
    }
    shell_state.sidebar_focus_action_at_point(layout, model, point)
}

fn folder_row_pointer_action(model: &AppModel, index: usize) -> UiAction {
    let Some(row) = model.sources.folder_rows.get(index) else {
        return UiAction::FocusFolderRow { index };
    };
    if matches!(
        row.kind,
        crate::app::FolderRowKind::CreateDraft | crate::app::FolderRowKind::RenameDraft
    ) {
        return UiAction::FocusFolderCreateInput;
    }
    let source_index = row.source_index.unwrap_or(index);
    if folder_row_click_toggles_expansion(model, index) {
        UiAction::ActivateFolderRow {
            index: source_index,
        }
    } else {
        UiAction::FocusFolderRow {
            index: source_index,
        }
    }
}

fn folder_row_click_toggles_expansion(model: &AppModel, index: usize) -> bool {
    let Some(row) = model.sources.folder_rows.get(index) else {
        return false;
    };
    row.has_children
        && !row.is_root
        && !matches!(
            row.kind,
            crate::app::FolderRowKind::CreateDraft | crate::app::FolderRowKind::RenameDraft
        )
        && model.sources.folder_search_query.trim().is_empty()
}
