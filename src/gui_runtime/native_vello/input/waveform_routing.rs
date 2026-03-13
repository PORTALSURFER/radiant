//! Waveform gesture routing from pointer state into `UiAction`s.

use super::*;

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
        && alt
        && !shift
        && let Some(action) =
            waveform_selection_resize_action_from_pointer(layout, model, point, true)
    {
        return action;
    }
    if !command
        && !alt
        && !shift
        && let Some(action) =
            waveform_selection_resize_action_from_pointer(layout, model, point, false)
    {
        return action;
    }
    if !command
        && !alt
        && !shift
        && layout.waveform_plot.contains(point)
        && model.waveform.edit_selection_milli.is_some()
        && !waveform_edit_selection_contains_point(layout, model, point)
    {
        return UiAction::ClearWaveformEditSelection;
    }
    if !command
        && !alt
        && !shift
        && layout.waveform_plot.contains(point)
        && model.waveform.selection_milli.is_some()
        && !waveform_selection_contains_point(layout, model, point)
    {
        return UiAction::ClearWaveformSelection;
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
        UiAction::SetWaveformSelectionRange {
            start_micros: micros_from_milli(position_milli),
            end_micros: micros_from_milli(position_milli),
            preserve_view_edge: false,
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
                pointer_micros: waveform_position_micros_from_point(layout, model, point),
                start_micros: selection.start_micros,
                end_micros: selection.end_micros,
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
                pointer_micros: waveform_position_micros_from_point(layout, model, point),
                start_micros: selection.start_micros,
                end_micros: selection.end_micros,
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
    smart_scale: bool,
) -> Option<UiAction> {
    let selection = model.waveform.selection_milli?;
    if !layout.waveform_plot.contains(point) {
        return None;
    }
    let selection_start = selection.start_micros.min(selection.end_micros);
    let selection_end = selection.start_micros.max(selection.end_micros);
    if selection_end <= selection_start {
        return None;
    }
    let selection_start_x = waveform_x_for_micros(layout.waveform_plot, model, selection_start);
    let selection_end_x = waveform_x_for_micros(layout.waveform_plot, model, selection_end);
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
    let position_micros = waveform_position_micros_from_point(layout, model, point);
    if left_hit && (!right_hit || left_distance <= right_distance) {
        return Some(if smart_scale {
            UiAction::SetWaveformSelectionRangeSmartScale {
                start_micros: selection.end_micros,
                end_micros: position_micros,
            }
        } else {
            UiAction::SetWaveformSelectionRange {
                start_micros: selection.end_micros,
                end_micros: position_micros,
                preserve_view_edge: false,
            }
        });
    }
    Some(if smart_scale {
        UiAction::SetWaveformSelectionRangeSmartScale {
            start_micros: selection.start_micros,
            end_micros: position_micros,
        }
    } else {
        UiAction::SetWaveformSelectionRange {
            start_micros: selection.start_micros,
            end_micros: position_micros,
            preserve_view_edge: false,
        }
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

/// Resolve one waveform action for a captured waveform drag mode.
pub(super) fn waveform_drag_action_for_mode(
    layout: &ShellLayout,
    model: &AppModel,
    point: Point,
    mode: WaveformPointerDragMode,
) -> UiAction {
    let position_micros = waveform_position_micros_from_point(layout, model, point);
    let preserve_view_edge = waveform_point_is_outside_plot_x(layout, point);
    match mode {
        WaveformPointerDragMode::Seek => UiAction::SeekWaveform {
            position_milli: ratio_to_milli(normalized_waveform_position_ratio(
                layout, model, point,
            )),
        },
        WaveformPointerDragMode::Cursor => UiAction::SetWaveformCursor {
            position_milli: ratio_to_milli(normalized_waveform_position_ratio(
                layout, model, point,
            )),
        },
        WaveformPointerDragMode::Selection { anchor_micros } => {
            UiAction::SetWaveformSelectionRange {
                start_micros: anchor_micros,
                end_micros: position_micros,
                preserve_view_edge,
            }
        }
        WaveformPointerDragMode::SelectionSmartScale { anchor_micros } => {
            UiAction::SetWaveformSelectionRangeSmartScale {
                start_micros: anchor_micros,
                end_micros: position_micros,
            }
        }
        WaveformPointerDragMode::SelectionShift {
            pointer_micros,
            start_micros,
            end_micros,
        } => {
            let (start_micros, end_micros) = shift_waveform_range_micros(
                pointer_micros,
                position_micros,
                start_micros,
                end_micros,
            );
            UiAction::SetWaveformSelectionRange {
                start_micros,
                end_micros,
                preserve_view_edge: false,
            }
        }
        WaveformPointerDragMode::EditSelection { anchor_micros } => {
            UiAction::SetWaveformEditSelectionRange {
                start_micros: anchor_micros,
                end_micros: position_micros,
                preserve_view_edge,
            }
        }
        WaveformPointerDragMode::EditSelectionShift {
            pointer_micros,
            start_micros,
            end_micros,
        } => {
            let (start_micros, end_micros) = shift_waveform_range_micros(
                pointer_micros,
                position_micros,
                start_micros,
                end_micros,
            );
            UiAction::SetWaveformEditSelectionRange {
                start_micros,
                end_micros,
                preserve_view_edge: false,
            }
        }
        WaveformPointerDragMode::EditFadeInEnd => {
            UiAction::SetWaveformEditFadeInEnd { position_micros }
        }
        WaveformPointerDragMode::EditFadeInMuteStart => {
            UiAction::SetWaveformEditFadeInMuteStart { position_micros }
        }
        WaveformPointerDragMode::EditFadeInCurve => UiAction::SetWaveformEditFadeInCurve {
            curve_milli: waveform_edit_fade_curve_milli_from_point(layout, point),
        },
        WaveformPointerDragMode::EditFadeOutStart => {
            UiAction::SetWaveformEditFadeOutStart { position_micros }
        }
        WaveformPointerDragMode::EditFadeOutMuteEnd => {
            UiAction::SetWaveformEditFadeOutMuteEnd { position_micros }
        }
        WaveformPointerDragMode::EditFadeOutCurve => UiAction::SetWaveformEditFadeOutCurve {
            curve_milli: waveform_edit_fade_curve_milli_from_point(layout, point),
        },
    }
}

/// Return whether an armed waveform drag moved far enough to emit selection updates.
///
/// New playback-selection drags use a small horizontal click-slop so minor
/// pointer wobble still behaves like a click/seek instead of creating a
/// micro-selection.
pub(super) fn waveform_drag_exceeds_click_slop(
    layout: &ShellLayout,
    model: &AppModel,
    point: Point,
    mode: WaveformPointerDragMode,
) -> bool {
    match mode {
        WaveformPointerDragMode::Selection { anchor_micros } => {
            let anchor_x = waveform_x_for_micros(layout.waveform_plot, model, anchor_micros);
            (point.x - anchor_x).abs() > WAVEFORM_SELECTION_CLICK_SLOP_PX
        }
        _ => true,
    }
}

/// Resolve drag mode from an initial waveform action emitted on pointer press.
pub(super) fn waveform_drag_mode_for_action(action: &UiAction) -> Option<WaveformPointerDragMode> {
    match action {
        UiAction::SeekWaveform { .. } => Some(WaveformPointerDragMode::Seek),
        UiAction::SetWaveformCursor { .. } => Some(WaveformPointerDragMode::Cursor),
        UiAction::SetWaveformSelectionRange { start_micros, .. } => {
            Some(WaveformPointerDragMode::Selection {
                anchor_micros: *start_micros,
            })
        }
        UiAction::SetWaveformSelectionRangeSmartScale { start_micros, .. } => {
            Some(WaveformPointerDragMode::SelectionSmartScale {
                anchor_micros: *start_micros,
            })
        }
        UiAction::BeginWaveformSelectionShift {
            pointer_micros,
            start_micros,
            end_micros,
        } => Some(WaveformPointerDragMode::SelectionShift {
            pointer_micros: *pointer_micros,
            start_micros: *start_micros,
            end_micros: *end_micros,
        }),
        UiAction::SetWaveformEditSelectionRange { start_micros, .. } => {
            Some(WaveformPointerDragMode::EditSelection {
                anchor_micros: *start_micros,
            })
        }
        UiAction::BeginWaveformEditSelectionShift {
            pointer_micros,
            start_micros,
            end_micros,
        } => Some(WaveformPointerDragMode::EditSelectionShift {
            pointer_micros: *pointer_micros,
            start_micros: *start_micros,
            end_micros: *end_micros,
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
            | UiAction::SetWaveformSelectionRangeSmartScale { .. }
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

fn normalized_waveform_position_ratio(layout: &ShellLayout, model: &AppModel, point: Point) -> f32 {
    let view_start = model.waveform.view_start_micros.min(1_000_000) as f32 / 1_000_000.0;
    let view_end = model
        .waveform
        .view_end_micros
        .min(1_000_000)
        .max(model.waveform.view_start_micros.min(1_000_000)) as f32
        / 1_000_000.0;
    let view_width = (view_end - view_start).max(0.0);
    let ratio_in_view = waveform_ratio_from_point(layout, point);
    (view_start + (view_width * ratio_in_view)).clamp(0.0, 1.0)
}
