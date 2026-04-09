use super::SingleLineTextEditorState;
use crate::gui::native_shell::TextFieldVisualState;
use crate::gui::types::Point;

/// Active browser-scrollbar thumb drag state while the primary pointer is held.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct BrowserScrollbarDragState {
    pub(super) thumb_pointer_offset_y: f32,
}

/// Active folder-scrollbar thumb drag state while the primary pointer is held.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct FolderScrollbarDragState {
    pub(super) pane: crate::app::FolderPaneIdModel,
    pub(super) thumb_pointer_offset_y: f32,
}

/// Active waveform-scrollbar thumb drag state while the primary pointer is held.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct WaveformScrollbarDragState {
    pub(super) thumb_pointer_offset_x: f32,
    pub(super) thumb_pointer_ratio_x: f32,
}

/// Active middle-button waveform pan drag state.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct WaveformPanDragState {
    pub(super) origin_x: f32,
    pub(super) view_start_micros: u32,
    pub(super) view_end_micros: u32,
    pub(super) view_start_nanos: u32,
    pub(super) view_end_nanos: u32,
}

/// Exact waveform press state used to preserve click-to-seek precision on release.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct WaveformClickSeekPress {
    pub(super) press_x: f32,
    pub(super) position_micros: u32,
    pub(super) position_nanos: u32,
    pub(super) clear_selection_on_release: bool,
}

/// Deferred browser-row press used to preserve click behavior until release.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct PendingBrowserRowPress {
    pub(super) action: crate::app::UiAction,
    pub(super) visible_row: usize,
    pub(super) press_point: Point,
}

/// Active browser-sample drag session while the primary pointer is held.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct BrowserSampleDragState {
    pub(super) visible_row: usize,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) enum TextInputTarget {
    #[default]
    None,
    BrowserSearch,
    FolderSearch,
    FolderCreate,
    PromptInput,
    WaveformBpm,
}

