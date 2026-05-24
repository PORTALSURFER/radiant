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
        let app_animation_active = self
            .lifecycle
            .animation
            .as_mut()
            .is_some_and(|animation| animation(&mut self.state));
        let transient_overlay_animation = self
            .lifecycle
            .transient_overlay_activity
            .as_mut()
            .map_or_else(RuntimeAnimationActivity::idle, |activity| {
                activity(&mut self.state)
            });
        let frame_message_active = app_animation_active && self.lifecycle.frame_message.is_some();
        self.runtime_flags.pending_animation_frame_activity = Some(frame_message_active);
        let app_animation =
            RuntimeAnimationDemand::from_flags(app_animation_active, frame_message_active);
        RuntimeAnimationActivity::from_demand(app_animation).merge(transient_overlay_animation)
    }

    pub(super) fn queue_runtime_animation_frame(&mut self) -> bool {
        let active = self
            .runtime_flags
            .pending_animation_frame_activity
            .take()
            .unwrap_or_else(|| {
                let lifecycle = &mut self.lifecycle;
                lifecycle
                    .animation
                    .as_mut()
                    .is_some_and(|animation| animation(&mut self.state))
                    && lifecycle.frame_message.is_some()
            });
        if active && let Some(frame_message) = self.lifecycle.frame_message.as_mut() {
            return self.runtime.enqueue_frame(frame_message());
        }
        false
    }
}
