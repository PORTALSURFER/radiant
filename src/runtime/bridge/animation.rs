/// Runtime-visible animation demand for the next timed frame.
///
/// Frame-message animation mutates host state through
/// [`crate::runtime::RuntimeBridge::queue_animation_frame`]. Paint-only
/// animation only needs another presentation over the cached surface.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RuntimeAnimationActivity {
    paint_frames: bool,
    frame_messages: bool,
}

impl RuntimeAnimationActivity {
    /// Return an inactive animation state.
    pub const fn idle() -> Self {
        Self {
            paint_frames: false,
            frame_messages: false,
        }
    }

    /// Return an animation state that only needs paint-only redraws.
    pub const fn paint_only() -> Self {
        Self {
            paint_frames: true,
            frame_messages: false,
        }
    }

    /// Return an animation state that should queue host frame messages.
    pub const fn frame_messages() -> Self {
        Self {
            paint_frames: true,
            frame_messages: true,
        }
    }

    /// Build animation activity from explicit paint and message demands.
    pub const fn new(paint_frames: bool, frame_messages: bool) -> Self {
        Self {
            paint_frames,
            frame_messages: paint_frames && frame_messages,
        }
    }

    /// Return whether any animation-driven presentation is currently needed.
    pub const fn needs_animation(self) -> bool {
        self.paint_frames
    }

    /// Return whether the next timed frame should enqueue a host frame message.
    pub const fn needs_frame_message(self) -> bool {
        self.frame_messages
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
}