#[derive(Clone, Debug, PartialEq)]
pub(super) struct ActiveTextFieldVisualCacheEntry {
    pub(super) target: TextInputTarget,
    pub(super) text: String,
    pub(super) editor: SingleLineTextEditorState,
    pub(super) font_size_bits: u32,
    pub(super) available_width_bits: u32,
    pub(super) visual: TextFieldVisualState,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum RuntimeInvalidationScope {
    OverlayStateOnly,
    OverlayMotionOnly,
    ModelAndOverlays,
    StaticAndOverlays,
    LayoutAndAll,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ActivePointerSession {
    Volume,
    FolderScrollbar,
    BrowserScrollbar,
    WaveformScrollbar,
    WaveformPan,
    WaveformDrag,
    BrowserSampleDrag,
    SelectionDrag,
    MapFocusDrag,
    TextInputDrag,
    Hover,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) struct NativeVelloFrameState {
    pub(crate) layout_dirty: bool,
    pub(crate) scene_dirty: bool,
    pub(crate) state_overlay_dirty: bool,
    pub(crate) motion_overlay_dirty: bool,
    pub(crate) model_dirty: bool,
}

impl NativeVelloFrameState {
    pub(super) fn mark_layout_dirty(&mut self) {
        self.layout_dirty = true;
        self.scene_dirty = true;
        self.state_overlay_dirty = true;
        self.motion_overlay_dirty = true;
    }

    pub(super) fn mark_state_overlay_dirty(&mut self) {
        self.state_overlay_dirty = true;
    }
    pub(super) fn mark_motion_overlay_dirty(&mut self) {
        self.motion_overlay_dirty = true;
    }
    pub(super) fn clear_layout_dirty(&mut self) {
        self.layout_dirty = false;
    }

    pub(super) fn mark_model_dirty(&mut self) {
        self.model_dirty = true;
        self.scene_dirty = true;
        self.state_overlay_dirty = true;
        self.motion_overlay_dirty = true;
    }

    pub(super) fn mark_model_overlay_dirty(&mut self) {
        self.model_dirty = true;
        self.state_overlay_dirty = true;
        self.motion_overlay_dirty = true;
    }

    pub(super) fn take_scene(&mut self) -> bool {
        let dirty = self.scene_dirty;
        self.scene_dirty = false;
        dirty
    }

    pub(super) fn take_state_overlay(&mut self) -> bool {
        let dirty = self.state_overlay_dirty;
        self.state_overlay_dirty = false;
        dirty
    }

    pub(super) fn take_motion_overlay(&mut self) -> bool {
        let dirty = self.motion_overlay_dirty;
        self.motion_overlay_dirty = false;
        dirty
    }

    pub(super) fn take_model(&mut self) -> bool {
        let dirty = self.model_dirty;
        self.model_dirty = false;
        dirty
    }

    pub(super) fn has_pending_rebuild(&self) -> bool {
        self.layout_dirty
            || self.scene_dirty
            || self.state_overlay_dirty
            || self.motion_overlay_dirty
            || self.model_dirty
    }
}

impl<Bridge> super::NativeVelloRunner<Bridge>
where
    Bridge: crate::app::NativeAppBridge,
{
    pub(super) fn has_external_drag_candidate(&self) -> bool {
        self.browser_sample_drag.is_some() || self.selection_drag_active
    }

    pub(super) fn maybe_launch_external_drag_session(
        &mut self,
        pointer_outside: bool,
        pointer_left: bool,
    ) -> bool {
        #[cfg(target_os = "windows")]
        {
            if self.has_external_drag_candidate() {
                return self
                    .bridge
                    .maybe_launch_external_drag(pointer_outside, pointer_left);
            }
        }
        let _ = pointer_outside;
        let _ = pointer_left;
        false
    }

    #[cfg(target_os = "windows")]
    pub(super) fn poll_external_drag_window_state(&self) -> Option<(bool, bool)> {
        use raw_window_handle::{HasWindowHandle, RawWindowHandle};
        use windows_sys::Win32::Foundation::{POINT, RECT};
        use windows_sys::Win32::UI::WindowsAndMessaging::{GetCursorPos, GetWindowRect};

        if !self.has_external_drag_candidate() {
            return None;
        }
        let window = self.window.as_ref()?;
        let handle = window.window_handle().ok()?;
        let RawWindowHandle::Win32(handle) = handle.as_raw() else {
            return None;
        };
        let hwnd = handle.hwnd.get();
        let mut cursor = POINT { x: 0, y: 0 };
        if unsafe { GetCursorPos(&mut cursor) } == 0 {
            return None;
        }
        let mut rect = RECT {
            left: 0,
            top: 0,
            right: 0,
            bottom: 0,
        };
        if unsafe { GetWindowRect(hwnd as *mut _, &mut rect) } == 0 {
            return None;
        }
        let inside = cursor.x >= rect.left
            && cursor.x < rect.right
            && cursor.y >= rect.top
            && cursor.y < rect.bottom;
        Some((!inside, !inside))
    }

    pub(super) fn active_pointer_session(&self) -> ActivePointerSession {
        if self.volume_drag_active {
            ActivePointerSession::Volume
        } else if self.folder_scrollbar_drag.is_some() {
            ActivePointerSession::FolderScrollbar
        } else if self.browser_scrollbar_drag.is_some() {
            ActivePointerSession::BrowserScrollbar
        } else if self.waveform_scrollbar_drag.is_some() {
            ActivePointerSession::WaveformScrollbar
        } else if self.waveform_pan_drag.is_some() {
            ActivePointerSession::WaveformPan
        } else if self.waveform_drag_mode.is_some() {
            ActivePointerSession::WaveformDrag
        } else if self.browser_sample_drag.is_some() {
            ActivePointerSession::BrowserSampleDrag
        } else if self.selection_drag_active {
            ActivePointerSession::SelectionDrag
        } else if self.map_focus_drag_active {
            ActivePointerSession::MapFocusDrag
        } else if self.text_input_drag_active {
            ActivePointerSession::TextInputDrag
        } else {
            ActivePointerSession::Hover
        }
    }

    pub(super) fn begin_pointer_press_cycle(&mut self) {
        self.pending_volume_milli = None;
        self.volume_drag_active = false;
        self.last_emitted_volume_milli = None;
        self.pending_browser_row_press = None;
        self.clear_pointer_drag_session();
    }

    pub(super) fn clear_pointer_release_state(&mut self) {
        self.text_input_drag_active = false;
        self.folder_scrollbar_drag = None;
        self.browser_scrollbar_drag = None;
        self.waveform_scrollbar_drag = None;
        self.waveform_pan_drag = None;
        self.last_emitted_browser_view_start = None;
        self.last_emitted_waveform_view_center = None;
    }

    pub(super) fn clear_pointer_drag_session(&mut self) {
        self.waveform_drag_mode = None;
        self.waveform_click_seek_press = None;
        self.browser_sample_drag = None;
        self.selection_drag_active = false;
        self.last_emitted_waveform_drag_action = None;
        self.map_focus_drag_active = false;
        self.last_emitted_map_drag_sample_id = None;
        self.folder_scrollbar_drag = None;
        self.browser_scrollbar_drag = None;
        self.last_emitted_browser_view_start = None;
        self.waveform_scrollbar_drag = None;
        self.waveform_pan_drag = None;
        self.last_emitted_waveform_view_center = None;
    }

    pub(super) fn begin_folder_scrollbar_drag(
        &mut self,
        pane: crate::app::FolderPaneIdModel,
        thumb_pointer_offset_y: f32,
    ) {
        self.folder_scrollbar_drag = Some(FolderScrollbarDragState {
            pane,
            thumb_pointer_offset_y,
        });
    }

    pub(super) fn begin_browser_scrollbar_drag(&mut self, thumb_pointer_offset_y: f32) {
        self.browser_scrollbar_drag = Some(BrowserScrollbarDragState {
            thumb_pointer_offset_y,
        });
        self.last_emitted_browser_view_start = None;
    }

    pub(super) fn begin_waveform_scrollbar_drag(
        &mut self,
        thumb_pointer_offset_x: f32,
        thumb_pointer_ratio_x: f32,
    ) {
        self.waveform_scrollbar_drag = Some(WaveformScrollbarDragState {
            thumb_pointer_offset_x,
            thumb_pointer_ratio_x,
        });
        self.last_emitted_waveform_view_center = None;
    }

    pub(super) fn begin_waveform_pan_drag(&mut self, origin_x: f32) {
        self.refresh_waveform_view_if_needed();
        self.waveform_pan_drag = Some(WaveformPanDragState {
            origin_x,
            view_start_micros: self.model.waveform.view_start_micros,
            view_end_micros: self.model.waveform.view_end_micros,
            view_start_nanos: self.model.waveform.view_start_nanos,
            view_end_nanos: self.model.waveform.view_end_nanos,
        });
        self.last_emitted_waveform_view_center = None;
    }

    pub(super) fn begin_map_focus_drag(&mut self, sample_id: Option<String>) {
        self.map_focus_drag_active = true;
        self.last_emitted_map_drag_sample_id = sample_id;
    }

    pub(super) fn begin_browser_sample_drag(&mut self, visible_row: usize) {
        self.browser_sample_drag = Some(BrowserSampleDragState { visible_row });
    }

    pub(super) fn begin_waveform_pointer_interaction(
        &mut self,
        action: &crate::app::UiAction,
        click_seek_press: Option<WaveformClickSeekPress>,
    ) {
        self.waveform_drag_mode =
            super::input::waveform_drag_mode_for_action(action).or_else(|| {
                click_seek_press.and_then(|press| {
                    matches!(
                        action,
                        crate::app::UiAction::ClearWaveformSelection
                            | crate::app::UiAction::ClearWaveformEditSelection
                            | crate::app::UiAction::ClearWaveformSelections
                    )
                    .then_some(
                        super::input::WaveformPointerDragMode::Selection {
                            anchor_micros: press.position_micros,
                            boundary_lock: None,
                        },
                    )
                })
            });
        self.waveform_click_seek_press = click_seek_press;
        if self.waveform_drag_mode.is_some() {
            self.shell_state.clear_waveform_hover_feedback();
        }
        self.selection_drag_active = matches!(
            action,
            crate::app::UiAction::StartWaveformSelectionDrag { .. }
        );
    }
}
