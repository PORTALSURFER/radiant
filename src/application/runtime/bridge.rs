use super::subscription::spawn_subscription;
use super::{
    AppAnimation, AppAuxiliaryWindows, AppCloseRequested, AppFrameClockActivity, AppFrameMessage,
    AppFrameRepaintPolicy, AppNativeFileDrop, AppNativeFileOpen, AppRuntime, AppScroll,
    AppShortcuts, AppShutdown, AppStartup, AppSubscriptions, RetainedPainter,
    TransientOverlayActivity, TransientOverlayPainter, UiUpdateContext,
};
use crate::{
    application::{IntoView, RepaintPolicy},
    gui::{input::KeyPress, shortcuts::ShortcutResolution},
    runtime::{Command, RepaintScope},
};
use crate::runtime::RuntimeUpdateSnapshot;
use std::{any::Any, collections::HashMap, marker::PhantomData, sync::Arc};

mod adapter;

pub(in crate::application) struct AppBridge<State, Message, Project, Update, View> {
    pub(in crate::application) state: State,
    pub(in crate::application) project: Project,
    pub(in crate::application) update: Update,
    pub(in crate::application) runtime: Arc<AppRuntime<Message>>,
    pub(in crate::application) lifecycle: AppBridgeLifecycle<State, Message>,
    pub(in crate::application) runtime_flags: AppBridgeRuntimeFlags,
    pub(in crate::application) _view: PhantomData<View>,
}

