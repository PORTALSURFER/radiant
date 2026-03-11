/// Active browser-scrollbar thumb drag state while the primary pointer is held.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct BrowserScrollbarDragState {
    pub(super) thumb_pointer_offset_y: f32,
}

/// Active waveform-scrollbar thumb drag state while the primary pointer is held.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct WaveformScrollbarDragState {
    pub(super) thumb_pointer_offset_x: f32,
}

/// Active middle-button waveform pan drag state.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct WaveformPanDragState {
    pub(super) origin_x: f32,
    pub(super) view_start_micros: u32,
    pub(super) view_end_micros: u32,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) enum TextInputTarget {
    #[default]
    None,
    BrowserSearch,
    FolderSearch,
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
