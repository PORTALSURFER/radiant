use super::super::AppBridge;
use super::launch_animation;
use crate::{
    application::{
        FrameMessageActivity, FrameRepaintSource, IntoView, PendingFrameRepaint, UiUpdateContext,
    },
    runtime::RuntimeAnimationActivity,
};
use std::time::{Duration, Instant};

impl<State, Message, Project, Update, View> AppBridge<State, Message, Project, Update, View>
where
    Project: FnMut(&State) -> View + 'static,
    Update: FnMut(&mut State, Message, &mut UiUpdateContext<Message>) + 'static,
    View: IntoView<Message> + 'static,
{
    pub(super) fn needs_runtime_animation(&mut self) -> bool {
        self.runtime_animation_activity().needs_animation()
    }

    pub(super) fn runtime_animation_activity(&mut self) -> RuntimeAnimationActivity {
        let launch_animation =
            launch_animation::poll_launch_animation_activity(&mut self.lifecycle, &mut self.state);
        let frame_clock_animation = self
            .lifecycle
            .frame_clock_activity
            .as_mut()
            .map_or_else(RuntimeAnimationActivity::idle, |activity| {
                activity(&mut self.state)
            });
        let scene_frame_clock_animation = self
            .lifecycle
            .scene_frame_clock_activity
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
        let scene_transient_overlay_animation = self
            .lifecycle
            .scene_transient_overlay_activity
            .as_mut()
            .map_or_else(RuntimeAnimationActivity::idle, |activity| {
                activity(&mut self.state)
            });
        let launch_animation = launch_animation.into_runtime_activity();
        let app_frame_message_animation = launch_animation.merge(frame_clock_animation);
        let app_frame_message_active = app_frame_message_animation.needs_frame_message();
        let scene_frame_message_active = scene_frame_clock_animation.needs_frame_message()
            && self.lifecycle.scene_frame_message.is_some();
        self.runtime_flags.pending_animation_frame_activity = Some(FrameMessageActivity {
            app: app_frame_message_active,
            app_target_fps: app_frame_message_animation.frame_message_target_fps(),
            scene: scene_frame_message_active,
            scene_target_fps: scene_frame_clock_animation.frame_message_target_fps(),
        });
        launch_animation
            .merge(frame_clock_animation)
            .merge(scene_frame_clock_animation)
            .merge(transient_overlay_animation)
            .merge(scene_transient_overlay_animation)
    }

    pub(super) fn queue_runtime_animation_frame(&mut self) -> bool {
        let active = self
            .runtime_flags
            .pending_animation_frame_activity
            .take()
            .unwrap_or_else(|| self.poll_frame_message_activity());
        let now = Instant::now();
        let scene_due = active.scene
            && frame_message_due(
                now,
                self.runtime_flags.last_scene_frame_message_at,
                active.scene_target_fps,
            );
        if scene_due && let Some(frame_message) = self.lifecycle.scene_frame_message.as_mut() {
            let queued = self.runtime.enqueue_frame(frame_message());
            if queued {
                self.runtime_flags.last_scene_frame_message_at = Some(now);
                self.capture_scene_frame_repaint();
            }
            return queued;
        }
        let app_due = active.app
            && frame_message_due(
                now,
                self.runtime_flags.last_app_frame_message_at,
                active.app_target_fps,
            );
        if app_due && let Some(frame_message) = self.lifecycle.frame_message.as_mut() {
            let queued = self.runtime.enqueue_frame(frame_message());
            if queued {
                self.runtime_flags.last_app_frame_message_at = Some(now);
                self.capture_app_frame_repaint();
            }
            return queued;
        }
        false
    }

    fn poll_frame_message_activity(&mut self) -> FrameMessageActivity {
        let launch_active = launch_animation::poll_launch_frame_message_activity(
            &mut self.lifecycle,
            &mut self.state,
        );
        let frame_clock_animation = self
            .lifecycle
            .frame_clock_activity
            .as_mut()
            .map_or_else(RuntimeAnimationActivity::idle, |activity| {
                activity(&mut self.state)
            });
        let scene_animation = self
            .lifecycle
            .scene_frame_clock_activity
            .as_mut()
            .map_or_else(RuntimeAnimationActivity::idle, |activity| {
                activity(&mut self.state)
            });
        let frame_clock_active = frame_clock_animation.needs_frame_message();
        let scene_active =
            scene_animation.needs_frame_message() && self.lifecycle.scene_frame_message.is_some();
        FrameMessageActivity {
            app: launch_active || frame_clock_active,
            app_target_fps: if launch_active {
                None
            } else {
                frame_clock_animation.frame_message_target_fps()
            },
            scene: scene_active,
            scene_target_fps: scene_animation.frame_message_target_fps(),
        }
    }

    fn capture_app_frame_repaint(&mut self) {
        let scope = self
            .lifecycle
            .frame_repaint_policy
            .as_mut()
            .map(|policy| policy.capture_before_frame(&mut self.state));
        self.runtime_flags.pending_frame_repaint = Some(PendingFrameRepaint {
            source: FrameRepaintSource::App,
            scope,
        });
    }

    fn capture_scene_frame_repaint(&mut self) {
        let scope = self
            .lifecycle
            .scene_frame_repaint_policy
            .as_mut()
            .map(|policy| policy.capture_before_frame(&mut self.state));
        self.runtime_flags.pending_frame_repaint = Some(PendingFrameRepaint {
            source: FrameRepaintSource::Scene,
            scope,
        });
    }
}

fn frame_message_due(
    now: Instant,
    last_frame_message_at: Option<Instant>,
    target_fps: Option<u32>,
) -> bool {
    let Some(target_fps) = target_fps else {
        return true;
    };
    let Some(last_frame_message_at) = last_frame_message_at else {
        return true;
    };
    now.duration_since(last_frame_message_at) >= frame_message_interval(target_fps)
}

fn frame_message_interval(target_fps: u32) -> Duration {
    Duration::from_secs_f64(1.0 / f64::from(target_fps.max(1)))
}
