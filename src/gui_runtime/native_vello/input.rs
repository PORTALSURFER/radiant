//! Keyboard/pointer/wheel action mapping for the native runtime.

use super::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum WaveformPointerDragMode {
    /// Drag updates seek/playhead position.
    Seek,
    /// Drag updates cursor position.
    Cursor,
    /// Drag extends playback selection from a fixed anchor milli value.
    Selection {
        /// Fixed anchor milli captured at drag start.
        anchor_milli: u16,
    },
    /// Drag extends edit selection from a fixed anchor milli value.
    EditSelection {
        /// Fixed anchor milli captured at drag start.
        anchor_milli: u16,
    },
}
pub(super) fn action_from_key(
    key: KeyCode,
    modifiers: ModifiersState,
    model: &AppModel,
) -> Option<UiAction> {
    if model.confirm_prompt.visible {
        let confirm_enabled = model
            .confirm_prompt
            .input_error
            .as_ref()
            .is_none_or(|error| error.trim().is_empty());
        return match key {
            KeyCode::Enter if confirm_enabled => Some(UiAction::ConfirmPrompt),
            KeyCode::C => Some(UiAction::CancelPrompt),
            _ => None,
        };
    }
    let shift = modifiers.shift_key();
    let command = modifiers.control_key() || modifiers.super_key();
    match key {
        KeyCode::ArrowLeft => Some(UiAction::MoveColumn { delta: -1 }),
        KeyCode::ArrowRight => Some(UiAction::MoveColumn { delta: 1 }),
        KeyCode::ArrowUp => {
            if shift && command {
                Some(UiAction::AddRangeBrowserSelectionFromFocus { delta: -1 })
            } else if shift {
                Some(UiAction::ExtendBrowserSelectionFromFocus { delta: -1 })
            } else {
                Some(UiAction::MoveBrowserFocus { delta: -1 })
            }
        }
        KeyCode::ArrowDown => {
            if shift && command {
                Some(UiAction::AddRangeBrowserSelectionFromFocus { delta: 1 })
            } else if shift {
                Some(UiAction::ExtendBrowserSelectionFromFocus { delta: 1 })
            } else {
                Some(UiAction::MoveBrowserFocus { delta: 1 })
            }
        }
        KeyCode::Num1 => Some(UiAction::SelectColumn { index: 0 }),
        KeyCode::Num2 => Some(UiAction::SelectColumn { index: 1 }),
        KeyCode::Num3 => Some(UiAction::SelectColumn { index: 2 }),
        KeyCode::A => Some(UiAction::SelectAllBrowserRows),
        KeyCode::B => Some(UiAction::StartNewFolder),
        KeyCode::C => Some(UiAction::ClearWaveformSelection),
        KeyCode::D => Some(UiAction::DeleteBrowserSelection),
        KeyCode::Enter => Some(UiAction::CommitFocusedBrowserRow),
        KeyCode::F => Some(UiAction::FocusBrowserSearch),
        KeyCode::G => Some(UiAction::DeleteFocusedFolder),
        KeyCode::I => Some(UiAction::StartBrowserRename),
        KeyCode::L => Some(UiAction::ToggleLoopPlayback),
        KeyCode::M => Some(UiAction::ZoomWaveformToSelection),
        KeyCode::N => Some(UiAction::TagBrowserSelection {
            target: crate::app::BrowserTagTarget::Neutral,
        }),
        KeyCode::OpenBracket => Some(UiAction::ZoomWaveform {
            zoom_in: false,
            steps: 1,
        }),
        KeyCode::P => model
            .progress_overlay
            .cancelable
            .then_some(UiAction::CancelProgress),
        KeyCode::CloseBracket => Some(UiAction::ZoomWaveform {
            zoom_in: true,
            steps: 1,
        }),
        KeyCode::Slash => Some(UiAction::ZoomWaveformFull),
        KeyCode::Quote => Some(UiAction::FocusFolderSearch),
        KeyCode::R => Some(UiAction::Redo),
        KeyCode::S => Some(UiAction::FocusSourcesPanel),
        KeyCode::Space => Some(UiAction::ReplayFromLastStart),
        KeyCode::T => Some(UiAction::ToggleFocusedBrowserRowSelection),
        KeyCode::U => Some(UiAction::Undo),
        KeyCode::W => Some(UiAction::FocusWaveformPanel),
        KeyCode::X => Some(UiAction::TagBrowserSelection {
            target: crate::app::BrowserTagTarget::Trash,
        }),
        KeyCode::Y => Some(UiAction::TagBrowserSelection {
            target: crate::app::BrowserTagTarget::Keep,
        }),
        KeyCode::Z => Some(UiAction::StartFolderRename),
        _ => None,
    }
}