#[derive(Default)]
pub(in crate::application) struct AppBridgeRuntimeFlags {
    /// Cached animation-frame activity from the latest animation poll.
    pub(in crate::application) pending_animation_frame_activity: Option<FrameMessageActivity>,
    /// Origin and optional captured state for the currently queued frame message.
    pub(in crate::application) pending_frame_repaint: Option<PendingFrameRepaint>,
    /// Whether app subscriptions have been installed for this bridge.
    pub(in crate::application) subscriptions_started: bool,
    /// Whether the startup hook has already run for this bridge.
    pub(in crate::application) startup_ran: bool,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(in crate::application) struct FrameMessageActivity {
    pub(in crate::application) app: bool,
    pub(in crate::application) scene: bool,
}

pub(in crate::application) struct PendingFrameRepaint {
    pub(in crate::application) source: FrameRepaintSource,
    pub(in crate::application) scope: Option<Box<dyn Any>>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::application) enum FrameRepaintSource {
    App,
    Scene,
}

/// Lifecycle hooks carried from application launch builders into the runtime bridge.
pub(in crate::application) struct AppBridgeLifecycle<State, Message> {
    /// Animation activity callback.
    pub(in crate::application) animation: Option<AppAnimation<State>>,
    /// Frame-message callback.
    pub(in crate::application) frame_message: Option<AppFrameMessage<Message>>,
    /// Typed frame-clock activity callback.
    pub(in crate::application) frame_clock_activity: Option<AppFrameClockActivity<State>>,
    /// Optional frame-message repaint policy.
    pub(in crate::application) frame_repaint_policy: Option<Box<dyn AppFrameRepaintPolicy<State>>>,
    /// Scene-declared frame-message callback.
    pub(in crate::application) scene_frame_message: Option<AppFrameMessage<Message>>,
    /// Scene-declared frame-clock activity callback.
    pub(in crate::application) scene_frame_clock_activity: Option<AppFrameClockActivity<State>>,
    /// Scene-declared frame-message repaint policy.
    pub(in crate::application) scene_frame_repaint_policy:
        Option<Box<dyn AppFrameRepaintPolicy<State>>>,
    /// Automatic repaint behavior for ordinary app messages.
    pub(in crate::application) repaint_policy: Option<RepaintPolicy<Message>>,
    /// App-level subscription factory.
    pub(in crate::application) subscriptions: Option<AppSubscriptions<State, Message>>,
    /// App-level shortcut resolver.
    pub(in crate::application) shortcuts: Option<AppShortcuts<State, Message>>,
    /// Scene-declared shortcut catalog.
    pub(in crate::application) scene_shortcuts:
        Option<Box<dyn Fn(KeyPress) -> ShortcutResolution<Message>>>,
    /// Runtime scroll observer.
    pub(in crate::application) scroll: Option<AppScroll<State, Message>>,
    /// Native drag/drop observer.
    pub(in crate::application) native_file_drop: Option<AppNativeFileDrop<State, Message>>,
    /// Native file-open observer.
    pub(in crate::application) native_file_open: Option<AppNativeFileOpen<State, Message>>,
    /// Startup hook.
    pub(in crate::application) startup: Option<AppStartup<State, Message>>,
    /// Shutdown artifact hook.
    pub(in crate::application) shutdown: Option<AppShutdown<State>>,
    /// Close-request hook.
    pub(in crate::application) close_requested: Option<AppCloseRequested<State>>,
    /// Auxiliary top-level native windows projected from app state.
    pub(in crate::application) auxiliary_windows: Option<AppAuxiliaryWindows<State, Message>>,
    /// Retained-surface painters keyed by descriptor key.
    pub(in crate::application) retained_painters: HashMap<u64, RetainedPainter<State>>,
    /// Transient-overlay frame activity callback.
    pub(in crate::application) transient_overlay_activity: Option<TransientOverlayActivity<State>>,
    /// Lightweight frame-time overlay painter.
    pub(in crate::application) transient_overlay: Option<TransientOverlayPainter<State>>,
    /// Scene-declared transient-overlay frame activity callback.
    pub(in crate::application) scene_transient_overlay_activity:
        Option<TransientOverlayActivity<State>>,
    /// Scene-declared lightweight frame-time overlay painter.
    pub(in crate::application) scene_transient_overlay: Option<TransientOverlayPainter<State>>,
}

impl<State, Message> Default for AppBridgeLifecycle<State, Message> {
    fn default() -> Self {
        Self {
            animation: None,
            frame_message: None,
            frame_clock_activity: None,
            frame_repaint_policy: None,
            scene_frame_message: None,
            scene_frame_clock_activity: None,
            scene_frame_repaint_policy: None,
            repaint_policy: None,
            subscriptions: None,
            shortcuts: None,
            scene_shortcuts: None,
            scroll: None,
            native_file_drop: None,
            native_file_open: None,
            startup: None,
            shutdown: None,
            close_requested: None,
            auxiliary_windows: None,
            retained_painters: HashMap::new(),
            transient_overlay_activity: None,
            transient_overlay: None,
            scene_transient_overlay_activity: None,
            scene_transient_overlay: None,
        }
    }
}

impl<State, Message> AppBridgeLifecycle<State, Message> {
    pub(in crate::application) fn clear_scene_presentation(&mut self) {
        self.scene_frame_message = None;
        self.scene_frame_clock_activity = None;
        self.scene_frame_repaint_policy = None;
        self.scene_transient_overlay_activity = None;
        self.scene_transient_overlay = None;
        self.scene_shortcuts = None;
    }
}

impl<State, Message, Project, Update, View> AppBridge<State, Message, Project, Update, View>
where
    Project: FnMut(&mut State) -> View,
    Update: FnMut(&mut State, Message, &mut UiUpdateContext<Message>),
    View: IntoView<Message>,
{
    /// Build an app bridge from host state, projection, reducer, and lifecycle hooks.
    pub(in crate::application) fn new(
        state: State,
        project: Project,
        update: Update,
        lifecycle: AppBridgeLifecycle<State, Message>,
    ) -> Self {
        Self {
            state,
            project,
            update,
            runtime: Arc::new(AppRuntime::default()),
            lifecycle,
            runtime_flags: AppBridgeRuntimeFlags::default(),
            _view: PhantomData,
        }
    }

    fn run_update(&mut self, message: Message) -> Command<Message> {
        self.run_update_with_runtime(message, RuntimeUpdateSnapshot::default())
    }

    fn run_update_with_runtime(
        &mut self,
        message: Message,
        runtime_snapshot: RuntimeUpdateSnapshot,
    ) -> Command<Message> {
        let pending_frame = self.runtime_flags.pending_frame_repaint.take();
        let ordinary_repaint = pending_frame
            .is_none()
            .then(|| self.ordinary_repaint_scope_for_message(&message))
            .flatten();
        let mut context = UiUpdateContext::from_runtime_snapshot(runtime_snapshot);
        (self.update)(&mut self.state, message, &mut context);
        let command = context.into_command();
        self.apply_repaint_policy(pending_frame, ordinary_repaint, command)
    }

    fn apply_repaint_policy(
        &mut self,
        pending_frame: Option<PendingFrameRepaint>,
        ordinary_repaint: Option<RepaintScope>,
        command: Command<Message>,
    ) -> Command<Message> {
        let repaint = match pending_frame {
            Some(pending) => self.frame_repaint_scope(pending, &command),
            None => ordinary_repaint.filter(|_| !command.requests_repaint()),
        };

        match repaint {
            Some(repaint) => Command::batch([command, Command::repaint(repaint)]),
            None => command,
        }
    }

    fn ordinary_repaint_scope_for_message(&self, message: &Message) -> Option<RepaintScope> {
        self.lifecycle
            .repaint_policy
            .as_ref()
            .map_or(Some(RepaintScope::Surface), |policy| {
                policy.scope_for(message)
            })
    }

    fn frame_repaint_scope(
        &mut self,
        pending: PendingFrameRepaint,
        command: &Command<Message>,
    ) -> Option<RepaintScope> {
        if command.requests_repaint() {
            return None;
        }

        let Some(scope) = pending.scope else {
            return Some(RepaintScope::Surface);
        };

        let can_use_paint_only = match pending.source {
            FrameRepaintSource::App => {
                let Some(policy) = self.lifecycle.frame_repaint_policy.as_mut() else {
                    return Some(RepaintScope::Surface);
                };
                policy.resolve_after_frame(&mut self.state, scope)
            }
            FrameRepaintSource::Scene => {
                let Some(policy) = self.lifecycle.scene_frame_repaint_policy.as_mut() else {
                    return Some(RepaintScope::Surface);
                };
                policy.resolve_after_frame(&mut self.state, scope)
            }
        };

        Some(if can_use_paint_only {
            RepaintScope::PaintOnly
        } else {
            RepaintScope::Surface
        })
    }

    fn run_startup_once(&mut self) {
        if self.runtime_flags.startup_ran {
            return;
        }
        if let Some(startup) = self.lifecycle.startup.as_mut() {
            let mut context = UiUpdateContext::default();
            startup(&mut self.state, &mut context);
            self.runtime.enqueue_command(context.into_command());
        }
        self.runtime_flags.startup_ran = true;
    }

    fn start_subscriptions_once(&mut self)
    where
        Message: Send + 'static,
    {
        if self.runtime_flags.subscriptions_started {
            return;
        }
        if let Some(subscriptions) = self.lifecycle.subscriptions.as_mut() {
            spawn_subscription(
                Arc::downgrade(&self.runtime),
                subscriptions(&mut self.state),
            );
        }
        self.runtime_flags.subscriptions_started = true;
    }

    pub(in crate::application) fn project_app_auxiliary_windows(
        &mut self,
    ) -> Vec<crate::runtime::AuxiliaryWindow<Message>> {
        self.lifecycle
            .auxiliary_windows
            .as_mut()
            .map(|project| project(&mut self.state))
            .unwrap_or_default()
    }
}
