use super::super::NativeFrameDiagnosticsPublication;
use crate::runtime::{
    Command, NativeFrameDiagnostics, RuntimeBridge, RuntimeFrameDiagnosticsHost,
    RuntimeHostCapabilities, UiSurface,
};
use std::sync::Arc;

pub(super) struct AuxiliarySurfaceBridge<Message> {
    pub(super) surface: Arc<UiSurface<Message>>,
    outbox: Vec<Message>,
    frame_diagnostics_enabled: bool,
    frame_diagnostics_publication: NativeFrameDiagnosticsPublication,
}

impl<Message> AuxiliarySurfaceBridge<Message> {
    pub(super) fn new(surface: Arc<UiSurface<Message>>, frame_diagnostics_enabled: bool) -> Self {
        Self {
            surface,
            outbox: Vec::new(),
            frame_diagnostics_enabled,
            frame_diagnostics_publication: NativeFrameDiagnosticsPublication::default(),
        }
    }

    pub(super) fn take_messages(&mut self) -> Vec<Message> {
        std::mem::take(&mut self.outbox)
    }

    pub(super) fn require_schedule_admission(&mut self) {
        if self.frame_diagnostics_enabled {
            self.frame_diagnostics_publication
                .require_schedule_admission();
        }
    }

    pub(super) fn mark_observation_finalized(&mut self) {
        if self.frame_diagnostics_enabled {
            self.frame_diagnostics_publication
                .mark_observation_finalized();
        }
    }

    pub(super) fn mark_schedule_admission_recorded(&mut self) {
        if self.frame_diagnostics_enabled {
            self.frame_diagnostics_publication
                .mark_schedule_admission_recorded();
        }
    }

    pub(super) fn take_ready_frame_diagnostics(&mut self) -> Option<NativeFrameDiagnostics> {
        if self.frame_diagnostics_enabled {
            self.frame_diagnostics_publication.take_ready()
        } else {
            None
        }
    }

    pub(super) fn discard_frame_diagnostics(&mut self) {
        self.frame_diagnostics_publication.discard();
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
        if self.frame_diagnostics_enabled {
            RuntimeHostCapabilities::new().with_frame_diagnostics()
        } else {
            RuntimeHostCapabilities::new()
        }
    }
}

impl<Message> RuntimeFrameDiagnosticsHost for AuxiliarySurfaceBridge<Message> {
    fn observe_frame_diagnostics(&mut self, diagnostics: NativeFrameDiagnostics) {
        if !self.frame_diagnostics_enabled {
            return;
        }

        // One auxiliary redraw event invokes one child presentation path. Keep
        // the handoff bounded to that event and drain it at the parent boundary.
        self.frame_diagnostics_publication.stage(diagnostics);
    }
}

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
        let mut bridge = AuxiliarySurfaceBridge::new(empty_surface(), false);

        let _ = bridge.update("open");
        let _ = bridge.update("close");

        assert_eq!(bridge.take_messages(), vec!["open", "close"]);
        assert!(bridge.take_messages().is_empty());
    }

    #[test]
    fn auxiliary_bridge_keeps_frame_diagnostics_disabled_without_capability_work() {
        let mut bridge = AuxiliarySurfaceBridge::<()>::new(empty_surface(), false);
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
        let mut bridge = AuxiliarySurfaceBridge::<()>::new(empty_surface(), true);
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
        let mut bridge = AuxiliarySurfaceBridge::<()>::new(empty_surface(), true);
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
}
