use super::super::NativeFrameDiagnosticsPublication;
use crate::runtime::{
    Command, FrameGpuTimingSample, NativeFrameDiagnostics, RuntimeBridge,
    RuntimeFrameDiagnosticsHost, RuntimeFrameGpuTimingHost, RuntimeHostCapabilities,
    RuntimeInputHost, UiSurface,
};
use std::sync::Arc;

#[derive(Default)]
struct AuxiliaryFrameGpuTimingPublication {
    pending: Option<FrameGpuTimingSample>,
}

impl AuxiliaryFrameGpuTimingPublication {
    fn stage(&mut self, sample: FrameGpuTimingSample) {
        if self.pending.is_some() {
            debug_assert!(
                false,
                "auxiliary frame GPU timing staged more than once before take"
            );
            return;
        }
        self.pending = Some(sample);
    }

    fn take(&mut self) -> Option<FrameGpuTimingSample> {
        self.pending.take()
    }

    fn discard(&mut self) {
        self.pending = None;
    }
}

#[derive(Debug, PartialEq, Eq)]
pub(in crate::gui_runtime::native_vello::generic_runtime) struct AuxiliaryFrameDiagnostics {
    pub(in crate::gui_runtime::native_vello::generic_runtime) diagnostics: NativeFrameDiagnostics,
    pub(in crate::gui_runtime::native_vello::generic_runtime) profile_enabled: bool,
}

pub(super) struct AuxiliarySurfaceBridge<Message> {
    pub(super) command_service: Option<crate::application::CommandService<Message>>,
    pub(super) surface: Arc<UiSurface<Message>>,
    outbox: Vec<Message>,
    frame_observation_enabled: bool,
    frame_profile_enabled: bool,
    frame_gpu_timing_enabled: bool,
    frame_diagnostics_publication: NativeFrameDiagnosticsPublication,
    frame_gpu_timing_publication: AuxiliaryFrameGpuTimingPublication,
}

impl<Message> AuxiliarySurfaceBridge<Message> {
    #[cfg(test)]
    pub(super) fn new(
        surface: Arc<UiSurface<Message>>,
        frame_diagnostics_enabled: bool,
        frame_profile_enabled: bool,
    ) -> Self {
        Self::new_with_gpu_timing(
            surface,
            frame_diagnostics_enabled,
            frame_profile_enabled,
            false,
        )
    }

    pub(super) fn new_with_gpu_timing(
        surface: Arc<UiSurface<Message>>,
        frame_diagnostics_enabled: bool,
        frame_profile_enabled: bool,
        frame_gpu_timing_enabled: bool,
    ) -> Self {
        Self {
            surface,
            command_service: None,
            outbox: Vec::new(),
            frame_observation_enabled: frame_diagnostics_enabled || frame_profile_enabled,
            frame_profile_enabled,
            frame_gpu_timing_enabled,
            frame_diagnostics_publication: NativeFrameDiagnosticsPublication::default(),
            frame_gpu_timing_publication: AuxiliaryFrameGpuTimingPublication::default(),
        }
    }

    pub(super) fn take_messages(&mut self) -> Vec<Message> {
        std::mem::take(&mut self.outbox)
    }

    pub(super) fn require_schedule_admission(&mut self) {
        if self.frame_observation_enabled {
            self.frame_diagnostics_publication
                .require_schedule_admission();
        }
    }

    pub(super) fn mark_observation_finalized(&mut self) {
        if self.frame_observation_enabled {
            self.frame_diagnostics_publication
                .mark_observation_finalized();
        }
    }

    pub(super) fn mark_schedule_admission_recorded(&mut self) {
        if self.frame_observation_enabled {
            self.frame_diagnostics_publication
                .mark_schedule_admission_recorded();
        }
    }

    #[cfg(test)]
    pub(super) fn take_ready_frame_diagnostics(&mut self) -> Option<NativeFrameDiagnostics> {
        self.take_ready_frame_observation()
            .map(|handoff| handoff.diagnostics)
    }

