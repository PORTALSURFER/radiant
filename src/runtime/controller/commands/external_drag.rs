use super::*;
use crate::runtime::{ExternalDragOutcome, ExternalDragSession};

impl<Bridge, Message> SurfaceRuntime<Bridge, Message>
where
    Bridge: RuntimeBridge<Message>,
{
    /// Return whether a native external drag session is currently armed.
    pub fn external_drag_armed(&self) -> bool {
        self.interaction.drag.external_session.is_some()
    }

    pub(crate) fn take_external_drag_session(&mut self) -> Option<ExternalDragSession<Message>> {
        self.interaction.drag.external_session.take()
    }

    pub(crate) fn dispatch_external_drag_result(
        &mut self,
        session: ExternalDragSession<Message>,
        result: Result<ExternalDragOutcome, String>,
    ) -> CommandOutcome {
        let Some(map) = session.on_completed else {
            return CommandOutcome::default();
        };
        self.dispatch_message(map(result))
    }
}
