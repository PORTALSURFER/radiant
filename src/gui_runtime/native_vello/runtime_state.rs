/// Active browser-scrollbar thumb drag state while the primary pointer is held.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct BrowserScrollbarDragState {
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
}

/// Exact waveform press state used to preserve click-to-seek precision on release.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct WaveformClickSeekPress {
    pub(super) press_x: f32,
    pub(super) position_micros: u32,
    pub(super) position_nanos: u32,
    pub(super) clear_selection_on_release: bool,
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
    BrowserScrollbar,
    WaveformScrollbar,
    WaveformPan,
    WaveformDrag,
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
    pub(super) fn active_pointer_session(&self) -> ActivePointerSession {
        if self.volume_drag_active {
            ActivePointerSession::Volume
        } else if self.browser_scrollbar_drag.is_some() {
            ActivePointerSession::BrowserScrollbar
        } else if self.waveform_scrollbar_drag.is_some() {
            ActivePointerSession::WaveformScrollbar
        } else if self.waveform_pan_drag.is_some() {
            ActivePointerSession::WaveformPan
        } else if self.waveform_drag_mode.is_some() {
            ActivePointerSession::WaveformDrag
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
        self.clear_pointer_drag_session();
    }

    pub(super) fn clear_pointer_release_state(&mut self) {
        self.text_input_drag_active = false;
        self.browser_scrollbar_drag = None;
        self.waveform_scrollbar_drag = None;
        self.waveform_pan_drag = None;
        self.last_emitted_browser_view_start = None;
        self.last_emitted_waveform_view_center = None;
    }

    pub(super) fn clear_pointer_drag_session(&mut self) {
        self.waveform_drag_mode = None;
        self.waveform_click_seek_press = None;
        self.selection_drag_active = false;
        self.last_emitted_waveform_drag_action = None;
        self.map_focus_drag_active = false;
        self.last_emitted_map_drag_sample_id = None;
        self.browser_scrollbar_drag = None;
        self.last_emitted_browser_view_start = None;
        self.waveform_scrollbar_drag = None;
        self.waveform_pan_drag = None;
        self.last_emitted_waveform_view_center = None;
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
        });
        self.last_emitted_waveform_view_center = None;
    }

    pub(super) fn begin_map_focus_drag(&mut self, sample_id: Option<String>) {
        self.map_focus_drag_active = true;
        self.last_emitted_map_drag_sample_id = sample_id;
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
