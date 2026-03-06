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
    /// Drag shifts the playback selection while preserving its width.
    SelectionShift {
        /// Pointer milli captured at drag start.
        pointer_milli: u16,
        /// Original playback-selection start milli.
        start_milli: u16,
        /// Original playback-selection end milli.
        end_milli: u16,
    },
    /// Drag extends edit selection from a fixed anchor milli value.
    EditSelection {
        /// Fixed anchor milli captured at drag start.
        anchor_milli: u16,
    },
    /// Drag shifts the edit selection while preserving its width.
    EditSelectionShift {
        /// Pointer milli captured at drag start.
        pointer_milli: u16,
        /// Original edit-selection start milli.
        start_milli: u16,
        /// Original edit-selection end milli.
        end_milli: u16,
    },
    /// Drag updates the edit fade-in end handle.
    EditFadeInEnd,
    /// Drag updates the edit fade-in mute-start handle.
    EditFadeInMuteStart,
    /// Drag updates the edit fade-in curve.
    EditFadeInCurve,
    /// Drag updates the edit fade-out start handle.
    EditFadeOutStart,
    /// Drag updates the edit fade-out mute-end handle.
    EditFadeOutMuteEnd,
    /// Drag updates the edit fade-out curve.
    EditFadeOutCurve,
}

