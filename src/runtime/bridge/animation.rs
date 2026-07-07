#[cfg(test)]
#[path = "animation/tests.rs"]
mod tests;

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
    frame_message_target_fps: Option<u32>,
}

/// Named animation demand used to construct [`RuntimeAnimationActivity`].
///
/// This keeps call sites from encoding the runtime policy as a pair of boolean
/// flags, while preserving the compact compatibility constructor for existing
/// custom bridges.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum RuntimeAnimationDemand {
    /// No animation-driven frames are needed.
    #[default]
    Idle,
    /// Present paint-only frames over the cached surface.
    PaintOnly,
    /// Queue host frame messages before presenting animation frames.
    FrameMessages,
}

impl RuntimeAnimationActivity {
    /// Return an inactive animation state.
    pub const fn idle() -> Self {
        Self {
            paint_frames: false,
            frame_messages: false,
            target_fps: None,
            frame_message_target_fps: None,
        }
    }

    /// Return an animation state that only needs paint-only redraws.
    pub const fn paint_only() -> Self {
        Self {
            paint_frames: true,
            frame_messages: false,
            target_fps: None,
            frame_message_target_fps: None,
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
            frame_message_target_fps: None,
        }
    }

    /// Return an animation state that should queue host frame messages.
    pub const fn frame_messages() -> Self {
        Self {
            paint_frames: true,
            frame_messages: true,
            target_fps: None,
            frame_message_target_fps: None,
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
            frame_message_target_fps: Some(target_fps),
        }
    }

    /// Build animation activity from a named demand.
    pub const fn from_demand(demand: RuntimeAnimationDemand) -> Self {
        match demand {
            RuntimeAnimationDemand::Idle => Self::idle(),
            RuntimeAnimationDemand::PaintOnly => Self::paint_only(),
            RuntimeAnimationDemand::FrameMessages => Self::frame_messages(),
        }
    }

    /// Build animation activity from explicit paint and message demands.
    ///
    /// Prefer [`Self::from_demand`] for new code so call sites name the policy
    /// they intend instead of passing a pair of related booleans.
    pub const fn new(paint_frames: bool, frame_messages: bool) -> Self {
        Self::from_demand(RuntimeAnimationDemand::from_flags(
            paint_frames,
            frame_messages,
        ))
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
            frame_message_target_fps: merge_frame_message_target_fps(self, other),
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

    /// Return the requested frame-message rate, if frame messages are active.
    pub const fn frame_message_target_fps(self) -> Option<u32> {
        if self.frame_messages {
            self.frame_message_target_fps
        } else {
            None
        }
    }
}

impl RuntimeAnimationDemand {
    pub(crate) const fn from_flags(paint_frames: bool, frame_messages: bool) -> Self {
        match (paint_frames, frame_messages) {
            (false, _) => Self::Idle,
            (true, false) => Self::PaintOnly,
            (true, true) => Self::FrameMessages,
        }
    }
}

const fn merge_frame_message_target_fps(
    left: RuntimeAnimationActivity,
    right: RuntimeAnimationActivity,
) -> Option<u32> {
    match (
        left.frame_message_target_fps(),
        right.frame_message_target_fps(),
    ) {
        (None, None) if left.frame_messages && right.frame_messages => None,
        (None, None) => None,
        (Some(_), None) if right.frame_messages => None,
        (None, Some(_)) if left.frame_messages => None,
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
