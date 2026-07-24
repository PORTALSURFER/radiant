//! Native application activation policy for delayed initial window reveal.

use super::{FrameWork, FrameWorkReason, GenericNativeVelloRunner, SceneRebuildMode};
use crate::{
    gui_runtime::{NativeRunOptions, NativeWindowMode},
    runtime::RuntimeBridge,
};
use std::time::{Duration, Instant};
use tracing::{info, warn};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoopBuilder};

mod platform;
mod reopen;

pub(super) use reopen::ApplicationReopenRegistration;

pub(super) fn needs_application_reopen_handler(options: &NativeRunOptions) -> bool {
    StartupActivationPolicy::for_options(options) == StartupActivationPolicy::DelayedNormalWindow
}

const ACTIVATION_CONFIRMATION_POLL_INTERVAL: Duration = Duration::from_millis(16);
const ACTIVATION_CONFIRMATION_TIMEOUT: Duration = Duration::from_secs(1);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum StartupActivationPolicy {
    DelayedNormalWindow,
    EagerFocusedPopup,
    Passive,
}

impl StartupActivationPolicy {
    pub(super) fn for_options(options: &NativeRunOptions) -> Self {
        if !super::reveal_window_after_surface_setup(options) {
            return Self::Passive;
        }
        match options.window.behavior.mode {
            NativeWindowMode::Window => Self::DelayedNormalWindow,
            NativeWindowMode::Popup(popup) if popup.initially_focused => Self::EagerFocusedPopup,
            NativeWindowMode::Popup(_) => Self::Passive,
        }
    }

