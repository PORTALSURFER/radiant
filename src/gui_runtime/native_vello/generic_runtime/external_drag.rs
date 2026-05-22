//! Native external drag launching for the generic Vello runtime.

use super::*;
use crate::runtime::{ExternalDragPayload, RuntimeBridge};

mod platform;

impl<Bridge, Message> GenericNativeVelloRunner<Bridge, Message>
where
    Bridge: RuntimeBridge<Message>,
{
    pub(super) fn launch_external_drag_if_armed(&mut self) -> GenericRouteOutcome {
        let Some(session) = self.core.runtime.take_external_drag_session() else {
            return GenericRouteOutcome::default();
        };
        self.core.runtime.cancel_pointer_capture();
        let preview_cleared = self.core.runtime.take_drag_preview_for_external_drag();
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
        let mut route_outcome = self.core.route_command_outcome(outcome);
        if preview_cleared {
            route_outcome.repaint_requested = true;
        }
        route_outcome
    }
}