    pub(super) fn take_ready_frame_observation(&mut self) -> Option<AuxiliaryFrameDiagnostics> {
        if !self.frame_observation_enabled {
            return None;
        }
        self.frame_diagnostics_publication
            .take_ready()
            .map(|diagnostics| AuxiliaryFrameDiagnostics {
                diagnostics,
                profile_enabled: self.frame_profile_enabled,
            })
    }

    pub(super) fn discard_frame_diagnostics(&mut self) {
        self.frame_diagnostics_publication.discard();
    }

    pub(super) fn take_ready_frame_gpu_timing(&mut self) -> Option<FrameGpuTimingSample> {
        self.frame_gpu_timing_publication.take()
    }

    pub(super) fn discard_frame_gpu_timing(&mut self) {
        self.frame_gpu_timing_publication.discard();
    }
}

impl<Message> RuntimeBridge<Message> for AuxiliarySurfaceBridge<Message> {
    fn project_surface(&mut self) -> Arc<UiSurface<Message>> {
        Arc::clone(&self.surface)
    }

    fn update(&mut self, message: Message) -> Command<Message> {
        self.outbox.push(message);
        Command::none()
    }

    fn host_capabilities(&self) -> RuntimeHostCapabilities<Self, Message> {
        let mut capabilities = RuntimeHostCapabilities::new().with_input();
        if self.frame_observation_enabled {
            capabilities = capabilities.with_frame_diagnostics();
        }
        if self.frame_gpu_timing_enabled {
            capabilities = capabilities.with_frame_gpu_timing();
        }
        capabilities
    }
}

impl<Message> RuntimeInputHost<Message> for AuxiliarySurfaceBridge<Message> {
    fn command_service(&self) -> Option<crate::application::CommandService<Message>> {
        self.command_service.clone()
    }

    fn resolve_command_with_scopes(
        &mut self,
        request: crate::application::CommandRequest<'_>,
        _focus: crate::application::CommandFocus,
        scopes: crate::application::CommandScopeProjection<'_>,
    ) -> crate::application::CommandDispatch<Message> {
        self.command_service
            .as_ref()
            .map_or_else(crate::application::CommandDispatch::unhandled, |service| {
                service.resolve(request, scopes)
            })
    }
}

impl<Message> RuntimeFrameDiagnosticsHost for AuxiliarySurfaceBridge<Message> {
    fn observe_frame_diagnostics(&mut self, diagnostics: NativeFrameDiagnostics) {
        if !self.frame_observation_enabled {
            return;
        }

        // One auxiliary redraw event invokes one child presentation path. Keep
        // the handoff bounded to that event and drain it at the parent boundary.
        self.frame_diagnostics_publication.stage(diagnostics);
    }
}

impl<Message> RuntimeFrameGpuTimingHost for AuxiliarySurfaceBridge<Message> {
    fn observe_frame_gpu_timing(&mut self, sample: FrameGpuTimingSample) {
        if self.frame_gpu_timing_enabled {
            // The parent event boundary drains this handoff and invokes the
            // application's observer; the child never publishes directly.
            self.frame_gpu_timing_publication.stage(sample);
        }
    }
}