/// Half-width in pixels used for fade-handle hit testing.
const WAVEFORM_EDIT_FADE_HANDLE_HIT_HALF_WIDTH: f32 = 7.0;
/// Half-width in pixels used for waveform edge-resize hit testing.
const WAVEFORM_RESIZE_EDGE_HIT_HALF_WIDTH: f32 = 7.0;
/// Fraction of waveform height used by centered resize-edge hit regions.
const WAVEFORM_RESIZE_EDGE_HEIGHT_RATIO: f32 = 0.34;
/// Width/height in logical pixels for the playback-selection drag handle.
const WAVEFORM_SELECTION_DRAG_HANDLE_SIZE: f32 = 12.0;
/// Extra hit slop around the playback-selection drag handle.
const WAVEFORM_SELECTION_DRAG_HANDLE_HIT_INSET: f32 = 4.0;
/// Width in logical pixels for bottom-center selection shift handles.
const WAVEFORM_SELECTION_SHIFT_HANDLE_WIDTH: f32 = 14.0;
/// Height in logical pixels for bottom-center selection shift handles.
const WAVEFORM_SELECTION_SHIFT_HANDLE_HEIGHT: f32 = 7.0;
/// Extra hit slop around bottom-center selection shift handles.
const WAVEFORM_SELECTION_SHIFT_HANDLE_HIT_INSET: f32 = 4.0;
/// Pixel-delta normalization factor for wheel-driven waveform zoom steps.
const WAVEFORM_WHEEL_ZOOM_PIXEL_STEP: f32 = 48.0;
/// Integer precision used by pointer-anchored zoom ratios (`0..=1_000_000`).
const WAVEFORM_ANCHOR_RATIO_MICROS_SCALE: u32 = 1_000_000;
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
        KeyCode::OpenBracket => Some(UiAction::TagBrowserSelection {
            target: crate::app::BrowserTagTarget::Trash,
        }),
        KeyCode::P => model
            .progress_overlay
            .cancelable
            .then_some(UiAction::CancelProgress),
        KeyCode::CloseBracket => Some(UiAction::TagBrowserSelection {
            target: crate::app::BrowserTagTarget::Keep,
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

#[cfg(test)]
pub(super) fn action_from_pointer(
    layout: &ShellLayout,
    model: &AppModel,
    shell_state: &mut NativeShellState,
    point: Point,
    modifiers: ModifiersState,
) -> Option<UiAction> {
    action_from_pointer_with_motion(layout, model, None, shell_state, point, modifiers)
}

/// Resolve one pointer click action using optional retained motion-model context.
pub(super) fn action_from_pointer_with_motion(
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
    if let Some(action) = motion_model.and_then(|motion_model| {
        shell_state.waveform_toolbar_action_at_point_with_motion(layout, motion_model, point)
    }) {
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
    let position_milli = waveform_position_milli_from_point(layout, model, point);
    let alt = modifiers.alt_key();
    let shift = modifiers.shift_key();
    let command = modifiers.control_key() || modifiers.super_key();
    if !command
        && !alt
        && !shift
        && let Some(action) =
            waveform_edit_selection_shift_action_from_pointer(layout, model, point)
    {
        return action;
    }
    if !command
        && !alt
        && !shift
        && let Some(action) = waveform_selection_drag_action_from_pointer(layout, model, point)
    {
        return action;
    }
    if !command
        && !alt
        && !shift
        && let Some(action) = waveform_selection_shift_action_from_pointer(layout, model, point)
    {
        return action;
    }
    if !command
        && !alt
        && !shift
        && let Some(action) = waveform_edit_resize_action_from_pointer(layout, model, point)
    {
        return action;
    }
    if !command
        && alt
        && !shift
        && let Some(action) = waveform_edit_fade_curve_action_from_pointer(layout, model, point)
    {
        return action;
    }
    if !command
        && !alt
        && !shift
        && let Some(action) = waveform_edit_fade_handle_action_from_pointer(layout, model, point)
    {
        return action;
    }
    if !command
        && !alt
        && !shift
        && let Some(action) = waveform_selection_resize_action_from_pointer(layout, model, point)
    {
        return action;
    }
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

/// Return whether the pointer is hovering any waveform resize/fade handle.
pub(super) fn waveform_resize_handle_hovered(
    layout: &ShellLayout,
    model: &AppModel,
    point: Point,
) -> bool {
    waveform_selection_drag_action_from_pointer(layout, model, point).is_some()
        || waveform_selection_shift_action_from_pointer(layout, model, point).is_some()
        || waveform_edit_selection_shift_action_from_pointer(layout, model, point).is_some()
        || waveform_edit_resize_action_from_pointer(layout, model, point).is_some()
        || waveform_edit_fade_handle_action_from_pointer(layout, model, point).is_some()
        || waveform_selection_resize_action_from_pointer(layout, model, point).is_some()
}

/// Resolve one selection-drag action when the pointer lands on the playback-selection handle.
fn waveform_selection_drag_action_from_pointer(
    layout: &ShellLayout,
    model: &AppModel,
    point: Point,
) -> Option<UiAction> {
    waveform_selection_drag_handle_hit_rect(layout, model).and_then(|rect| {
        rect.contains(point)
            .then_some(UiAction::StartWaveformSelectionDrag {
                pointer_x: point.x.max(0.0).round() as u16,
                pointer_y: point.y.max(0.0).round() as u16,
            })
    })
}

/// Resolve one playback-selection shift action from the bottom-center handle.
fn waveform_selection_shift_action_from_pointer(
    layout: &ShellLayout,
    model: &AppModel,
    point: Point,
) -> Option<UiAction> {
    let selection = model.waveform.selection_milli?;
    waveform_selection_shift_handle_hit_rect(layout, model, selection).and_then(|rect| {
        rect.contains(point)
            .then_some(UiAction::BeginWaveformSelectionShift {
                pointer_milli: waveform_position_milli_from_point(layout, model, point),
                start_milli: selection.start_milli,
                end_milli: selection.end_milli,
            })
    })
}

/// Resolve one edit-selection shift action from the bottom-center handle.
fn waveform_edit_selection_shift_action_from_pointer(
    layout: &ShellLayout,
    model: &AppModel,
    point: Point,
) -> Option<UiAction> {
    let selection = model.waveform.edit_selection_milli?;
    waveform_selection_shift_handle_hit_rect(layout, model, selection).and_then(|rect| {
        rect.contains(point)
            .then_some(UiAction::BeginWaveformEditSelectionShift {
                pointer_milli: waveform_position_milli_from_point(layout, model, point),
                start_milli: selection.start_milli,
                end_milli: selection.end_milli,
            })
    })
}

/// Return whether the pointer is hovering the playback-selection drag handle.
pub(super) fn waveform_selection_drag_handle_hovered(
    layout: &ShellLayout,
    model: &AppModel,
    point: Point,
) -> bool {
    waveform_selection_drag_handle_hit_rect(layout, model).is_some_and(|rect| rect.contains(point))
}

/// Resolve one playback-selection resize action when the pointer lands on an edge handle.
fn waveform_selection_resize_action_from_pointer(
    layout: &ShellLayout,
    model: &AppModel,
    point: Point,
) -> Option<UiAction> {
    let selection = model.waveform.selection_milli?;
    if !layout.waveform_plot.contains(point) {
        return None;
    }
    let selection_start = selection.start_milli.min(selection.end_milli);
    let selection_end = selection.start_milli.max(selection.end_milli);
    if selection_end <= selection_start {
        return None;
    }
    let selection_start_x = waveform_x_for_milli(layout.waveform_plot, model, selection_start);
    let selection_end_x = waveform_x_for_milli(layout.waveform_plot, model, selection_end);
    let (handle_top, handle_bottom) = waveform_centered_resize_edge_y_bounds(layout.waveform_plot);
    if point.y < handle_top || point.y > handle_bottom {
        return None;
    }
    let left_distance = (point.x - selection_start_x).abs();
    let right_distance = (point.x - selection_end_x).abs();
    let left_hit = left_distance <= WAVEFORM_RESIZE_EDGE_HIT_HALF_WIDTH;
    let right_hit = right_distance <= WAVEFORM_RESIZE_EDGE_HIT_HALF_WIDTH;
    if !left_hit && !right_hit {
        return None;
    }
    let position_milli = waveform_position_milli_from_point(layout, model, point);
    if left_hit && (!right_hit || left_distance <= right_distance) {
        return Some(UiAction::SetWaveformSelectionRange {
            start_milli: selection_end,
            end_milli: position_milli,
        });
    }
    Some(UiAction::SetWaveformSelectionRange {
        start_milli: selection_start,
        end_milli: position_milli,
    })
}

/// Build one waveform edit-selection action from pointer position.
pub(super) fn waveform_edit_action_from_pointer(
    layout: &ShellLayout,
    model: &AppModel,
    point: Point,
    modifiers: ModifiersState,
) -> UiAction {
    if modifiers.alt_key()
        && let Some(action) = waveform_edit_fade_curve_action_from_pointer(layout, model, point)
    {
        return action;
    }
    if let Some(action) = waveform_edit_selection_shift_action_from_pointer(layout, model, point) {
        return action;
    }
    if let Some(action) = waveform_edit_resize_action_from_pointer(layout, model, point) {
        return action;
    }
    if let Some(action) = waveform_edit_fade_handle_action_from_pointer(layout, model, point) {
        return action;
    }
    let position_milli = waveform_position_milli_from_point(layout, model, point);
    UiAction::SetWaveformEditSelectionRange {
        start_milli: position_milli,
        end_milli: position_milli,
    }
}

/// Resolve one waveform action for a captured waveform drag mode.
pub(super) fn waveform_drag_action_for_mode(
    layout: &ShellLayout,
    model: &AppModel,
    point: Point,
    mode: WaveformPointerDragMode,
) -> UiAction {
    let position_milli = waveform_position_milli_from_point(layout, model, point);
    match mode {
        WaveformPointerDragMode::Seek => UiAction::SeekWaveform { position_milli },
        WaveformPointerDragMode::Cursor => UiAction::SetWaveformCursor { position_milli },
        WaveformPointerDragMode::Selection { anchor_milli } => {
            UiAction::SetWaveformSelectionRange {
                start_milli: anchor_milli,
                end_milli: position_milli,
            }
        }
        WaveformPointerDragMode::SelectionShift {
            pointer_milli,
            start_milli,
            end_milli,
        } => {
            let (start_milli, end_milli) =
                shift_waveform_range_milli(pointer_milli, position_milli, start_milli, end_milli);
            UiAction::SetWaveformSelectionRange {
                start_milli,
                end_milli,
            }
        }
        WaveformPointerDragMode::EditSelection { anchor_milli } => {
            UiAction::SetWaveformEditSelectionRange {
                start_milli: anchor_milli,
                end_milli: position_milli,
            }
        }
        WaveformPointerDragMode::EditSelectionShift {
            pointer_milli,
            start_milli,
            end_milli,
        } => {
            let (start_milli, end_milli) =
                shift_waveform_range_milli(pointer_milli, position_milli, start_milli, end_milli);
            UiAction::SetWaveformEditSelectionRange {
                start_milli,
                end_milli,
            }
        }
        WaveformPointerDragMode::EditFadeInEnd => {
            UiAction::SetWaveformEditFadeInEnd { position_milli }
        }
        WaveformPointerDragMode::EditFadeInMuteStart => {
            UiAction::SetWaveformEditFadeInMuteStart { position_milli }
        }
        WaveformPointerDragMode::EditFadeInCurve => UiAction::SetWaveformEditFadeInCurve {
            curve_milli: waveform_edit_fade_curve_milli_from_point(layout, point),
        },
        WaveformPointerDragMode::EditFadeOutStart => {
            UiAction::SetWaveformEditFadeOutStart { position_milli }
        }
        WaveformPointerDragMode::EditFadeOutMuteEnd => {
            UiAction::SetWaveformEditFadeOutMuteEnd { position_milli }
        }
        WaveformPointerDragMode::EditFadeOutCurve => UiAction::SetWaveformEditFadeOutCurve {
            curve_milli: waveform_edit_fade_curve_milli_from_point(layout, point),
        },
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
        UiAction::BeginWaveformSelectionShift {
            pointer_milli,
            start_milli,
            end_milli,
        } => Some(WaveformPointerDragMode::SelectionShift {
            pointer_milli: *pointer_milli,
            start_milli: *start_milli,
            end_milli: *end_milli,
        }),
        UiAction::SetWaveformEditSelectionRange { start_milli, .. } => {
            Some(WaveformPointerDragMode::EditSelection {
                anchor_milli: *start_milli,
            })
        }
        UiAction::BeginWaveformEditSelectionShift {
            pointer_milli,
            start_milli,
            end_milli,
        } => Some(WaveformPointerDragMode::EditSelectionShift {
            pointer_milli: *pointer_milli,
            start_milli: *start_milli,
            end_milli: *end_milli,
        }),
        UiAction::SetWaveformEditFadeInEnd { .. } => Some(WaveformPointerDragMode::EditFadeInEnd),
        UiAction::SetWaveformEditFadeInMuteStart { .. } => {
            Some(WaveformPointerDragMode::EditFadeInMuteStart)
        }
        UiAction::SetWaveformEditFadeInCurve { .. } => {
            Some(WaveformPointerDragMode::EditFadeInCurve)
        }
        UiAction::SetWaveformEditFadeOutStart { .. } => {
            Some(WaveformPointerDragMode::EditFadeOutStart)
        }
        UiAction::SetWaveformEditFadeOutMuteEnd { .. } => {
            Some(WaveformPointerDragMode::EditFadeOutMuteEnd)
        }
        UiAction::SetWaveformEditFadeOutCurve { .. } => {
            Some(WaveformPointerDragMode::EditFadeOutCurve)
        }
        _ => None,
    }
}

/// Return whether one waveform drag mode edits fade geometry and needs a release callback.
pub(super) fn waveform_drag_mode_is_edit_fade(mode: WaveformPointerDragMode) -> bool {
    matches!(
        mode,
        WaveformPointerDragMode::EditFadeInEnd
            | WaveformPointerDragMode::EditFadeInMuteStart
            | WaveformPointerDragMode::EditFadeInCurve
            | WaveformPointerDragMode::EditFadeOutStart
            | WaveformPointerDragMode::EditFadeOutMuteEnd
            | WaveformPointerDragMode::EditFadeOutCurve
    )
}

/// Return whether one waveform press action should mutate model state immediately.
///
/// Selection/edit/fade gestures are armed on press and only emit once the
/// pointer actually moves. This keeps simple clicks from creating zero-width
/// markers or nudging handles without a drag.
pub(super) fn waveform_press_action_emits_immediately(action: &UiAction) -> bool {
    !matches!(
        action,
        UiAction::SetWaveformSelectionRange { .. }
            | UiAction::BeginWaveformSelectionShift { .. }
            | UiAction::SetWaveformEditSelectionRange { .. }
            | UiAction::BeginWaveformEditSelectionShift { .. }
            | UiAction::SetWaveformEditFadeInEnd { .. }
            | UiAction::SetWaveformEditFadeInMuteStart { .. }
            | UiAction::SetWaveformEditFadeInCurve { .. }
            | UiAction::SetWaveformEditFadeOutStart { .. }
            | UiAction::SetWaveformEditFadeOutMuteEnd { .. }
            | UiAction::SetWaveformEditFadeOutCurve { .. }
    )
}

/// Resolve normalized waveform milli position from an arbitrary pointer point.
pub(super) fn waveform_position_milli_from_point(
    layout: &ShellLayout,
    model: &AppModel,
    point: Point,
) -> u16 {
    let view = normalized_waveform_view(model);
    let ratio_in_view = waveform_ratio_from_point(layout, point);
    let absolute_ratio = view.start + (view.width * ratio_in_view);
    ratio_to_milli(absolute_ratio)
}

/// Resolve pointer x-position as a normalized ratio within the current plot.
pub(super) fn waveform_ratio_from_point(layout: &ShellLayout, point: Point) -> f32 {
    let inner = layout.waveform_plot;
    let width = inner.width().max(1.0);
    let clamped_x = point.x.clamp(inner.min.x, inner.max.x);
    ((clamped_x - inner.min.x) / width).clamp(0.0, 1.0)
}

pub(super) fn ratio_to_milli(ratio: f32) -> u16 {
    (ratio.clamp(0.0, 1.0) * 1000.0).round() as u16
}

/// Convert one normalized view ratio to deterministic micro-units.
pub(super) fn ratio_to_micros(ratio: f32) -> u32 {
    (ratio.clamp(0.0, 1.0) * WAVEFORM_ANCHOR_RATIO_MICROS_SCALE as f32).round() as u32
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

/// Resolve one fade-handle action when a pointer lands near edit fade handles.
fn waveform_edit_fade_handle_action_from_pointer(
    layout: &ShellLayout,
    model: &AppModel,
    point: Point,
) -> Option<UiAction> {
    let selection = model.waveform.edit_selection_milli?;
    if !layout.waveform_plot.contains(point) {
        return None;
    }
    if selection.end_milli <= selection.start_milli {
        return None;
    }
    let fade_in_end_milli = model
        .waveform
        .edit_fade_in_end_milli
        .unwrap_or(selection.start_milli)
        .clamp(selection.start_milli, selection.end_milli);
    let fade_in_mute_start_milli = model
        .waveform
        .edit_fade_in_mute_start_milli
        .unwrap_or(selection.start_milli)
        .min(selection.start_milli);
    let fade_out_start_milli = model
        .waveform
        .edit_fade_out_start_milli
        .unwrap_or(selection.end_milli)
        .clamp(selection.start_milli, selection.end_milli);
    let fade_out_mute_end_milli = model
        .waveform
        .edit_fade_out_mute_end_milli
        .unwrap_or(selection.end_milli)
        .max(selection.end_milli);
    let fade_in_x = waveform_x_for_milli(layout.waveform_plot, model, fade_in_end_milli);
    let fade_in_mute_x =
        waveform_x_for_milli(layout.waveform_plot, model, fade_in_mute_start_milli);
    let fade_out_x = waveform_x_for_milli(layout.waveform_plot, model, fade_out_start_milli);
    let fade_out_mute_x =
        waveform_x_for_milli(layout.waveform_plot, model, fade_out_mute_end_milli);
    let in_distance = (point.x - fade_in_x).abs();
    let in_mute_distance = (point.x - fade_in_mute_x).abs();
    let out_distance = (point.x - fade_out_x).abs();
    let out_mute_distance = (point.x - fade_out_mute_x).abs();
    let threshold = WAVEFORM_EDIT_FADE_HANDLE_HIT_HALF_WIDTH;
    if in_distance > threshold
        && in_mute_distance > threshold
        && out_distance > threshold
        && out_mute_distance > threshold
    {
        return None;
    }
    let position_milli = waveform_position_milli_from_point(layout, model, point);
    let bottom_half = point.y >= layout.waveform_plot.min.y + (layout.waveform_plot.height() * 0.5);
    if bottom_half
        && in_mute_distance <= threshold
        && (in_mute_distance <= out_mute_distance || out_mute_distance > threshold)
    {
        Some(UiAction::SetWaveformEditFadeInMuteStart { position_milli })
    } else if bottom_half && out_mute_distance <= threshold {
        Some(UiAction::SetWaveformEditFadeOutMuteEnd { position_milli })
    } else if in_distance <= out_distance {
        Some(UiAction::SetWaveformEditFadeInEnd { position_milli })
    } else {
        Some(UiAction::SetWaveformEditFadeOutStart { position_milli })
    }
}

/// Resolve one edit-fade curve action when Alt is held over a fade region or handle.
fn waveform_edit_fade_curve_action_from_pointer(
    layout: &ShellLayout,
    model: &AppModel,
    point: Point,
) -> Option<UiAction> {
    let selection = model.waveform.edit_selection_milli?;
    if !layout.waveform_plot.contains(point) {
        return None;
    }
    let selection_start = selection.start_milli.min(selection.end_milli);
    let selection_end = selection.start_milli.max(selection.end_milli);
    if selection_end <= selection_start {
        return None;
    }
    let fade_in_end_milli = model
        .waveform
        .edit_fade_in_end_milli
        .unwrap_or(selection.start_milli)
        .clamp(selection_start, selection_end);
    let fade_in_mute_start_milli = model
        .waveform
        .edit_fade_in_mute_start_milli
        .unwrap_or(selection_start)
        .min(selection_start);
    let fade_out_start_milli = model
        .waveform
        .edit_fade_out_start_milli
        .unwrap_or(selection.end_milli)
        .clamp(selection_start, selection_end);
    let fade_out_mute_end_milli = model
        .waveform
        .edit_fade_out_mute_end_milli
        .unwrap_or(selection_end)
        .max(selection_end);
    let fade_in_mute_x =
        waveform_x_for_milli(layout.waveform_plot, model, fade_in_mute_start_milli);
    let selection_start_x = waveform_x_for_milli(layout.waveform_plot, model, selection_start);
    let selection_end_x = waveform_x_for_milli(layout.waveform_plot, model, selection_end);
    let fade_in_x = waveform_x_for_milli(layout.waveform_plot, model, fade_in_end_milli);
    let fade_out_x = waveform_x_for_milli(layout.waveform_plot, model, fade_out_start_milli);
    let fade_out_mute_x =
        waveform_x_for_milli(layout.waveform_plot, model, fade_out_mute_end_milli);
    let threshold = WAVEFORM_EDIT_FADE_HANDLE_HIT_HALF_WIDTH;
    let in_region_hit = point.x >= fade_in_mute_x - threshold && point.x <= fade_in_x + threshold;
    let out_region_hit =
        point.x >= fade_out_x - threshold && point.x <= fade_out_mute_x + threshold;
    let curve_milli = waveform_edit_fade_curve_milli_from_point(layout, point);
    if in_region_hit && (!out_region_hit || point.x <= (selection_start_x + selection_end_x) * 0.5)
    {
        return Some(UiAction::SetWaveformEditFadeInCurve { curve_milli });
    }
    if out_region_hit {
        return Some(UiAction::SetWaveformEditFadeOutCurve { curve_milli });
    }
    None
}

/// Resolve one edit-selection resize action when the pointer lands on an edge handle.
fn waveform_edit_resize_action_from_pointer(
    layout: &ShellLayout,
    model: &AppModel,
    point: Point,
) -> Option<UiAction> {
    let selection = model.waveform.edit_selection_milli?;
    if !layout.waveform_plot.contains(point) {
        return None;
    }
    let selection_start = selection.start_milli.min(selection.end_milli);
    let selection_end = selection.start_milli.max(selection.end_milli);
    if selection_end <= selection_start {
        return None;
    }
    let selection_start_x = waveform_x_for_milli(layout.waveform_plot, model, selection_start);
    let selection_end_x = waveform_x_for_milli(layout.waveform_plot, model, selection_end);
    let (handle_top, handle_bottom) = waveform_centered_resize_edge_y_bounds(layout.waveform_plot);
    if point.y < handle_top || point.y > handle_bottom {
        return None;
    }
    let left_distance = (point.x - selection_start_x).abs();
    let right_distance = (point.x - selection_end_x).abs();
    let left_hit = left_distance <= WAVEFORM_RESIZE_EDGE_HIT_HALF_WIDTH;
    let right_hit = right_distance <= WAVEFORM_RESIZE_EDGE_HIT_HALF_WIDTH;
    if !left_hit && !right_hit {
        return None;
    }
    let position_milli = waveform_position_milli_from_point(layout, model, point);
    if left_hit && (!right_hit || left_distance <= right_distance) {
        return Some(UiAction::SetWaveformEditSelectionRange {
            start_milli: selection_end,
            end_milli: position_milli,
        });
    }
    Some(UiAction::SetWaveformEditSelectionRange {
        start_milli: selection_start,
        end_milli: position_milli,
    })
}

/// Return the expanded hit rect for the playback-selection drag handle.
fn waveform_selection_drag_handle_hit_rect(
    layout: &ShellLayout,
    model: &AppModel,
) -> Option<UiRect> {
    let selection = model.waveform.selection_milli?;
    let start_milli = selection.start_milli.min(selection.end_milli);
    let end_milli = selection.start_milli.max(selection.end_milli);
    if end_milli <= start_milli {
        return None;
    }
    let start_x = waveform_x_for_milli(layout.waveform_plot, model, start_milli);
    let end_x = waveform_x_for_milli(layout.waveform_plot, model, end_milli);
    let selection_rect = UiRect::from_min_max(
        Point::new(start_x.min(end_x), layout.waveform_plot.min.y),
        Point::new(start_x.max(end_x), layout.waveform_plot.max.y),
    );
    let handle = waveform_selection_drag_handle_rect(selection_rect);
    let hit_min = Point::new(
        (handle.min.x - WAVEFORM_SELECTION_DRAG_HANDLE_HIT_INSET).max(layout.waveform_plot.min.x),
        (handle.min.y - WAVEFORM_SELECTION_DRAG_HANDLE_HIT_INSET).max(layout.waveform_plot.min.y),
    );
    let hit_max = Point::new(
        (handle.max.x + WAVEFORM_SELECTION_DRAG_HANDLE_HIT_INSET).min(layout.waveform_plot.max.x),
        (handle.max.y + WAVEFORM_SELECTION_DRAG_HANDLE_HIT_INSET).min(layout.waveform_plot.max.y),
    );
    Some(UiRect::from_min_max(hit_min, hit_max))
}

/// Return the expanded hit rect for one bottom-center selection shift handle.
fn waveform_selection_shift_handle_hit_rect(
    layout: &ShellLayout,
    model: &AppModel,
    selection: crate::app::NormalizedRangeModel,
) -> Option<UiRect> {
    let start_milli = selection.start_milli.min(selection.end_milli);
    let end_milli = selection.start_milli.max(selection.end_milli);
    if end_milli <= start_milli {
        return None;
    }
    let selection_rect = UiRect::from_min_max(
        Point::new(
            waveform_x_for_milli(layout.waveform_plot, model, start_milli),
            layout.waveform_plot.min.y,
        ),
        Point::new(
            waveform_x_for_milli(layout.waveform_plot, model, end_milli),
            layout.waveform_plot.max.y,
        ),
    );
    let handle = waveform_selection_shift_handle_rect(selection_rect);
    Some(UiRect::from_min_max(
        Point::new(
            (handle.min.x - WAVEFORM_SELECTION_SHIFT_HANDLE_HIT_INSET)
                .max(layout.waveform_plot.min.x),
            (handle.min.y - WAVEFORM_SELECTION_SHIFT_HANDLE_HIT_INSET)
                .max(layout.waveform_plot.min.y),
        ),
        Point::new(
            (handle.max.x + WAVEFORM_SELECTION_SHIFT_HANDLE_HIT_INSET)
                .min(layout.waveform_plot.max.x),
            (handle.max.y + WAVEFORM_SELECTION_SHIFT_HANDLE_HIT_INSET)
                .min(layout.waveform_plot.max.y),
        ),
    ))
}

/// Return the visible playback-selection drag handle rect.
fn waveform_selection_drag_handle_rect(selection_rect: UiRect) -> UiRect {
    let size = WAVEFORM_SELECTION_DRAG_HANDLE_SIZE
        .min(selection_rect.width().max(1.0))
        .min(selection_rect.height().max(1.0));
    let min = Point::new(selection_rect.max.x - size, selection_rect.max.y - size);
    UiRect::from_min_max(min, selection_rect.max)
}

/// Return the visible bottom-center selection shift handle rect.
fn waveform_selection_shift_handle_rect(selection_rect: UiRect) -> UiRect {
    let width = WAVEFORM_SELECTION_SHIFT_HANDLE_WIDTH.min(selection_rect.width().max(1.0));
    let height = WAVEFORM_SELECTION_SHIFT_HANDLE_HEIGHT.min(selection_rect.height().max(1.0));
    let left = (selection_rect.min.x + (selection_rect.width() - width) * 0.5)
        .clamp(selection_rect.min.x, selection_rect.max.x - width.max(1.0));
    let top = (selection_rect.max.y - height).max(selection_rect.min.y);
    UiRect::from_min_max(
        Point::new(left, top),
        Point::new(
            (left + width).min(selection_rect.max.x),
            selection_rect.max.y,
        ),
    )
}

/// Shift one milli-based waveform range while preserving width and clamping to bounds.
fn shift_waveform_range_milli(
    pointer_milli: u16,
    position_milli: u16,
    start_milli: u16,
    end_milli: u16,
) -> (u16, u16) {
    let original_start = i32::from(start_milli.min(end_milli));
    let original_end = i32::from(start_milli.max(end_milli));
    let width = original_end - original_start;
    if width <= 0 {
        return (start_milli, end_milli);
    }
    let delta = i32::from(position_milli) - i32::from(pointer_milli);
    let shifted_start = (original_start + delta).clamp(0, 1000 - width);
    let shifted_end = shifted_start + width;
    (shifted_start as u16, shifted_end as u16)
}

/// Convert a normalized waveform milli position into plot-space x.
fn waveform_x_for_milli(plot: UiRect, model: &AppModel, milli: u16) -> f32 {
    let view = normalized_waveform_view(model);
    let absolute_ratio = f32::from(milli.min(1000)) / 1000.0;
    let ratio_in_view = if view.width <= f32::EPSILON {
        0.0
    } else {
        ((absolute_ratio - view.start) / view.width).clamp(0.0, 1.0)
    };
    plot.min.x + (plot.width() * ratio_in_view)
}

/// Return the centered vertical hit span used by waveform resize edges.
fn waveform_centered_resize_edge_y_bounds(plot: UiRect) -> (f32, f32) {
    let height = (plot.height() * WAVEFORM_RESIZE_EDGE_HEIGHT_RATIO)
        .max(1.0)
        .min(plot.height());
    let center_y = plot.min.y + (plot.height() * 0.5);
    let top = (center_y - (height * 0.5)).max(plot.min.y);
    let bottom = (top + height).min(plot.max.y).max(top + 1.0);
    (top, bottom)
}

/// Map pointer Y within the waveform plot to one fade-curve milli value.
fn waveform_edit_fade_curve_milli_from_point(layout: &ShellLayout, point: Point) -> u16 {
    let plot = layout.waveform_plot;
    let height = plot.height().max(1.0);
    let clamped_y = point.y.clamp(plot.min.y, plot.max.y);
    let ratio = 1.0 - ((clamped_y - plot.min.y) / height).clamp(0.0, 1.0);
    ratio_to_milli(ratio)
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

/// Map one mouse-wheel delta into waveform zoom action while hovering the waveform card.
pub(super) fn waveform_wheel_zoom_action(
    layout: &ShellLayout,
    _model: &AppModel,
    point: Point,
    delta: MouseScrollDelta,
) -> Option<UiAction> {
    if !layout.waveform_card.contains(point) {
        return None;
    }
    let raw = match delta {
        MouseScrollDelta::LineDelta(_, y) => y,
        MouseScrollDelta::PixelDelta(position) => {
            (position.y as f32) / WAVEFORM_WHEEL_ZOOM_PIXEL_STEP
        }
    };
    if raw.abs() <= f32::EPSILON {
        return None;
    }
    let zoom_in = raw > 0.0;
    let mut steps = raw.abs().round();
    if steps < 1.0 {
        steps = 1.0;
    }
    Some(UiAction::ZoomWaveform {
        zoom_in,
        steps: steps.min(u8::MAX as f32) as u8,
        anchor_ratio_micros: Some(ratio_to_micros(waveform_ratio_from_point(layout, point))),
    })
}

/// Normalized waveform viewport bounds (`0..=1`) resolved from panel milli fields.
fn normalized_waveform_view(model: &AppModel) -> WaveformNormalizedView {
    let start_milli = model.waveform.view_start_milli.min(1000);
    let end_milli = model.waveform.view_end_milli.min(1000).max(start_milli);
    let start = f32::from(start_milli) / 1000.0;
    let end = f32::from(end_milli) / 1000.0;
    let width = (end - start).max(0.0);
    WaveformNormalizedView { start, width }
}

/// Precomputed normalized waveform viewport bounds for pointer conversions.
struct WaveformNormalizedView {
    start: f32,
    width: f32,
}
