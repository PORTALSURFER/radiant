//! Cached dispatch for explicitly enabled host capabilities.

use super::SurfaceRuntime;
use crate::{
    gui::{
        focus::FocusSurface, input::KeyPress, repaint::RepaintSignal, shortcuts::ShortcutResolution,
    },
    runtime::{
        AuxiliaryWindow, Command, NativeFileDrop, NativeFileOpen, NativeFrameDiagnostics,
        PaintPrimitive, PlatformCompletion, PlatformRequest, PlatformResultDelivery,
        PlatformServiceFallback, RuntimeAnimationActivity, RuntimeBridge, RuntimeDiagnostics,
        RuntimeHostCapabilities, RuntimePlatformResultSink, RuntimeRetainedSurfaceCapability,
        ScrollUpdate, TaskPriority, TransientOverlayContext,
    },
};
use std::{sync::Arc, time::Duration};

impl<Bridge, Message> SurfaceRuntime<Bridge, Message>
where
    Bridge: RuntimeBridge<Message>,
{
    /// Return the stable capability table cached when this runtime was created.
    pub fn host_capabilities(&self) -> &RuntimeHostCapabilities<Bridge, Message> {
        &self.host_capabilities
    }

    pub(crate) fn host_scroll_updated(&mut self, update: ScrollUpdate) -> Option<Command<Message>> {
        let capability = self.host_capabilities.input.as_ref()?;
        (capability.scroll_updated)(&mut self.bridge, update)
    }

    pub(crate) fn host_native_file_drop(&mut self, drop: NativeFileDrop) -> Command<Message> {
        self.host_capabilities
            .input
            .as_ref()
            .map_or_else(Command::none, |capability| {
                (capability.native_file_drop)(&mut self.bridge, drop)
            })
    }

    pub(crate) fn host_native_file_open(&mut self, open: NativeFileOpen) -> Command<Message> {
        self.host_capabilities
            .input
            .as_ref()
            .map_or_else(Command::none, |capability| {
                (capability.native_file_open)(&mut self.bridge, open)
            })
    }

    pub(crate) fn host_native_focus_regained(&mut self) -> Command<Message> {
        self.host_capabilities
            .input
            .as_ref()
            .map_or_else(Command::none, |capability| {
                (capability.native_focus_regained)(&mut self.bridge)
            })
    }

    pub(crate) fn host_resolve_key_press(
        &mut self,
        pending_chord: Option<KeyPress>,
        press: KeyPress,
        focus: FocusSurface,
    ) -> ShortcutResolution<Message> {
        self.host_capabilities.input.as_ref().map_or_else(
            ShortcutResolution::unhandled,
            |capability| {
                (capability.resolve_key_press)(&mut self.bridge, pending_chord, press, focus)
            },
        )
    }

    /// Install the host repaint signal when task hosting is enabled.
    pub fn host_install_repaint_signal(&mut self, signal: Arc<dyn RepaintSignal>) {
        if let Some(capability) = self.host_capabilities.tasks.as_ref() {
            (capability.install_repaint_signal)(&mut self.bridge, signal);
        }
    }

    #[allow(dead_code)]
    pub(crate) fn host_schedule_timer(
        &mut self,
        delay: Duration,
        wake: crate::runtime::RuntimeTimerWake,
    ) -> bool {
        self.host_capabilities
            .tasks
            .as_ref()
            .is_some_and(|capability| (capability.schedule_timer)(&mut self.bridge, delay, wake))
    }

    pub(crate) fn host_spawn_worker_task(
        &mut self,
        name: &'static str,
        priority: TaskPriority,
        is_cancelled: Option<Box<dyn Fn() -> bool + Send + Sync + 'static>>,
        work: Box<dyn FnOnce() + Send + 'static>,
    ) -> bool {
        self.host_capabilities
            .tasks
            .as_ref()
            .is_some_and(|capability| {
                (capability.spawn_worker_task)(&mut self.bridge, name, priority, is_cancelled, work)
            })
    }

    pub(crate) fn host_request_platform_service(
        &mut self,
        request: PlatformRequest,
        on_completed: PlatformCompletion<Message>,
    ) -> Result<(), PlatformServiceFallback<Message>> {
        if self.host_capabilities.platform_result.is_some() {
            let identity = self.platform_registry.register(on_completed);
            let Some(reservation) =
                crate::runtime::controller::platform::PlatformResultIngress::reserve(
                    &self.platform_results,
                )
            else {
                let accepted = self
                    .platform_results
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner())
                    .enqueue_overflow(PlatformResultDelivery {
                        identity,
                        result: Err(String::from("platform result ingress is saturated")),
                    });
                if !accepted {
                    let _ = self.platform_registry.remove(identity);
                }
                return Ok(());
            };
            let sink = RuntimePlatformResultSink::new(identity, move |result| {
                let _ = reservation.commit(PlatformResultDelivery { identity, result });
            });
            let Some(capability) = self.host_capabilities.platform_result.as_ref() else {
                unreachable!("platform-result capability was checked above")
            };
            if let Err(fallback) =
                (capability.request_platform_result)(&mut self.bridge, request, sink)
            {
                let (_request, sink) = *fallback;
                sink.send(Err(String::from(
                    "platform service request was rejected by the runtime host",
                )));
            }
            return Ok(());
        }
        let Some(capability) = self.host_capabilities.platform.as_ref() else {
            return Err(Box::new((request, on_completed)));
        };
        (capability.request_platform_service)(&mut self.bridge, request, on_completed)
    }

    /// Poll the cached host animation capability.
    pub fn host_animation_activity(&mut self) -> RuntimeAnimationActivity {
        self.host_capabilities
            .animation
            .as_ref()
            .map_or_else(RuntimeAnimationActivity::idle, |capability| {
                (capability.animation_activity)(&mut self.bridge)
            })
    }

    /// Queue one host animation-frame message when enabled.
    pub fn host_queue_animation_frame(&mut self) -> bool {
        self.host_capabilities
            .animation
            .as_ref()
            .is_some_and(|capability| (capability.queue_animation_frame)(&mut self.bridge))
    }

    pub(crate) fn host_project_auxiliary_windows(&mut self) -> Vec<AuxiliaryWindow<Message>> {
        self.host_capabilities
            .windows
            .as_ref()
            .map_or_else(Vec::new, |capability| {
                (capability.project_auxiliary_windows)(&mut self.bridge)
            })
    }

    pub(crate) fn retained_surface_capability(
        &self,
    ) -> Option<RuntimeRetainedSurfaceCapability<Bridge>> {
        self.host_capabilities.retained_surface
    }

    /// Return whether transient overlay painting is enabled.
    pub fn has_transient_overlay_host(&self) -> bool {
        self.host_capabilities.has_transient_overlay()
    }

    /// Paint the enabled host transient overlay.
    pub fn host_paint_transient_overlay(
        &mut self,
        context: TransientOverlayContext<'_>,
        primitives: &mut Vec<PaintPrimitive>,
    ) {
        if let Some(capability) = self.host_capabilities.transient_overlay.as_ref() {
            (capability.paint_transient_overlay)(&mut self.bridge, context, primitives);
        }
    }

    pub(crate) fn has_frame_diagnostics_host(&self) -> bool {
        self.host_capabilities.has_frame_diagnostics()
    }

    pub(crate) fn host_observe_frame_diagnostics(&mut self, diagnostics: NativeFrameDiagnostics) {
        if let Some(capability) = self.host_capabilities.frame_diagnostics.as_ref() {
            (capability.observe_frame_diagnostics)(&mut self.bridge, diagnostics);
        }
    }

    pub(crate) fn host_runtime_diagnostics(&self) -> RuntimeDiagnostics {
        self.host_capabilities
            .runtime_diagnostics
            .as_ref()
            .map_or_else(RuntimeDiagnostics::default, |capability| {
                (capability.runtime_diagnostics)(&self.bridge)
            })
    }

    /// Run the optional host runtime-exit hook.
    pub fn host_on_runtime_exit(&mut self) -> Option<serde_json::Value> {
        let capability = self.host_capabilities.lifecycle.as_ref()?;
        (capability.on_runtime_exit)(&mut self.bridge)
    }

    pub(crate) fn host_close_requested(&mut self) -> bool {
        self.host_capabilities
            .lifecycle
            .as_ref()
            .is_none_or(|capability| (capability.close_requested)(&mut self.bridge))
    }
}
