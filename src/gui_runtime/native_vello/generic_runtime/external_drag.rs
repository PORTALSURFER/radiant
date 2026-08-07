//! Native external drag launching for the generic Vello runtime.

use super::{FrameWorkReason, GenericNativeVelloRunner, GenericRouteOutcome};
use crate::runtime::{
    ExternalDragIdentity, ExternalDragOutcome, ExternalDragPayload, RuntimeBridge,
};
use tracing::info;
use winit::{keyboard::ModifiersState, window::WindowId};

mod platform;

#[derive(Debug, PartialEq, Eq)]
pub(super) enum ExternalDragLaunchDisposition {
    #[allow(
        dead_code,
        reason = "The completed disposition is used only by the Windows adapter."
    )]
    Completed(crate::runtime::ExternalDragOutcome),
    #[allow(
        dead_code,
        reason = "The pending disposition is used only by the macOS adapter."
    )]
    Pending,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ExternalDragOwner {
    Primary,
    Auxiliary(usize),
}

fn external_drag_owner(
    window_id: WindowId,
    primary_window_id: Option<WindowId>,
    mut auxiliary_window_ids: impl Iterator<Item = Option<WindowId>>,
) -> Option<ExternalDragOwner> {
    if primary_window_id == Some(window_id) {
        return Some(ExternalDragOwner::Primary);
    }
    auxiliary_window_ids
        .position(|candidate| candidate == Some(window_id))
        .map(ExternalDragOwner::Auxiliary)
}

impl<Bridge, Message> GenericNativeVelloRunner<Bridge, Message>
where
    Bridge: RuntimeBridge<Message>,
{
    pub(super) fn should_launch_external_drag_before_app_switch(
        &self,
        modifiers: ModifiersState,
    ) -> bool {
        platform::should_launch_before_app_switch(
            self.core.runtime.external_drag_armed(),
            self.core.runtime.drag_session_active(),
            self.input.modifiers.super_key(),
            modifiers.super_key(),
        )
    }

    pub(super) fn launch_external_drag_if_armed(&mut self) -> GenericRouteOutcome {
        let Some(launch) = self.core.runtime.take_external_drag_launch() else {
            return GenericRouteOutcome::default();
        };
        self.input.effective_pointer_gesture = None;
        self.core.runtime.cancel_pointer_capture();
        let preview_cleared = self.core.runtime.take_drag_preview_for_external_drag();
        let path_count = match &launch.request.payload {
            ExternalDragPayload::Files(paths) => paths.len(),
        };
        info!(
            path_count,
            preview = %launch.request.preview.label,
            "radiant generic native vello: launching external drag"
        );
        let launch_result = platform::start_external_drag(
            &launch.request,
            platform::ExternalDragLaunchContext::new(
                self.window.id,
                self.runtime_wakeup.event_loop_proxy(),
                launch.identity,
            ),
        );
        let outcome =
            self.dispatch_external_drag_launch_disposition(launch.identity, launch_result);
        let mut route_outcome = self.core.route_command_outcome(outcome);
        if preview_cleared {
            route_outcome.request_scene_rebuild(FrameWorkReason::ExternalDragPreview);
        }
        route_outcome
    }

    pub(super) fn dispatch_external_drag_launch_disposition(
        &mut self,
        identity: ExternalDragIdentity,
        launch_result: Result<ExternalDragLaunchDisposition, String>,
    ) -> crate::runtime::CommandOutcome {
        match launch_result {
            Ok(ExternalDragLaunchDisposition::Pending) => {
                // A macOS NSDraggingSession has only been admitted here. Its
                // source callback owns the terminal result and will enqueue
                // it after AppKit calls draggingSession:endedAtPoint:operation:.
                crate::runtime::CommandOutcome::default()
            }
            Ok(ExternalDragLaunchDisposition::Completed(result)) => self
                .core
                .runtime
                .dispatch_external_drag_launch_result(identity, Ok(result)),
            Err(error) => self
                .core
                .runtime
                .dispatch_external_drag_launch_result(identity, Err(error)),
        }
    }

    pub(super) fn handle_external_drag_completion(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        window_id: WindowId,
        identity: ExternalDragIdentity,
        result: Result<ExternalDragOutcome, String>,
    ) {
        if !self.is_running() {
            return;
        }
        match external_drag_owner(
            window_id,
            self.window.id,
            self.auxiliary_windows
                .iter()
                .map(|window| window.window_id()),
        ) {
            Some(ExternalDragOwner::Primary) => {
                let outcome = self
                    .core
                    .runtime
                    .dispatch_external_drag_launch_result(identity, result);
                let routed = self.core.route_command_outcome(outcome);
                self.handle_route_outcome(event_loop, routed);
            }
            Some(ExternalDragOwner::Auxiliary(index)) => {
                let Some(adapter) = self.adapter.as_mut() else {
                    return;
                };
                self.auxiliary_windows[index]
                    .dispatch_external_drag_completion(event_loop, identity, result, adapter);
            }
            None => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{ExternalDragOwner, external_drag_owner};
    use winit::window::WindowId;

    #[test]
    fn owner_lookup_prefers_exact_primary_or_auxiliary_window_id() {
        let primary = WindowId::from(11);
        let auxiliary = WindowId::from(12);

        assert_eq!(
            external_drag_owner(primary, Some(primary), [Some(auxiliary)].into_iter()),
            Some(ExternalDragOwner::Primary)
        );
        assert_eq!(
            external_drag_owner(auxiliary, Some(primary), [Some(auxiliary)].into_iter()),
            Some(ExternalDragOwner::Auxiliary(0))
        );
        assert_eq!(
            external_drag_owner(
                WindowId::from(99),
                Some(primary),
                [Some(auxiliary)].into_iter()
            ),
            None
        );
    }

    #[test]
    fn owner_lookup_keeps_controller_local_session_id_collisions_window_scoped() {
        let primary = WindowId::from(21);
        let auxiliary = WindowId::from(22);
        let primary_identity = crate::runtime::ExternalDragIdentity { id: 7, epoch: 3 };
        let auxiliary_identity = primary_identity;

        assert_eq!(
            external_drag_owner(primary, Some(primary), [Some(auxiliary)].into_iter()),
            Some(ExternalDragOwner::Primary)
        );
        assert_eq!(
            external_drag_owner(auxiliary, Some(primary), [Some(auxiliary)].into_iter()),
            Some(ExternalDragOwner::Auxiliary(0))
        );
        assert_eq!(primary_identity, auxiliary_identity);
    }
}
