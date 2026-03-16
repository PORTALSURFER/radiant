//! Keyboard/pointer/wheel action mapping for the native runtime.

use super::*;

mod key;
mod pointer;
mod waveform_geometry;
mod waveform_handles;
mod waveform_routing;
mod wheel;

use self::waveform_geometry::micros_from_milli;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum WaveformPointerDragMode {
    /// Drag updates seek/playhead position.
    Seek,
    /// Drag updates cursor position.
    Cursor,
    /// Drag extends playback selection from a fixed anchor micro value.
    Selection {
        /// Fixed anchor micro position captured at drag start.
        anchor_micros: u32,
    },
    /// Drag resizes a playback selection without snapping and recomputes BPM from a 4-beat span.
    SelectionSmartScale {
        /// Fixed anchor micro position captured at drag start.
        anchor_micros: u32,
    },
    /// Drag shifts the playback selection while preserving its width.
    SelectionShift {
        /// Pointer micro position captured at drag start.
        pointer_micros: u32,
        /// Original playback-selection start micro position.
        start_micros: u32,
        /// Original playback-selection end micro position.
        end_micros: u32,
    },
    /// Drag extends edit selection from a fixed anchor micro value.
    EditSelection {
        /// Fixed anchor micro position captured at drag start.
        anchor_micros: u32,
    },
    /// Drag shifts the edit selection while preserving its width.
    EditSelectionShift {
        /// Pointer micro position captured at drag start.
        pointer_micros: u32,
        /// Original edit-selection start micro position.
        start_micros: u32,
        /// Original edit-selection end micro position.
        end_micros: u32,
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
const WAVEFORM_EDIT_FADE_TOP_TAB_SIZE: f32 = 10.0;
/// Horizontal drag distance required before a new playback selection counts as intentional.
const WAVEFORM_SELECTION_CLICK_SLOP_PX: f32 = 3.0;
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
    key::action_from_key(key, modifiers, model)
}

#[cfg(test)]
pub(super) fn action_from_pointer(
    layout: &ShellLayout,
    model: &AppModel,
    shell_state: &mut NativeShellState,
    point: Point,
    modifiers: ModifiersState,
) -> Option<UiAction> {
    pointer::action_from_pointer_with_motion(layout, model, None, shell_state, point, modifiers)
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
    pointer::action_from_pointer_with_motion(
        layout,
        model,
        motion_model,
        shell_state,
        point,
        modifiers,
    )
}

pub(super) fn waveform_action_from_pointer(
    layout: &ShellLayout,
    model: &AppModel,
    point: Point,
    modifiers: ModifiersState,
) -> UiAction {
    waveform_routing::waveform_action_from_pointer(layout, model, point, modifiers)
}

pub(super) fn waveform_resize_handle_hovered(
    layout: &ShellLayout,
    model: &AppModel,
    point: Point,
) -> bool {
    waveform_routing::waveform_resize_handle_hovered(layout, model, point)
}

pub(super) fn waveform_selection_drag_handle_hovered(
    layout: &ShellLayout,
    model: &AppModel,
    point: Point,
) -> bool {
    waveform_routing::waveform_selection_drag_handle_hovered(layout, model, point)
}

pub(super) fn waveform_edit_action_from_pointer(
    layout: &ShellLayout,
    model: &AppModel,
    point: Point,
    modifiers: ModifiersState,
) -> UiAction {
    waveform_routing::waveform_edit_action_from_pointer(layout, model, point, modifiers)
}

pub(super) fn waveform_drag_action_for_mode(
    layout: &ShellLayout,
    model: &AppModel,
    point: Point,
    mode: WaveformPointerDragMode,
) -> UiAction {
    waveform_routing::waveform_drag_action_for_mode(layout, model, point, mode)
}

pub(super) fn waveform_drag_exceeds_click_slop(
    layout: &ShellLayout,
    model: &AppModel,
    point: Point,
    mode: WaveformPointerDragMode,
) -> bool {
    waveform_routing::waveform_drag_exceeds_click_slop(layout, model, point, mode)
}

pub(super) fn waveform_drag_mode_for_action(action: &UiAction) -> Option<WaveformPointerDragMode> {
    waveform_routing::waveform_drag_mode_for_action(action)
}

pub(super) fn waveform_drag_mode_is_edit_fade(mode: WaveformPointerDragMode) -> bool {
    waveform_routing::waveform_drag_mode_is_edit_fade(mode)
}

pub(super) fn waveform_press_action_emits_immediately(action: &UiAction) -> bool {
    waveform_routing::waveform_press_action_emits_immediately(action)
}

pub(super) fn waveform_position_milli_from_point(
    layout: &ShellLayout,
    model: &AppModel,
    point: Point,
) -> u16 {
    waveform_geometry::waveform_position_milli_from_point(layout, model, point)
}

pub(super) fn waveform_position_micros_from_point(
    layout: &ShellLayout,
    model: &AppModel,
    point: Point,
) -> u32 {
    waveform_geometry::waveform_position_micros_from_point(layout, model, point)
}

pub(super) fn waveform_ratio_from_point(layout: &ShellLayout, point: Point) -> f32 {
    waveform_geometry::waveform_ratio_from_point(layout, point)
}

pub(super) fn ratio_to_milli(ratio: f32) -> u16 {
    waveform_geometry::ratio_to_milli(ratio)
}

pub(super) fn ratio_to_micros(ratio: f32) -> u32 {
    waveform_geometry::ratio_to_micros(ratio)
}

pub(super) fn waveform_anchor_micros(model: &AppModel) -> u32 {
    waveform_geometry::waveform_anchor_micros(model)
}

pub(super) fn shift_waveform_range_micros(
    pointer_micros: u32,
    position_micros: u32,
    start_micros: u32,
    end_micros: u32,
) -> (u32, u32) {
    waveform_geometry::shift_waveform_range_micros(
        pointer_micros,
        position_micros,
        start_micros,
        end_micros,
    )
}

pub(super) fn waveform_point_is_outside_plot_x(layout: &ShellLayout, point: Point) -> bool {
    waveform_geometry::waveform_point_is_outside_plot_x(layout, point)
}

pub(super) fn waveform_x_for_micros(plot: UiRect, model: &AppModel, micros: u32) -> f32 {
    waveform_geometry::waveform_x_for_micros(plot, model, micros)
}

pub(super) fn waveform_centered_resize_edge_y_bounds(plot: UiRect) -> (f32, f32) {
    waveform_geometry::waveform_centered_resize_edge_y_bounds(plot)
}

pub(super) fn waveform_edit_fade_curve_milli_from_point(layout: &ShellLayout, point: Point) -> u16 {
    waveform_geometry::waveform_edit_fade_curve_milli_from_point(layout, point)
}

pub(super) fn waveform_edit_fade_handle_action_from_pointer(
    layout: &ShellLayout,
    model: &AppModel,
    point: Point,
) -> Option<UiAction> {
    waveform_handles::waveform_edit_fade_handle_action_from_pointer(layout, model, point)
}

pub(super) fn waveform_edit_fade_curve_action_from_pointer(
    layout: &ShellLayout,
    model: &AppModel,
    point: Point,
) -> Option<UiAction> {
    waveform_handles::waveform_edit_fade_curve_action_from_pointer(layout, model, point)
}

pub(super) fn waveform_edit_resize_action_from_pointer(
    layout: &ShellLayout,
    model: &AppModel,
    point: Point,
) -> Option<UiAction> {
    waveform_handles::waveform_edit_resize_action_from_pointer(layout, model, point)
}

pub(super) fn waveform_edit_selection_contains_point(
    layout: &ShellLayout,
    model: &AppModel,
    point: Point,
) -> bool {
    waveform_handles::waveform_edit_selection_contains_point(layout, model, point)
}

pub(super) fn waveform_selection_contains_point(
    layout: &ShellLayout,
    model: &AppModel,
    point: Point,
) -> bool {
    waveform_handles::waveform_selection_contains_point(layout, model, point)
}

pub(super) fn waveform_selection_drag_handle_hit_rect(
    layout: &ShellLayout,
    model: &AppModel,
) -> Option<UiRect> {
    waveform_handles::waveform_selection_drag_handle_hit_rect(layout, model)
}

pub(super) fn waveform_selection_shift_handle_hit_rect(
    layout: &ShellLayout,
    model: &AppModel,
    selection: crate::app::NormalizedRangeModel,
) -> Option<UiRect> {
    waveform_handles::waveform_selection_shift_handle_hit_rect(layout, model, selection)
}

pub(super) fn browser_wheel_row_delta(
    layout: &ShellLayout,
    model: &AppModel,
    point: Point,
    style: &StyleTokens,
    delta: MouseScrollDelta,
) -> Option<i8> {
    wheel::browser_wheel_row_delta(layout, model, point, style, delta)
}

pub(super) fn browser_view_start_after_wheel(
    current_view_start: usize,
    visible_count: usize,
    viewport_len: usize,
    steps: i8,
) -> Option<usize> {
    wheel::browser_view_start_after_wheel(current_view_start, visible_count, viewport_len, steps)
}

pub(super) fn waveform_wheel_zoom_action(
    layout: &ShellLayout,
    model: &AppModel,
    point: Point,
    delta: MouseScrollDelta,
) -> Option<UiAction> {
    wheel::waveform_wheel_zoom_action(layout, model, point, delta)
}
