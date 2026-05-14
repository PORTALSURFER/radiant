//! Native external drag launching for the generic Vello runtime.

use super::*;
use crate::runtime::{
    ExternalDragOutcome, ExternalDragPayload, ExternalDragRequest, RuntimeBridge,
};

#[cfg(target_os = "windows")]
mod windows;

impl<Bridge, Message> GenericNativeVelloRunner<Bridge, Message>
where
    Bridge: RuntimeBridge<Message>,
{
    pub(super) fn launch_external_drag_if_armed(&mut self) -> GenericRouteOutcome {
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
        let result = platform_start_external_drag(&session.request);
        let outcome = self
            .core
            .runtime
            .dispatch_external_drag_result(session, result);
        self.core.route_command_outcome(outcome)
    }
}

#[cfg(target_os = "windows")]
fn platform_start_external_drag(
    request: &ExternalDragRequest,
) -> Result<ExternalDragOutcome, String> {
    windows::start_external_drag(request)
}

#[cfg(not(target_os = "windows"))]
fn platform_start_external_drag(
    _request: &ExternalDragRequest,
) -> Result<ExternalDragOutcome, String> {
    Err(String::from(
        "External drag-out is only supported on Windows in this backend",
    ))
}
