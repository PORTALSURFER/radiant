use super::SurfaceRuntime;
use crate::runtime::RuntimeBridge;
use crate::runtime::{RepaintScope, SurfaceInvalidation};

/// Summary of one command-dispatch pass through a [`SurfaceRuntime`].
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct CommandOutcome {
    /// Number of host-defined messages reduced during this pass.
    pub messages_dispatched: usize,
    /// Whether any command requested a repaint.
    pub repaint_requested: bool,
    /// Whether any command requested a redraw without surface reprojection.
    pub paint_only_requested: bool,
    /// Whether any command requested a repaint of the projected surface.
    pub surface_repaint_requested: bool,
    /// Whether this pass selected a declarative surface refresh stage.
    pub surface_refresh_requested: bool,
    /// Narrowest correctness-preserving refresh scope selected for this pass.
    pub surface_refresh_scope: Option<RepaintScope>,
    /// Whether that refresh was already applied during message dispatch.
    pub surface_refresh_applied: bool,
    /// Whether any command requested runtime exit.
    pub exit_requested: bool,
    /// Whether runtime-owned background work still has queued commands/messages.
    ///
    /// Native backends use this to keep the UI/event/render owner responsive:
    /// one drain pass handles a bounded slice of background commands/messages,
    /// then schedules another wakeup instead of monopolizing the UI path.
    pub runtime_work_remaining: bool,
    /// Requested native DPI scale override from host-visible runtime commands.
    pub dpi_scale_override: Option<crate::theme::DpiScale>,
    /// Requested native-window logical viewport size from host-visible runtime commands.
    pub window_logical_size: Option<crate::layout::Vector2>,
}

impl CommandOutcome {
    pub(in crate::runtime) fn merge(&mut self, other: Self) {
        let refresh_requested = self.surface_refresh_requested;
        let refresh_applied = self.surface_refresh_applied;
        self.messages_dispatched += other.messages_dispatched;
        self.repaint_requested |= other.repaint_requested;
        self.paint_only_requested |= other.paint_only_requested;
        self.surface_repaint_requested |= other.surface_repaint_requested;
        let other_refresh_requested = other.surface_refresh_requested;
        self.surface_refresh_requested |= other_refresh_requested;
        self.surface_refresh_scope = match (self.surface_refresh_scope, other.surface_refresh_scope)
        {
            (Some(current), Some(next)) => Some(current.merge(next)),
            (Some(scope), None) | (None, Some(scope)) => Some(scope),
            (None, None) => None,
        };
        self.surface_refresh_applied = match (refresh_requested, other_refresh_requested) {
            (false, false) => false,
            (true, false) => refresh_applied,
            (false, true) => other.surface_refresh_applied,
            (true, true) => refresh_applied && other.surface_refresh_applied,
        };
        self.exit_requested |= other.exit_requested;
        self.runtime_work_remaining |= other.runtime_work_remaining;
        self.dpi_scale_override = other.dpi_scale_override.or(self.dpi_scale_override);
        self.window_logical_size = other.window_logical_size.or(self.window_logical_size);
    }

    /// Return the typed invalidation stage selected by this command pass.
    pub const fn surface_invalidation(&self) -> SurfaceInvalidation {
        SurfaceInvalidation::from_repaint_scope(self.surface_refresh_scope)
    }
}

impl<Bridge, Message> SurfaceRuntime<Bridge, Message>
where
    Bridge: RuntimeBridge<Message>,
{
    pub(crate) fn take_pending_input_command_outcome(&mut self) -> CommandOutcome {
        std::mem::take(&mut self.pending_input_command_outcome)
    }

    pub(super) fn finish_command_outcome(&mut self, outcome: CommandOutcome) -> CommandOutcome {
        self.refresh_if_requested(
            outcome.surface_refresh_requested && !outcome.surface_refresh_applied,
            outcome.surface_refresh_scope,
        );
        if outcome.surface_refresh_requested {
            self.repaint_requested = true;
        }
        if outcome.exit_requested {
            self.exit_requested = true;
        }
        outcome
    }

    fn refresh_if_requested(&mut self, requested: bool, scope: Option<RepaintScope>) {
        if requested {
            self.refresh_with_scope(scope.unwrap_or(RepaintScope::Surface));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::CommandOutcome;
    use crate::runtime::RepaintScope;

    #[test]
    fn merging_refresh_outcomes_only_marks_the_result_applied_when_all_work_was_applied() {
        let mut pending = CommandOutcome {
            surface_refresh_requested: true,
            surface_refresh_scope: Some(RepaintScope::Surface),
            surface_refresh_applied: false,
            ..CommandOutcome::default()
        };
        pending.merge(CommandOutcome {
            surface_refresh_requested: true,
            surface_refresh_scope: Some(RepaintScope::Projection),
            surface_refresh_applied: true,
            ..CommandOutcome::default()
        });

        assert_eq!(pending.surface_refresh_scope, Some(RepaintScope::Surface));
        assert!(!pending.surface_refresh_applied);
    }
}
