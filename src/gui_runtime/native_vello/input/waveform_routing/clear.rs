use super::*;

/// Arm a fresh playback-selection drag when plain left press starts outside the
/// current playback selection body.
///
/// Release-without-drag is handled later by the runtime, which clears the old
/// playback selection and seeks from the click point. Dragging past click slop
/// converts the interaction into a normal new selection drag.
pub(super) fn waveform_new_selection_action_from_pointer(
    layout: &ShellLayout,
    model: &AppModel,
    point: Point,
    command: bool,
    alt: bool,
    shift: bool,
) -> Option<UiAction> {
    if command || alt || shift || !layout.waveform_plot.contains(point) {
        return None;
    }
    if model.waveform.selection_milli.is_none()
        || model.waveform.edit_selection_milli.is_some()
        || waveform_selection_contains_point(layout, model, point)
        || waveform_edit_selection_contains_point(layout, model, point)
    {
        return None;
    }
    let position_micros = waveform_position_micros_from_point(layout, model, point);
    Some(UiAction::BeginWaveformSelectionAt {
        anchor_micros: position_micros,
    })
}

/// Resolve outside-click deselection for one plain left-click in the waveform plot.
pub(super) fn waveform_clear_action_from_pointer(
    layout: &ShellLayout,
    model: &AppModel,
    point: Point,
    command: bool,
    alt: bool,
    shift: bool,
) -> Option<UiAction> {
    if command || alt || shift || !layout.waveform_plot.contains(point) {
        return None;
    }
    let clear_edit = model.waveform.edit_selection_milli.is_some()
        && !waveform_edit_selection_contains_point(layout, model, point);
    let clear_playback = model.waveform.selection_milli.is_some()
        && !waveform_selection_contains_point(layout, model, point);
    match (clear_edit, clear_playback) {
        (true, true) => Some(UiAction::ClearWaveformSelections),
        (true, false) => Some(UiAction::ClearWaveformEditSelection),
        (false, true) => Some(UiAction::ClearWaveformSelection),
        (false, false) => None,
    }
}