pub(super) fn action_from_pointer(
    layout: &ShellLayout,
    model: &AppModel,
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
    if let Some(action) = shell_state.top_bar_options_action_at_point(layout, point) {
        return Some(action);
    }
    if let Some(action) = shell_state.top_bar_volume_action_at_point(layout, point) {
        return Some(action);
    }
    if let Some(action) = shell_state.update_action_at_point(layout, model, point) {
        return Some(action);
    }
    if let Some(action) = shell_state.browser_tab_action_at_point(layout, point) {
        return Some(action);
    }
    if let Some(action) = shell_state.map_sample_action_at_point(layout, model, point) {
        return Some(action);
    }
    if let Some(action) = shell_state.browser_action_at_point(layout, model, point) {
        return Some(action);
    }
    if let Some(action) = shell_state.source_action_at_point(layout, model, point) {
        return Some(action);
    }
    if let Some(action) = shell_state.waveform_toolbar_action_at_point(layout, model, point) {
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
    if let Some(index) = shell_state.folder_row_at_point(layout, model, point) {
        return Some(UiAction::FocusFolderRow { index });
    }

    let hit = layout.hit_test(point)?;
    match hit {
        ShellNodeKind::Sidebar => shell_state
            .source_row_at_point(layout, model, point)
            .map_or(Some(UiAction::FocusSourcesPanel), |index| {
                Some(UiAction::SelectSourceRow { index })
            }),
        ShellNodeKind::WaveformCard => Some(waveform_action_from_pointer(
            layout, model, point, modifiers,
        )),
        ShellNodeKind::TopBar => Some(UiAction::ToggleTransport),
        ShellNodeKind::Content
        | ShellNodeKind::BrowserPanel
        | ShellNodeKind::BrowserTabs
        | ShellNodeKind::BrowserTable => Some(UiAction::FocusBrowserPanel),
        ShellNodeKind::StatusBar => Some(UiAction::FocusLoadedSampleInBrowser),
        _ => None,
    }
}

/// Build one waveform action from pointer position and active modifier keys.
pub(super) fn waveform_action_from_pointer(
    layout: &ShellLayout,
    model: &AppModel,
    point: Point,
    modifiers: ModifiersState,
) -> UiAction {
    let position_milli = waveform_position_milli_from_point(layout, point);
    let alt = modifiers.alt_key();
    let shift = modifiers.shift_key();
    let command = modifiers.control_key() || modifiers.super_key();
    if command {
        UiAction::SetWaveformCursor { position_milli }
    } else if alt {
        UiAction::SeekWaveform { position_milli }
    } else if shift {
        UiAction::SetWaveformSelectionRange {
            start_milli: waveform_anchor_milli(model),
            end_milli: position_milli,
        }
    } else {
        UiAction::SetWaveformSelectionRange {
            start_milli: position_milli,
            end_milli: position_milli,
        }
    }
}

/// Build one waveform edit-selection action from pointer position.
pub(super) fn waveform_edit_action_from_pointer(
    layout: &ShellLayout,
    point: Point,
    _modifiers: ModifiersState,
) -> UiAction {
    let position_milli = waveform_position_milli_from_point(layout, point);
    UiAction::SetWaveformEditSelectionRange {
        start_milli: position_milli,
        end_milli: position_milli,
    }
}

/// Resolve one waveform action for a captured waveform drag mode.
pub(super) fn waveform_drag_action_for_mode(
    layout: &ShellLayout,
    point: Point,
    mode: WaveformPointerDragMode,
) -> UiAction {
    let position_milli = waveform_position_milli_from_point(layout, point);
    match mode {
        WaveformPointerDragMode::Seek => UiAction::SeekWaveform { position_milli },
        WaveformPointerDragMode::Cursor => UiAction::SetWaveformCursor { position_milli },
        WaveformPointerDragMode::Selection { anchor_milli } => {
            UiAction::SetWaveformSelectionRange {
                start_milli: anchor_milli,
                end_milli: position_milli,
            }
        }
        WaveformPointerDragMode::EditSelection { anchor_milli } => {
            UiAction::SetWaveformEditSelectionRange {
                start_milli: anchor_milli,
                end_milli: position_milli,
            }
        }
    }
}

/// Resolve drag mode from an initial waveform action emitted on pointer press.
pub(super) fn waveform_drag_mode_for_action(action: &UiAction) -> Option<WaveformPointerDragMode> {
    match action {
        UiAction::SeekWaveform { .. } => Some(WaveformPointerDragMode::Seek),
        UiAction::SetWaveformCursor { .. } => Some(WaveformPointerDragMode::Cursor),
        UiAction::SetWaveformSelectionRange { start_milli, .. } => {
            Some(WaveformPointerDragMode::Selection {
                anchor_milli: *start_milli,
            })
        }
        UiAction::SetWaveformEditSelectionRange { start_milli, .. } => {
            Some(WaveformPointerDragMode::EditSelection {
                anchor_milli: *start_milli,
            })
        }
        _ => None,
    }
}

/// Resolve normalized waveform milli position from an arbitrary pointer point.
pub(super) fn waveform_position_milli_from_point(layout: &ShellLayout, point: Point) -> u16 {
    let inner = layout.waveform_plot;
    let width = inner.width().max(1.0);
    let clamped_x = point.x.clamp(inner.min.x, inner.max.x);
    let ratio = ((clamped_x - inner.min.x) / width).clamp(0.0, 1.0);
    ratio_to_milli(ratio)
}

pub(super) fn ratio_to_milli(ratio: f32) -> u16 {
    (ratio.clamp(0.0, 1.0) * 1000.0).round() as u16
}

pub(super) fn waveform_anchor_milli(model: &AppModel) -> u16 {
    model
        .waveform
        .selection_milli
        .map(|selection| selection.start_milli)
        .or(model.waveform.cursor_milli)
        .or(model.waveform.playhead_milli)
        .unwrap_or(0)
}

pub(super) fn browser_wheel_row_delta(
    layout: &ShellLayout,
    model: &AppModel,
    point: Point,
    style: &StyleTokens,
    delta: MouseScrollDelta,
) -> Option<i8> {
    if model.map.active || !layout.browser_panel.contains(point) {
        return None;
    }
    let row_stride = (style.sizing.browser_row_height + style.sizing.browser_row_gap).max(1.0);
    let raw = match delta {
        MouseScrollDelta::LineDelta(_, y) => -y,
        MouseScrollDelta::PixelDelta(position) => -(position.y as f32) / row_stride,
    };
    if raw == 0.0 {
        return None;
    }
    let mut steps = raw.round();
    if steps.abs() < 1.0 {
        steps = raw.signum();
        if steps == 0.0 {
            return None;
        }
    }
    if steps == 0.0 {
        return None;
    }
    let clamped = if steps > 1.0 {
        steps.min(i8::MAX as f32)
    } else {
        steps.max(i8::MIN as f32)
    };
    Some(clamped as i8)
}
