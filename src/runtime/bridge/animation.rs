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

    /// Combine two animation demands into one runtime-visible activity.
    ///
    /// This keeps frame-message and paint-only animation sources on the same
    /// scheduling contract. If either active source is uncapped, the combined
    /// activity is uncapped; otherwise the faster requested cadence is
    /// preserved.
    pub const fn merge(self, other: Self) -> Self {
        let paint_frames = self.paint_frames || other.paint_frames;
        let frame_messages = self.frame_messages || other.frame_messages;
        Self {
            paint_frames,
            frame_messages: paint_frames && frame_messages,
            target_fps: merge_target_fps(self, other),
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

    /// Return the requested animation frame rate, if this activity is active.
    pub const fn target_fps(self) -> Option<u32> {
        if self.paint_frames {
            self.target_fps
        } else {
            None
        }
    }
}

const fn merge_target_fps(
    left: RuntimeAnimationActivity,
    right: RuntimeAnimationActivity,
) -> Option<u32> {
    match (left.target_fps(), right.target_fps()) {
        (None, None) if left.paint_frames && right.paint_frames => None,
        (None, None) => None,
        (Some(_), None) if right.paint_frames => None,
        (None, Some(_)) if left.paint_frames => None,
        (Some(fps), None) | (None, Some(fps)) => Some(fps),
        (Some(left), Some(right)) => {
            if left >= right {
                Some(left)
            } else {
                Some(right)
            }
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

    #[test]
    fn runtime_animation_activity_merges_message_and_paint_demands() {
        let activity =
            RuntimeAnimationActivity::frame_messages_at(24).merge(RuntimeAnimationActivity::idle());

        assert!(activity.needs_animation());
        assert!(activity.needs_frame_message());
        assert_eq!(activity.target_fps(), Some(24));
    }

    #[test]
    fn runtime_animation_activity_merge_preserves_fastest_capped_source() {
        let activity = RuntimeAnimationActivity::paint_only_at(24)
            .merge(RuntimeAnimationActivity::frame_messages_at(60));

        assert!(activity.needs_animation());
        assert!(activity.needs_frame_message());
        assert_eq!(activity.target_fps(), Some(60));
    }

    #[test]
    fn runtime_animation_activity_merge_keeps_uncapped_source_uncapped() {
        let activity = RuntimeAnimationActivity::paint_only_at(24)
            .merge(RuntimeAnimationActivity::frame_messages());

        assert!(activity.needs_animation());
        assert!(activity.needs_frame_message());
        assert_eq!(activity.target_fps(), None);
    }
}