#[cfg(test)]
mod command_tests;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{layout::NodeId, runtime::SurfaceNode};

    fn empty_surface<Message>() -> Arc<UiSurface<Message>> {
        crate::runtime::test_arc_surface(UiSurface::new(SurfaceNode::column(
            NodeId::from(1_u64),
            0.0,
            Vec::new(),
        )))
    }

    #[test]
    fn auxiliary_bridge_queues_surface_messages_until_drained() {
        let mut bridge = AuxiliarySurfaceBridge::new(empty_surface(), false, false);

        let _ = bridge.update("open");
        let _ = bridge.update("close");

        assert_eq!(bridge.take_messages(), vec!["open", "close"]);
        assert!(bridge.take_messages().is_empty());
    }

    #[test]
    fn auxiliary_bridge_keeps_frame_diagnostics_disabled_without_capability_work() {
        let mut bridge = AuxiliarySurfaceBridge::<()>::new(empty_surface(), false, false);
        let diagnostics = NativeFrameDiagnostics {
            window_identity: Some(
                crate::runtime::NativeWindowDiagnosticIdentity::from_runtime_value(2),
            ),
            frame_sequence: Some(7),
            ..NativeFrameDiagnostics::default()
        };

        assert!(!bridge.host_capabilities().has_frame_diagnostics());
        bridge.observe_frame_diagnostics(diagnostics);
        assert_eq!(bridge.take_ready_frame_diagnostics(), None);
    }

    #[test]
    fn auxiliary_bridge_hands_off_each_enabled_frame_once_without_stale_state() {
        let mut bridge = AuxiliarySurfaceBridge::<()>::new(empty_surface(), true, false);
        let first = NativeFrameDiagnostics {
            window_identity: Some(
                crate::runtime::NativeWindowDiagnosticIdentity::from_runtime_value(2),
            ),
            frame_sequence: Some(11),
            ..NativeFrameDiagnostics::default()
        };
        let second = NativeFrameDiagnostics {
            window_identity: Some(
                crate::runtime::NativeWindowDiagnosticIdentity::from_runtime_value(3),
            ),
            frame_sequence: Some(4),
            ..NativeFrameDiagnostics::default()
        };

        assert!(bridge.host_capabilities().has_frame_diagnostics());
        bridge.observe_frame_diagnostics(first);
        bridge.mark_observation_finalized();
        assert_eq!(bridge.take_ready_frame_diagnostics(), Some(first));
        assert_eq!(bridge.take_ready_frame_diagnostics(), None);

        bridge.observe_frame_diagnostics(second);
        bridge.mark_observation_finalized();
        assert_eq!(bridge.take_ready_frame_diagnostics(), Some(second));
        assert_eq!(bridge.take_ready_frame_diagnostics(), None);
    }

    #[test]
    fn auxiliary_bridge_keeps_scheduled_value_until_admission_is_recorded() {
        let mut bridge = AuxiliarySurfaceBridge::<()>::new(empty_surface(), true, false);
        let diagnostics = NativeFrameDiagnostics {
            window_identity: Some(
                crate::runtime::NativeWindowDiagnosticIdentity::from_runtime_value(2),
            ),
            frame_sequence: Some(11),
            ..NativeFrameDiagnostics::default()
        };

        bridge.require_schedule_admission();
        bridge.observe_frame_diagnostics(diagnostics);
        bridge.mark_observation_finalized();
        assert_eq!(bridge.take_ready_frame_diagnostics(), None);

        bridge.mark_schedule_admission_recorded();
        assert_eq!(bridge.take_ready_frame_diagnostics(), Some(diagnostics));
    }

    #[test]
    fn auxiliary_bridge_carries_profile_eligibility_through_frame_handoff() {
        let mut bridge = AuxiliarySurfaceBridge::<()>::new(empty_surface(), false, true);
        let diagnostics = NativeFrameDiagnostics {
            frame_sequence: Some(11),
            ..NativeFrameDiagnostics::default()
        };

        assert!(bridge.host_capabilities().has_frame_diagnostics());
        bridge.observe_frame_diagnostics(diagnostics);
        bridge.mark_observation_finalized();

        assert_eq!(
            bridge.take_ready_frame_observation(),
            Some(AuxiliaryFrameDiagnostics {
                diagnostics,
                profile_enabled: true,
            })
        );
    }

    #[test]
    fn auxiliary_bridge_hands_off_gpu_timing_once_through_parent_boundary() {
        let mut bridge =
            AuxiliarySurfaceBridge::<()>::new_with_gpu_timing(empty_surface(), false, false, true);
        let sample = FrameGpuTimingSample::new(
            9,
            41,
            crate::runtime::FrameGpuTimingOutcome::available(std::time::Duration::from_nanos(13)),
        );

        assert!(bridge.host_capabilities().has_frame_gpu_timing());
        bridge.observe_frame_gpu_timing(sample);
        assert_eq!(bridge.take_ready_frame_gpu_timing(), Some(sample));
        assert_eq!(bridge.take_ready_frame_gpu_timing(), None);
    }
}
