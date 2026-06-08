use super::super::AppBridge;
use crate::{
    application::{IntoView, UpdateContext},
    runtime::{RuntimeAnimationActivity, RuntimeAnimationDemand},
};

impl<State, Message, Project, Update, View> AppBridge<State, Message, Project, Update, View>
where
    Project: FnMut(&mut State) -> View + 'static,
    Update: FnMut(&mut State, Message, &mut UpdateContext<Message>) + 'static,
    View: IntoView<Message> + 'static,
{
    pub(super) fn needs_runtime_animation(&mut self) -> bool {
        self.runtime_animation_activity().needs_animation()
    }

    pub(super) fn runtime_animation_activity(&mut self) -> RuntimeAnimationActivity {
        let legacy_animation_active = self
            .lifecycle
            .animation
            .as_mut()
            .is_some_and(|animation| animation(&mut self.state));
        let frame_clock_animation = self
            .lifecycle
            .frame_clock_activity
            .as_mut()
            .map_or_else(RuntimeAnimationActivity::idle, |activity| {
                activity(&mut self.state)
            });
        let transient_overlay_animation = self
            .lifecycle
            .transient_overlay_activity
            .as_mut()
            .map_or_else(RuntimeAnimationActivity::idle, |activity| {
                activity(&mut self.state)
            });
        let legacy_frame_message_active =
            legacy_animation_active && self.lifecycle.frame_message.is_some();
        let frame_message_active =
            legacy_frame_message_active || frame_clock_animation.needs_frame_message();
        self.runtime_flags.pending_animation_frame_activity = Some(frame_message_active);
        let legacy_animation = RuntimeAnimationDemand::from_flags(
            legacy_animation_active,
            legacy_frame_message_active,
        );
        RuntimeAnimationActivity::from_demand(legacy_animation)
            .merge(frame_clock_animation)
            .merge(transient_overlay_animation)
    }

    pub(super) fn queue_runtime_animation_frame(&mut self) -> bool {
        let active = self
            .runtime_flags
            .pending_animation_frame_activity
            .take()
            .unwrap_or_else(|| self.poll_frame_message_activity());
        if active && let Some(frame_message) = self.lifecycle.frame_message.as_mut() {
            let queued = self.runtime.enqueue_frame(frame_message());
            if queued {
                self.capture_frame_repaint_scope();
            }
            return queued;
        }
        false
    }

    fn poll_frame_message_activity(&mut self) -> bool {
        let legacy_active = self
            .lifecycle
            .animation
            .as_mut()
            .is_some_and(|animation| animation(&mut self.state))
            && self.lifecycle.frame_message.is_some();
        let frame_clock_active = self
            .lifecycle
            .frame_clock_activity
            .as_mut()
            .is_some_and(|activity| activity(&mut self.state).needs_frame_message());
        legacy_active || frame_clock_active
    }

    fn capture_frame_repaint_scope(&mut self) {
        let Some(policy) = self.lifecycle.frame_repaint_policy.as_mut() else {
            return;
        };
        self.runtime_flags.pending_frame_repaint_scope =
            Some(policy.capture_before_frame(&mut self.state));
    }
}
