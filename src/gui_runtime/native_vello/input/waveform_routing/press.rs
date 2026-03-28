use super::*;

/// Resolve the highest-priority waveform press gesture for the current modifiers.
pub(super) fn waveform_primary_press_action_from_pointer(
    layout: &ShellLayout,
    model: &AppModel,
    point: Point,
    command: bool,
    alt: bool,
    shift: bool,
) -> Option<UiAction> {
    if !command
        && !alt
        && !shift
        && let Some(action) =
            waveform_edit_selection_shift_action_from_pointer(layout, model, point)
    {
        return Some(action);
    }
    if !command
        && !alt
        && !shift
        && let Some(action) = waveform_selection_drag_action_from_pointer(layout, model, point)
    {
        return Some(action);
    }
    if !command
        && !shift
        && let Some(action) = waveform_selection_shift_action_from_pointer(layout, model, point)
    {
        return Some(action);
    }
    if !command
        && !alt
        && !shift
        && let Some(action) = waveform_edit_resize_action_from_pointer(layout, model, point)
    {
        return Some(action);
    }
    if !command
        && alt
        && !shift
        && let Some(action) = waveform_edit_fade_curve_action_from_pointer(layout, model, point)
    {
        return Some(action);
    }
    if !command
        && !alt
        && !shift
        && let Some(action) = waveform_edit_fade_handle_action_from_pointer(layout, model, point)
    {
        return Some(action);
    }
    if !command
        && shift
        && let Some(action) =
            waveform_selection_resize_action_from_pointer(layout, model, point, true)
    {
        return Some(action);
    }
    if !command
        && !shift
        && let Some(action) =
            waveform_selection_resize_action_from_pointer(layout, model, point, false)
    {
        return Some(action);
    }
    None
}

/// Resolve one direct playback-selection edge adjustment for command-click gestures.
pub(super) fn waveform_selection_edge_adjust_action_from_pointer(
    layout: &ShellLayout,
    model: &AppModel,
    point: Point,
    shift: bool,
) -> Option<UiAction> {
    waveform_edge_adjust_action(
        layout,
        model,
        model.waveform.selection_milli,
        point,
        shift,
        |start_micros, end_micros| UiAction::SetWaveformSelectionRange {
            start_micros,
            end_micros,
            snap_override: false,
            preserve_view_edge: false,
        },
    )
}

/// Resolve one direct edit-selection edge adjustment for command-click gestures.
pub(super) fn waveform_edit_selection_edge_adjust_action_from_pointer(
    layout: &ShellLayout,
    model: &AppModel,
    point: Point,
    shift: bool,
) -> Option<UiAction> {
    waveform_edge_adjust_action(
        layout,
        model,
        model.waveform.edit_selection_milli,
        point,
        shift,
        |start_micros, end_micros| UiAction::SetWaveformEditSelectionRange {
            start_micros,
            end_micros,
            preserve_view_edge: false,
        },
    )
}

/// Resolve one direct playback-selection slide action for shift-click gestures.
pub(super) fn waveform_selection_slide_action_from_pointer(
    layout: &ShellLayout,
    model: &AppModel,
    point: Point,
) -> Option<UiAction> {
    waveform_slide_action_from_pointer(
        layout,
        model,
        model.waveform.selection_milli,
        point,
        |start_micros, end_micros| UiAction::SetWaveformSelectionRange {
            start_micros,
            end_micros,
            snap_override: false,
            preserve_view_edge: false,
        },
    )
}

/// Resolve one direct edit-selection slide action for shift-right-click gestures.
pub(super) fn waveform_edit_selection_slide_action_from_pointer(
    layout: &ShellLayout,
    model: &AppModel,
    point: Point,
) -> Option<UiAction> {
    waveform_slide_action_from_pointer(
        layout,
        model,
        model.waveform.edit_selection_milli,
        point,
        |start_micros, end_micros| UiAction::SetWaveformEditSelectionRange {
            start_micros,
            end_micros,
            preserve_view_edge: false,
        },
    )
}

/// Resolve one selection-drag action when the pointer lands on the playback-selection handle.
pub(super) fn waveform_selection_drag_action_from_pointer(
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
pub(super) fn waveform_selection_shift_action_from_pointer(
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
pub(super) fn waveform_edit_selection_shift_action_from_pointer(
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

/// Resolve one playback-selection resize action when the pointer lands on an edge handle.
pub(super) fn waveform_selection_resize_action_from_pointer(
    layout: &ShellLayout,
    model: &AppModel,
    point: Point,
    smart_scale: bool,
) -> Option<UiAction> {
    let selection = model.waveform.selection_milli?;
    if !layout.waveform_plot.contains(point)
        || !waveform_selection_contains_point(layout, model, point)
    {
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
                snap_override: false,
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
            snap_override: false,
            preserve_view_edge: false,
        }
    })
}

/// Build one direct edge-adjust action, sliding only when the new edge would invert the range.
fn waveform_edge_adjust_action(
    layout: &ShellLayout,
    model: &AppModel,
    selection: Option<crate::app::NormalizedRangeModel>,
    point: Point,
    shift: bool,
    build: impl FnOnce(u32, u32) -> UiAction,
) -> Option<UiAction> {
    if !layout.waveform_plot.contains(point) {
        return None;
    }
    let selection = selection?;
    let position_micros = waveform_position_micros_from_point(layout, model, point);
    let selection_start = selection.start_micros.min(selection.end_micros);
    let selection_end = selection.start_micros.max(selection.end_micros);
    let (start_micros, end_micros) = if shift {
        if position_micros < selection_start {
            shift_waveform_range_micros(
                selection_end,
                position_micros,
                selection_start,
                selection_end,
            )
        } else {
            (selection_start, position_micros)
        }
    } else if position_micros > selection_end {
        shift_waveform_range_micros(
            selection_start,
            position_micros,
            selection_start,
            selection_end,
        )
    } else {
        (position_micros, selection_end)
    };
    Some(build(start_micros, end_micros))
}

/// Build one direct slide action that moves the selection start to the click position.
fn waveform_slide_action_from_pointer(
    layout: &ShellLayout,
    model: &AppModel,
    selection: Option<crate::app::NormalizedRangeModel>,
    point: Point,
    build: impl FnOnce(u32, u32) -> UiAction,
) -> Option<UiAction> {
    if !layout.waveform_plot.contains(point) {
        return None;
    }
    let selection = selection?;
    let position_micros = waveform_position_micros_from_point(layout, model, point);
    let (start_micros, end_micros) = shift_waveform_range_micros(
        selection.start_micros,
        position_micros,
        selection.start_micros,
        selection.end_micros,
    );
    Some(build(start_micros, end_micros))
}
