use super::{CommandOutcome, SurfaceRuntime};
use crate::runtime::{
    ExternalDragCompletion, ExternalDragIdentity, ExternalDragLaunch, ExternalDragOutcome,
    ExternalDragRequest, ExternalDragSession, PendingExternalDragCompletion, RuntimeBridge,
};

impl<Bridge, Message> SurfaceRuntime<Bridge, Message>
where
    Bridge: RuntimeBridge<Message>,
{
    /// Return whether a native external drag session is currently armed.
    pub fn external_drag_armed(&self) -> bool {
        self.interaction.drag.external_session.is_some()
    }

    pub(crate) fn begin_external_drag_session(
        &mut self,
        request: ExternalDragRequest,
        on_completed: Option<ExternalDragCompletion<Message>>,
    ) {
        self.invalidate_external_drag();
        let identity = ExternalDragIdentity {
            id: self.interaction.drag.next_external_drag_id,
            epoch: self.interaction.drag.external_drag_epoch,
        };
        self.interaction.drag.next_external_drag_id = self
            .interaction
            .drag
            .next_external_drag_id
            .saturating_add(1);
        self.interaction.drag.external_identity = Some(identity);
        self.interaction.drag.external_session =
            Some(ExternalDragSession::new(request, on_completed, identity));
    }

    pub(crate) fn invalidate_external_drag(&mut self) {
        self.interaction.drag.external_session = None;
        self.interaction.drag.external_completion = None;
        self.interaction.drag.pending_external_completion = None;
        self.interaction.drag.external_identity = None;
        self.interaction.drag.external_drag_epoch =
            self.interaction.drag.external_drag_epoch.saturating_add(1);
    }

    pub(crate) fn take_external_drag_launch(&mut self) -> Option<ExternalDragLaunch> {
        let session = self.interaction.drag.external_session.take()?;
        self.interaction.drag.external_completion = session.on_completed;
        Some(ExternalDragLaunch {
            request: session.request,
            identity: session.identity,
        })
    }

    pub(crate) fn dispatch_external_drag_launch_result(
        &mut self,
        identity: ExternalDragIdentity,
        result: Result<ExternalDragOutcome, String>,
    ) -> CommandOutcome {
        if self.interaction.drag.external_identity != Some(identity)
            || self.interaction.drag.pending_external_completion.is_some()
        {
            return CommandOutcome::default();
        }
        let Some(on_completed) = self.interaction.drag.external_completion.take() else {
            self.interaction.drag.external_identity = None;
            return CommandOutcome::default();
        };
        self.interaction.drag.pending_external_completion = Some(PendingExternalDragCompletion {
            identity,
            on_completed,
            result,
        });
        // Keep the native route alive long enough for the next controller
        // drain to admit and map this completion on the UI owner.
        CommandOutcome {
            runtime_work_remaining: true,
            ..CommandOutcome::default()
        }
    }

    pub(crate) fn take_pending_external_drag_completion(
        &mut self,
    ) -> Option<PendingExternalDragCompletion<Message>> {
        let pending = self.interaction.drag.pending_external_completion.take()?;
        if self.interaction.drag.external_identity != Some(pending.identity) {
            return None;
        }
        self.interaction.drag.external_identity = None;
        Some(pending)
    }
}
