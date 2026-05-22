//! Native external drag launching for the generic Vello runtime.

use super::*;
use crate::runtime::{ExternalDragPayload, RuntimeBridge};

mod platform;

impl<Bridge, Message> GenericNativeVelloRunner<Bridge, Message>
where
    Bridge: RuntimeBridge<Message>,
{
    pub(super) fn launch_external_drag_if_armed(&mut self) -> GenericRouteOutcome {
        if self.core.runtime.drag_session_active() {
            return GenericRouteOutcome::default();
        }
        let Some(session) = self.core.runtime.take_external_drag_session() else {
            return GenericRouteOutcome::default();
        };
        let path_count = match &session.request.payload {
            ExternalDragPayload::Files(paths) => paths.len(),
        };
        info!(
            path_count,
            preview = %session.request.preview.label,
            "radiant generic native vello: launching external drag"
        );
        let result = platform::start_external_drag(&session.request);
        let outcome = self
            .core
            .runtime
            .dispatch_external_drag_result(session, result);
        self.core.route_command_outcome(outcome)
    }
}