    pub(super) const fn activate_ignoring_other_apps_at_launch(self) -> bool {
        matches!(self, Self::EagerFocusedPopup)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum SurfaceReadyActivationAction {
    RevealActiveApplication,
    RequestActivation,
    AwaitExternalActivation,
    RevealPassively,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PendingReveal {
    None,
    Requested {
        poll_until: Instant,
    },
    AwaitingUserIntent,
    #[cfg(any(target_os = "macos", test))]
    UserRequested,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ApplicationActivationMethod {
    #[cfg(target_os = "macos")]
    Modern,
    #[cfg(target_os = "macos")]
    Compatibility,
    Unavailable,
}

impl ApplicationActivationMethod {
    const fn label(self) -> &'static str {
        match self {
            #[cfg(target_os = "macos")]
            Self::Modern => "modern",
            #[cfg(target_os = "macos")]
            Self::Compatibility => "compatibility",
            Self::Unavailable => "unavailable",
        }
    }
}

pub(super) struct ActivationRevealController {
    policy: StartupActivationPolicy,
    launch_foreground_process: Option<i32>,
    application_process: Option<i32>,
    pending: PendingReveal,
}

impl ActivationRevealController {
    pub(super) fn new(options: &NativeRunOptions) -> Self {
        Self {
            policy: StartupActivationPolicy::for_options(options),
            launch_foreground_process: platform::frontmost_process_id(),
            application_process: i32::try_from(std::process::id()).ok(),
            pending: PendingReveal::None,
        }
    }

    #[cfg(test)]
    pub(super) const fn with_launch_foreground_process(
        policy: StartupActivationPolicy,
        launch_foreground_process: Option<i32>,
    ) -> Self {
        Self {
            policy,
            launch_foreground_process,
            application_process: Some(7),
            pending: PendingReveal::None,
        }
    }

    pub(super) fn surface_ready(
        &mut self,
        application_active: bool,
        current_foreground_process: Option<i32>,
        now: Instant,
    ) -> SurfaceReadyActivationAction {
        if self.policy != StartupActivationPolicy::DelayedNormalWindow {
            return SurfaceReadyActivationAction::RevealPassively;
        }
        if application_active {
            return SurfaceReadyActivationAction::RevealActiveApplication;
        }
        if foreground_application_changed(
            self.launch_foreground_process,
            current_foreground_process,
            self.application_process,
        ) {
            self.pending = PendingReveal::AwaitingUserIntent;
            return SurfaceReadyActivationAction::AwaitExternalActivation;
        }
        self.pending = PendingReveal::Requested {
            poll_until: now + ACTIVATION_CONFIRMATION_TIMEOUT,
        };
        SurfaceReadyActivationAction::RequestActivation
    }

    pub(super) fn observe_application_active(&mut self, application_active: bool) -> bool {
        if !application_active {
            return false;
        }
        let pending_activation = match self.pending {
            PendingReveal::Requested { .. } => true,
            #[cfg(any(target_os = "macos", test))]
            PendingReveal::UserRequested => true,
            PendingReveal::None | PendingReveal::AwaitingUserIntent => false,
        };
        if !pending_activation {
            return false;
        }
        self.pending = PendingReveal::None;
        true
    }

    #[cfg(any(target_os = "macos", test))]
    pub(super) fn observe_user_reopen(&mut self, application_active: bool) -> bool {
        if self.pending != PendingReveal::AwaitingUserIntent {
            return false;
        }
        if application_active {
            self.pending = PendingReveal::None;
            true
        } else {
            self.pending = PendingReveal::UserRequested;
            false
        }
    }

    pub(super) fn activation_poll(
        &mut self,
        now: Instant,
        current_foreground_process: Option<i32>,
    ) -> ActivationPoll {
        let PendingReveal::Requested { poll_until } = self.pending else {
            return ActivationPoll::None;
        };
        if foreground_application_changed(
            self.launch_foreground_process,
            current_foreground_process,
            self.application_process,
        ) {
            self.pending = PendingReveal::AwaitingUserIntent;
            return ActivationPoll::ForegroundChanged;
        }
        if now >= poll_until {
            self.pending = PendingReveal::AwaitingUserIntent;
            return ActivationPoll::TimedOut;
        }
        ActivationPoll::WaitUntil((now + ACTIVATION_CONFIRMATION_POLL_INTERVAL).min(poll_until))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ActivationPoll {
    None,
    WaitUntil(Instant),
    ForegroundChanged,
    TimedOut,
}

fn foreground_application_changed(
    launch: Option<i32>,
    current: Option<i32>,
    application: Option<i32>,
) -> bool {
    matches!(
        (launch, current, application),
        (Some(launch), Some(current), application)
            if launch != current && Some(current) != application
    )
}

pub(super) fn configure_event_loop_activation<T>(
    builder: &mut EventLoopBuilder<T>,
    options: &NativeRunOptions,
) {
    let policy = StartupActivationPolicy::for_options(options);
    platform::configure_event_loop_activation(
        builder,
        policy.activate_ignoring_other_apps_at_launch(),
    );
}

impl<Bridge, Message> GenericNativeVelloRunner<Bridge, Message>
where
    Bridge: RuntimeBridge<Message>,
{
    pub(super) fn install_application_reopen_handler_if_needed(&mut self) {
        if self.application_reopen_events.is_some() {
            return;
        }
        let Some(proxy) = self.application_reopen_proxy.take() else {
            return;
        };
        self.application_reopen_events = Some(reopen::install_application_reopen_handler(proxy));
    }

    pub(super) fn reveal_prepared_window_at_activation_boundary(&mut self) {
        let now = Instant::now();
        let application_active = platform::application_is_active();
        let current_foreground_process = platform::frontmost_process_id();
        match self.activation_reveal.surface_ready(
            application_active,
            current_foreground_process,
            now,
        ) {
            SurfaceReadyActivationAction::RevealActiveApplication => {
                self.record_application_active("already-active");
                self.reveal_prepared_window("application-active");
            }
            SurfaceReadyActivationAction::RequestActivation => {
                let method = platform::request_application_activation();
                info!(
                    target: "radiant::native::activation",
                    event = "radiant.window.activation.requested",
                    method = method.label(),
                    launch_foreground_process = ?self.activation_reveal.launch_foreground_process,
                    current_foreground_process = ?current_foreground_process,
                    application_process = ?self.activation_reveal.application_process,
                    "Radiant requested application activation at the prepared-window reveal boundary"
                );
                self.observe_pending_window_activation();
            }
            SurfaceReadyActivationAction::AwaitExternalActivation => {
                info!(
                    target: "radiant::native::activation",
                    event = "radiant.window.activation.deferred",
                    launch_foreground_process = ?self.activation_reveal.launch_foreground_process,
                    current_foreground_process = ?current_foreground_process,
                    "Radiant deferred activation because foreground ownership changed during startup"
                );
            }
            SurfaceReadyActivationAction::RevealPassively => {
                self.reveal_prepared_window("passive-window-policy");
            }
        }
    }

    pub(super) fn observe_pending_window_activation(&mut self) {
        if self
            .activation_reveal
            .observe_application_active(platform::application_is_active())
        {
            self.record_application_active("activation-confirmed");
            self.reveal_prepared_window("activation-confirmed");
        }
    }

    #[cfg(target_os = "macos")]
    pub(super) fn handle_application_reopen_intent(&mut self) {
        let application_active = platform::application_is_active();
        if self
            .activation_reveal
            .observe_user_reopen(application_active)
        {
            self.record_application_active("user-reopen");
            self.reveal_prepared_window("user-reopen");
        } else {
            info!(
                target: "radiant::native::activation",
                event = "radiant.window.activation.user-intent",
                application_active,
                "Radiant observed an explicit application reopen intent"
            );
        }
    }

    pub(super) fn schedule_activation_confirmation_poll(
        &mut self,
        event_loop: &ActiveEventLoop,
        now: Instant,
    ) {
        match self
            .activation_reveal
            .activation_poll(now, platform::frontmost_process_id())
        {
            ActivationPoll::None => {}
            ActivationPoll::ForegroundChanged => info!(
                target: "radiant::native::activation",
                event = "radiant.window.activation.deferred",
                "Radiant deferred window reveal because foreground ownership changed"
            ),
            ActivationPoll::TimedOut => warn!(
                target: "radiant::native::activation",
                event = "radiant.window.activation.confirmation-timeout",
                "Radiant is waiting for a later user-driven application activation before revealing the prepared window"
            ),
            ActivationPoll::WaitUntil(deadline) => match event_loop.control_flow() {
                ControlFlow::Poll => {}
                ControlFlow::Wait => event_loop.set_control_flow(ControlFlow::WaitUntil(deadline)),
                ControlFlow::WaitUntil(current) if deadline < current => {
                    event_loop.set_control_flow(ControlFlow::WaitUntil(deadline));
                }
                ControlFlow::WaitUntil(_) => {}
            },
        }
    }

    fn record_application_active(&self, source: &'static str) {
        info!(
            target: "radiant::native::activation",
            event = "radiant.window.application.active",
            source,
            "Radiant observed the application active before initial window reveal"
        );
    }

    fn reveal_prepared_window(&mut self, reason: &'static str) {
        let Some(window) = self.window.window.as_ref() else {
            return;
        };
        self.application_reopen_events.take();
        self.application_reopen_proxy.take();
        window.set_visible(true);
        self.timing.startup_timing.mark_window_revealed();
        info!(
            target: "radiant::native::activation",
            event = "radiant.window.revealed",
            reason,
            "Radiant revealed the prepared native window"
        );
        self.request_redraw_for_frame_work(FrameWork::RebuildScene {
            reason: FrameWorkReason::RuntimeSurfaceRepaint,
            mode: SceneRebuildMode::Immediate,
        });
    }
}
