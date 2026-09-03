//! Cached dispatch for explicitly enabled host capabilities.

use super::SurfaceRuntime;
use super::owner::EffectOrigin;
use crate::{
    gui::{
        focus::FocusSurface, input::KeyPress, repaint::RepaintSignal, shortcuts::ShortcutResolution,
    },
    runtime::{
        AuxiliaryWindow, Command, FrameGpuTimingSample, FrameProfile, NativeFileDrop,
        NativeFileOpen, NativeFrameDiagnostics, PaintPrimitive, PlatformCompletion, PlatformEffect,
        PlatformFailure, PlatformRequest, PlatformResultDelivery, PlatformServiceFallback,
        RuntimeAnimationActivity, RuntimeBridge, RuntimeDiagnostics, RuntimeHostCapabilities,
        RuntimePlatformResultSink, RuntimeRetainedSurfaceCapability, ScrollUpdate, TaskPriority,
        TransientOverlayContext,
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
        if !self.lifecycle_accepts_work() {
            return;
        }
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
        if !self.lifecycle_accepts_work() {
            return false;
        }
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
        if !self.lifecycle_accepts_work() {
            return false;
        }
        self.host_capabilities
            .tasks
            .as_ref()
            .is_some_and(|capability| {
                (capability.spawn_worker_task)(&mut self.bridge, name, priority, is_cancelled, work)
            })
    }

    pub(in crate::runtime::controller) fn host_request_platform_service(
        &mut self,
        request: PlatformRequest,
        on_completed: PlatformCompletion<Message>,
        origin: &EffectOrigin,
    ) -> Result<(), PlatformServiceFallback<Message>> {
        if !self.lifecycle_accepts_work() {
            return Err(Box::new((request, on_completed)));
        }
        if request.is_in_process_clipboard() {
            self.diagnostics
                .record_platform_owner_kind(origin.platform_owner_kind());
            let identity =
                self.platform_registry
                    .register_legacy_for_request(on_completed, &request, origin);
            let Some(reservation) =
                crate::runtime::controller::platform::PlatformResultIngress::reserve(
                    &self.platform_results,
                )
            else {
                let accepted =
                    self.enqueue_platform_result(identity, Err(PlatformFailure::Capacity));
                if !accepted {
                    let _ = self.platform_registry.remove(identity);
                }
                return Ok(());
            };
            let result = self.in_process_clipboard.execute(&request);
            let accepted =
                reservation.commit(PlatformResultDelivery::Completed { identity, result });
            if !accepted {
                let _ = self.platform_registry.remove(identity);
            }
            return Ok(());
        }
        if request.validate().is_err() {
            return Err(Box::new((request, on_completed)));
        }
        if self.host_capabilities.platform_result.is_some() {
            self.diagnostics
                .record_platform_owner_kind(origin.platform_owner_kind());
            let identity =
                self.platform_registry
                    .register_legacy_for_request(on_completed, &request, origin);
            let Some(reservation) =
                crate::runtime::controller::platform::PlatformResultIngress::reserve(
                    &self.platform_results,
                )
            else {
                let accepted =
                    self.enqueue_platform_result(identity, Err(PlatformFailure::Capacity));
                if !accepted {
                    let _ = self.platform_registry.remove(identity);
                }
                return Ok(());
            };
            let sink = RuntimePlatformResultSink::new(identity, move |delivery| {
                let _ = reservation.commit(delivery);
            });
            let Some(capability) = self.host_capabilities.platform_result.as_ref() else {
                unreachable!("platform-result capability was checked above")
            };
            if let Err(fallback) =
                (capability.request_platform_result)(&mut self.bridge, request, sink)
            {
                let (_request, sink) = *fallback;
                sink.send(Err(PlatformFailure::Unavailable(_request.service())));
            }
            return Ok(());
        }
        Err(Box::new((request, on_completed)))
    }

    pub(in crate::runtime::controller) fn host_request_platform_effect(
        &mut self,
        effect: PlatformEffect<Message>,
        origin: &EffectOrigin,
    ) -> bool {
        let PlatformEffect {
            request,
            transaction,
            lifecycle,
            map,
        } = effect;
        if !origin.is_live() || !transaction.is_active() || (lifecycle.cancellation)() {
            transaction.reject();
            return false;
        }
        self.diagnostics
            .record_platform_owner_kind(origin.platform_owner_kind());
        let identity =
            self.platform_registry
                .register_effect(map, &request, origin, &lifecycle, transaction);
        let Some(reservation) =
            crate::runtime::controller::platform::PlatformResultIngress::reserve(
                &self.platform_results,
            )
        else {
            if !self.platform_registry.effect_is_current(identity) {
                let _ = self.platform_registry.remove(identity);
                return true;
            }
            self.platform_registry.reject_effect(identity);
            let accepted = self.enqueue_platform_result(identity, Err(PlatformFailure::Capacity));
            if !accepted {
                let _ = self.platform_registry.remove(identity);
            }
            return true;
        };

        if !self.platform_registry.effect_is_current(identity) {
            let _ = self.platform_registry.remove(identity);
            return true;
        }

        if request.validate().is_err() {
            self.platform_registry.reject_effect(identity);
            if !reservation.commit(PlatformResultDelivery::Completed {
                identity,
                result: Err(PlatformFailure::InvalidRequest),
            }) {
                let _ = self.platform_registry.remove(identity);
            }
            return true;
        }

        if request.is_in_process_clipboard() {
            let result = self.in_process_clipboard.execute(&request);
            self.platform_registry.accept_effect(identity);
            if !reservation.commit(PlatformResultDelivery::Completed { identity, result }) {
                let _ = self.platform_registry.remove(identity);
            }
            return true;
        }

        let Some(capability) = self.host_capabilities.platform_result.as_ref() else {
            self.platform_registry.reject_effect(identity);
            if !reservation.commit(PlatformResultDelivery::Completed {
                identity,
                result: Err(PlatformFailure::Unsupported(request.service())),
            }) {
                let _ = self.platform_registry.remove(identity);
            }
            return true;
        };
        let sink = RuntimePlatformResultSink::new(identity, move |delivery| {
            let _ = reservation.commit(delivery);
        });
        if let Err(fallback) = (capability.request_platform_result)(&mut self.bridge, request, sink)
        {
            let (request, sink) = *fallback;
            self.platform_registry.reject_effect(identity);
            sink.send(Err(PlatformFailure::Unavailable(request.service())));
        } else {
            self.platform_registry.accept_effect(identity);
        }
        true
    }

    fn enqueue_platform_result(
        &mut self,
        identity: crate::runtime::PlatformCompletionIdentity,
        result: crate::runtime::PlatformResult,
    ) -> bool {
        self.platform_results
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .enqueue_overflow(PlatformResultDelivery::Completed { identity, result })
    }

    /// Poll the cached host animation capability.
    pub fn host_animation_activity(&mut self) -> RuntimeAnimationActivity {
        if !self.lifecycle_accepts_work() {
            return RuntimeAnimationActivity::idle();
        }
        self.host_capabilities
            .animation
            .as_ref()
            .map_or_else(RuntimeAnimationActivity::idle, |capability| {
                (capability.animation_activity)(&mut self.bridge)
            })
    }

    /// Queue one host animation-frame message when enabled.
    pub fn host_queue_animation_frame(&mut self) -> bool {
        if !self.lifecycle_accepts_work() {
            return false;
        }
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

    pub(crate) fn has_frame_profile_host(&self) -> bool {
        self.host_capabilities.has_frame_profile()
    }

    pub(crate) fn host_observe_frame_profile(&mut self, profile: FrameProfile) {
        if let Some(capability) = self.host_capabilities.frame_profile.as_ref() {
            (capability.observe_frame_profile)(&mut self.bridge, profile);
        }
    }

    pub(crate) fn has_frame_gpu_timing_host(&self) -> bool {
        self.host_capabilities.has_frame_gpu_timing()
    }

    pub(crate) fn host_observe_frame_gpu_timing(&mut self, sample: FrameGpuTimingSample) {
        if let Some(capability) = self.host_capabilities.frame_gpu_timing.as_ref() {
            (capability.observe_frame_gpu_timing)(&mut self.bridge, sample);
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
        if self.lifecycle_phase() == crate::runtime::RuntimeLifecyclePhase::Stopped
            || self.host_exit_hook_called
        {
            return None;
        }
        self.begin_closing();
        self.host_exit_hook_called = true;
        let artifact = self
            .host_capabilities
            .lifecycle
            .as_ref()
            .and_then(|capability| (capability.on_runtime_exit)(&mut self.bridge));
        let _ = self.transition_lifecycle(crate::runtime::RuntimeLifecyclePhase::Stopped);
        artifact
    }

    pub(crate) fn host_on_runtime_closing(&mut self) {
        if self.host_closing_hook_called {
            return;
        }
        self.host_closing_hook_called = true;
        if let Some(capability) = self.host_capabilities.lifecycle.as_ref() {
            (capability.on_runtime_closing)(&mut self.bridge);
        }
    }

    pub(crate) fn host_close_requested(&mut self) -> bool {
        self.host_capabilities
            .lifecycle
            .as_ref()
            .is_none_or(|capability| (capability.close_requested)(&mut self.bridge))
    }
}
