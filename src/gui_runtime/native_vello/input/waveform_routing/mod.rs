//! Waveform gesture routing from pointer state into `UiAction`s.

use super::*;

mod clear;
mod drag;
mod press;

use self::{
    clear::{waveform_clear_action_from_pointer, waveform_new_selection_action_from_pointer},
    press::{
        waveform_edit_selection_shift_action_from_pointer,
        waveform_primary_press_action_from_pointer, waveform_selection_drag_action_from_pointer,
        waveform_selection_resize_action_from_pointer,
        waveform_selection_shift_action_from_pointer,
    },
};

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
    if let Some(action) =
        waveform_primary_press_action_from_pointer(layout, model, point, command, alt, shift)
    {
        return action;
    }
    if let Some(action) =
        waveform_new_selection_action_from_pointer(layout, model, point, command, alt, shift)
    {
        return action;
    }
    if let Some(action) =
        waveform_clear_action_from_pointer(layout, model, point, command, alt, shift)
    {
        return action;
    }
    if command {
        UiAction::SetWaveformCursor { position_milli }
    } else if alt {
        UiAction::SeekWaveform { position_milli }
    } else if shift {
        UiAction::SetWaveformSelectionRange {
            start_micros: waveform_anchor_micros(model),
            end_micros: micros_from_milli(position_milli),
            preserve_view_edge: false,
        }
    } else {
        UiAction::BeginWaveformSelectionAt {
            anchor_micros: micros_from_milli(position_milli),
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
        || waveform_selection_resize_action_from_pointer(layout, model, point, false).is_some()
        || waveform_selection_resize_action_from_pointer(layout, model, point, true).is_some()
}

/// Return whether the pointer is hovering the playback-selection drag handle.
pub(super) fn waveform_selection_drag_handle_hovered(
    layout: &ShellLayout,
    model: &AppModel,
    point: Point,
) -> bool {
    waveform_selection_drag_handle_hit_rect(layout, model).is_some_and(|rect| rect.contains(point))
}

/// Build one waveform edit-selection action from pointer position.
pub(super) fn waveform_edit_action_from_pointer(
    layout: &ShellLayout,
    model: &AppModel,
    point: Point,
    modifiers: ModifiersState,
) -> UiAction {
    if !layout.waveform_plot.contains(point) {
        return UiAction::FocusWaveformPanel;
    }
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
    if layout.waveform_plot.contains(point)
        && model.waveform.edit_selection_milli.is_some()
        && !waveform_edit_selection_contains_point(layout, model, point)
    {
        return UiAction::ClearWaveformEditSelection;
    }
    let position_micros = waveform_position_micros_from_point(layout, model, point);
    UiAction::SetWaveformEditSelectionRange {
        start_micros: position_micros,
        end_micros: position_micros,
        preserve_view_edge: false,
    }
}

#[cfg(test)]
pub(super) fn waveform_drag_action_for_mode(
    layout: &ShellLayout,
    model: &AppModel,
    point: Point,
    mode: WaveformPointerDragMode,
) -> UiAction {
    drag::waveform_drag_action_for_mode(layout, model, point, mode)
}

/// Resolve one waveform drag action and the updated drag mode for the pointer.
pub(super) fn waveform_drag_action_and_mode_for_point(
    layout: &ShellLayout,
    model: &AppModel,
    point: Point,
    mode: WaveformPointerDragMode,
) -> (UiAction, WaveformPointerDragMode) {
    drag::waveform_drag_action_and_mode_for_point(layout, model, point, mode)
}

pub(super) fn waveform_drag_exceeds_click_slop(
    layout: &ShellLayout,
    model: &AppModel,
    point: Point,
    mode: WaveformPointerDragMode,
) -> bool {
    drag::waveform_drag_exceeds_click_slop(layout, model, point, mode)
}

pub(super) fn waveform_drag_mode_for_action(action: &UiAction) -> Option<WaveformPointerDragMode> {
    drag::waveform_drag_mode_for_action(action)
}

pub(super) fn waveform_drag_mode_is_edit_fade(mode: WaveformPointerDragMode) -> bool {
    drag::waveform_drag_mode_is_edit_fade(mode)
}

pub(super) fn waveform_press_action_emits_immediately(action: &UiAction) -> bool {
    drag::waveform_press_action_emits_immediately(action)
}
