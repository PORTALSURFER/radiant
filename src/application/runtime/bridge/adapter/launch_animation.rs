use super::super::AppBridgeLifecycle;
use crate::runtime::{RuntimeAnimationActivity, RuntimeAnimationDemand};

/// Activity reported by the advanced launch-level animation hooks.
///
/// Scene and `Presentation` frame clocks are the normal presentation path. This
/// adapter keeps the lower-level launch hooks isolated from the main frame-clock
/// demand flow while preserving their public compatibility behavior.
pub(super) struct LaunchAnimationActivity {
    runtime_activity: RuntimeAnimationActivity,
}

impl LaunchAnimationActivity {
    pub(super) fn into_runtime_activity(self) -> RuntimeAnimationActivity {
        self.runtime_activity
    }
}

pub(super) fn poll_launch_animation_activity<State, Message>(
    lifecycle: &mut AppBridgeLifecycle<State, Message>,
    state: &mut State,
) -> LaunchAnimationActivity {
    let active = lifecycle
        .animation
        .as_mut()
        .is_some_and(|animation| animation(state));
    let frame_message_active = active && lifecycle.frame_message.is_some();
    LaunchAnimationActivity {
        runtime_activity: RuntimeAnimationActivity::from_demand(
            RuntimeAnimationDemand::from_flags(active, frame_message_active),
        ),
    }
}

pub(super) fn poll_launch_frame_message_activity<State, Message>(
    lifecycle: &mut AppBridgeLifecycle<State, Message>,
    state: &mut State,
) -> bool {
    lifecycle
        .animation
        .as_mut()
        .is_some_and(|animation| animation(state))
        && lifecycle.frame_message.is_some()
}
