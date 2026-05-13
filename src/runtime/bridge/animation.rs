/// Runtime-visible animation demand for the next timed frame.
///
/// Frame-message animation mutates host state through
/// [`crate::runtime::RuntimeBridge::queue_animation_frame`]. Paint-only
/// animation only needs another presentation over the cached surface.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RuntimeAnimationActivity {
    paint_frames: bool,
    frame_messages: bool,
    target_fps: Option<u32>,
}

impl RuntimeAnimationActivity {
    /// Return an inactive animation state.
    pub const fn idle() -> Self {
        Self {
            paint_frames: false,
            frame_messages: false,
            target_fps: None,
        }
    }

    /// Return an animation state that only needs paint-only redraws.
    pub const fn paint_only() -> Self {
        Self {
            paint_frames: true,
            frame_messages: false,
            target_fps: None,
        }
    }

    /// Return paint-only animation capped to the requested frame rate.
    ///
    /// Native runtimes clamp this value through their normal target-FPS policy
    /// and never exceed the window-level frame-rate cap.
    pub const fn paint_only_at(target_fps: u32) -> Self {
        Self {
            paint_frames: true,
            frame_messages: false,
            target_fps: Some(target_fps),
        }
    }

    /// Return an animation state that should queue host frame messages.
    pub const fn frame_messages() -> Self {
        Self {
            paint_frames: true,
            frame_messages: true,
            target_fps: None,
        }
    }

    /// Return frame-message animation capped to the requested frame rate.
    ///
    /// Use this for host-state animation that should tick below the native
    /// window's maximum frame cadence.
    pub const fn frame_messages_at(target_fps: u32) -> Self {
        Self {
            paint_frames: true,
            frame_messages: true,
            target_fps: Some(target_fps),
        }
    }

    /// Build animation activity from explicit paint and message demands.
    pub const fn new(paint_frames: bool, frame_messages: bool) -> Self {
        Self {
            paint_frames,
            frame_messages: paint_frames && frame_messages,
            target_fps: None,
        }
    }

    /// Cap active animation to the requested frame rate.
    ///
    /// Calling this on [`Self::idle`] preserves the idle state but records no
    /// frame-rate demand.
    pub const fn with_target_fps(mut self, target_fps: u32) -> Self {
        if self.paint_frames {
            self.target_fps = Some(target_fps);
        }
        self
    }

    /// Return whether any animation-driven presentation is currently needed.
    pub const fn needs_animation(self) -> bool {
        self.paint_frames
    }

    /// Return whether the next timed frame should enqueue a host frame message.
    pub const fn needs_frame_message(self) -> bool {
        self.frame_messages
    }

    /// Return the requested animation frame rate, if this activity is active.
    pub const fn target_fps(self) -> Option<u32> {
        if self.paint_frames {
            self.target_fps
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runtime_animation_activity_keeps_frame_messages_bound_to_paint_frames() {
        assert!(!RuntimeAnimationActivity::idle().needs_animation());
        assert!(!RuntimeAnimationActivity::idle().needs_frame_message());
        assert!(RuntimeAnimationActivity::paint_only().needs_animation());
        assert!(!RuntimeAnimationActivity::paint_only().needs_frame_message());
        assert!(RuntimeAnimationActivity::frame_messages().needs_animation());
        assert!(RuntimeAnimationActivity::frame_messages().needs_frame_message());
        assert!(!RuntimeAnimationActivity::new(false, true).needs_frame_message());
    }

    #[test]
    fn runtime_animation_activity_carries_optional_frame_rate_policy() {
        assert_eq!(RuntimeAnimationActivity::idle().target_fps(), None);
        assert_eq!(
            RuntimeAnimationActivity::paint_only_at(24).target_fps(),
            Some(24)
        );
        assert_eq!(
            RuntimeAnimationActivity::frame_messages_at(30).target_fps(),
            Some(30)
        );
        assert_eq!(
            RuntimeAnimationActivity::idle()
                .with_target_fps(60)
                .target_fps(),
            None
        );
    }
}
