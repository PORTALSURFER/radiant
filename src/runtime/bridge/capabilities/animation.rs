use crate::runtime::RuntimeAnimationActivity;

/// Optional host capability for animation demand and frame messages.
pub trait RuntimeAnimationHost {
    /// Return whether the host currently needs animation-driven redraws.
    fn needs_animation(&mut self) -> bool {
        false
    }

    /// Return the kind of animation work currently needed.
    fn animation_activity(&mut self) -> RuntimeAnimationActivity {
        if self.needs_animation() {
            RuntimeAnimationActivity::frame_messages()
        } else {
            RuntimeAnimationActivity::idle()
        }
    }

    /// Queue one host-defined animation-frame message.
    fn queue_animation_frame(&mut self) -> bool {
        false
    }
}

pub(crate) struct RuntimeAnimationCapability<Bridge> {
    pub animation_activity: fn(&mut Bridge) -> RuntimeAnimationActivity,
    pub queue_animation_frame: fn(&mut Bridge) -> bool,
}

impl<Bridge> RuntimeAnimationCapability<Bridge>
where
    Bridge: RuntimeAnimationHost,
{
    pub const fn new() -> Self {
        Self {
            animation_activity: Bridge::animation_activity,
            queue_animation_frame: Bridge::queue_animation_frame,
        }
    }
}
