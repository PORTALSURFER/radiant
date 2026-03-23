use super::*;

#[cfg(test)]
/// Resolve one waveform action for a captured waveform drag mode.
pub(super) fn waveform_drag_action_for_mode(
    layout: &ShellLayout,
    model: &AppModel,
    point: Point,
    mode: WaveformPointerDragMode,
) -> UiAction {
    waveform_drag_action_and_mode_for_point(layout, model, point, mode).0
}

/// Resolve one waveform action and updated drag mode for a captured waveform drag.
pub(super) fn waveform_drag_action_and_mode_for_point(
    layout: &ShellLayout,
    model: &AppModel,
    point: Point,
    mode: WaveformPointerDragMode,
) -> (UiAction, WaveformPointerDragMode) {
    let pointer_position = waveform_pointer_position_from_point(layout, model, point);
    let position_nanos = pointer_position.position_nanos;
    let (position_micros, next_mode) =
        waveform_drag_position_and_mode_for_point(layout, model, point, mode);
    let preserve_view_edge = waveform_point_is_outside_plot_x(layout, point);
    let action = match next_mode {
        WaveformPointerDragMode::Seek => UiAction::SeekWaveformPrecise { position_nanos },
        WaveformPointerDragMode::Cursor => UiAction::SetWaveformCursorPrecise { position_nanos },
        WaveformPointerDragMode::Selection { anchor_micros, .. } => {
            UiAction::SetWaveformSelectionRange {
                start_micros: anchor_micros,
                end_micros: position_micros,
                preserve_view_edge,
            }
        }
        WaveformPointerDragMode::SelectionSmartScale { anchor_micros, .. } => {
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
        WaveformPointerDragMode::EditSelection { anchor_micros, .. } => {
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
    };
    (action, next_mode)
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
        WaveformPointerDragMode::Selection { anchor_micros, .. } => {
            let anchor_x = waveform_x_for_micros(layout.waveform_plot, model, anchor_micros);
            (point.x - anchor_x).abs() > WAVEFORM_SELECTION_CLICK_SLOP_PX
        }
        _ => true,
    }
}

/// Resolve drag mode from an initial waveform action emitted on pointer press.
pub(super) fn waveform_drag_mode_for_action(action: &UiAction) -> Option<WaveformPointerDragMode> {
    match action {
        UiAction::SeekWaveformPrecise { .. } | UiAction::SeekWaveform { .. } => {
            Some(WaveformPointerDragMode::Seek)
        }
        UiAction::SetWaveformCursorPrecise { .. } | UiAction::SetWaveformCursor { .. } => {
            Some(WaveformPointerDragMode::Cursor)
        }
        UiAction::BeginWaveformSelectionAt { anchor_micros } => {
            Some(WaveformPointerDragMode::Selection {
                anchor_micros: *anchor_micros,
                boundary_lock: None,
            })
        }
        UiAction::SetWaveformSelectionRange { start_micros, .. } => {
            Some(WaveformPointerDragMode::Selection {
                anchor_micros: *start_micros,
                boundary_lock: None,
            })
        }
        UiAction::SetWaveformSelectionRangeSmartScale { start_micros, .. } => {
            Some(WaveformPointerDragMode::SelectionSmartScale {
                anchor_micros: *start_micros,
                boundary_lock: None,
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
                boundary_lock: None,
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

/// Resolve one absolute waveform position and next drag-mode lock state for the pointer.
fn waveform_drag_position_and_mode_for_point(
    layout: &ShellLayout,
    model: &AppModel,
    point: Point,
    mode: WaveformPointerDragMode,
) -> (u32, WaveformPointerDragMode) {
    match mode {
        WaveformPointerDragMode::Selection {
            anchor_micros,
            boundary_lock,
        } => {
            let (position_micros, boundary_lock) =
                waveform_selection_boundary_lock_for_point(layout, model, point, boundary_lock);
            (
                position_micros,
                WaveformPointerDragMode::Selection {
                    anchor_micros,
                    boundary_lock,
                },
            )
        }
        WaveformPointerDragMode::SelectionSmartScale {
            anchor_micros,
            boundary_lock,
        } => {
            let (position_micros, boundary_lock) =
                waveform_selection_boundary_lock_for_point(layout, model, point, boundary_lock);
            (
                position_micros,
                WaveformPointerDragMode::SelectionSmartScale {
                    anchor_micros,
                    boundary_lock,
                },
            )
        }
        WaveformPointerDragMode::EditSelection {
            anchor_micros,
            boundary_lock,
        } => {
            let (position_micros, boundary_lock) =
                waveform_selection_boundary_lock_for_point(layout, model, point, boundary_lock);
            (
                position_micros,
                WaveformPointerDragMode::EditSelection {
                    anchor_micros,
                    boundary_lock,
                },
            )
        }
        _ => (
            waveform_position_micros_from_point(layout, model, point),
            mode,
        ),
    }
}

/// Keep anchor-based drags pinned to one absolute edge while the pointer remains off-plot.
fn waveform_selection_boundary_lock_for_point(
    layout: &ShellLayout,
    model: &AppModel,
    point: Point,
    boundary_lock: Option<WaveformSelectionBoundaryLock>,
) -> (u32, Option<WaveformSelectionBoundaryLock>) {
    let Some(side) = waveform_outside_plot_side(layout, point) else {
        return (
            waveform_position_micros_from_point(layout, model, point),
            None,
        );
    };
    if let Some(boundary_lock) = boundary_lock.filter(|lock| lock.side == side) {
        return (boundary_lock.position_micros, Some(boundary_lock));
    }
    let position_micros = waveform_position_micros_from_point(layout, model, point);
    (
        position_micros,
        Some(WaveformSelectionBoundaryLock {
            side,
            position_micros,
        }),
    )
}

/// Return which horizontal waveform-plot side the pointer is currently beyond.
fn waveform_outside_plot_side(
    layout: &ShellLayout,
    point: Point,
) -> Option<WaveformOutsidePlotSide> {
    if point.x < layout.waveform_plot.min.x {
        Some(WaveformOutsidePlotSide::Left)
    } else if point.x > layout.waveform_plot.max.x {
        Some(WaveformOutsidePlotSide::Right)
    } else {
        None
    }
}
