use super::*;

/// Summary of one command-dispatch pass through a [`SurfaceRuntime`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CommandOutcome {
    /// Number of host-defined messages reduced during this pass.
    pub messages_dispatched: usize,
    /// Whether any command requested a repaint.
    pub repaint_requested: bool,
    /// Whether any command requested a redraw without surface reprojection.
    pub paint_only_requested: bool,
    /// Whether any command requested a repaint of the projected surface.
    pub surface_repaint_requested: bool,
    /// Whether this pass requires declarative surface reprojection and layout.
    pub surface_refresh_requested: bool,
    /// Whether any command requested runtime exit.
    pub exit_requested: bool,
    /// Whether runtime-owned background work still has queued commands/messages.
    ///
    /// Native backends use this to keep the UI/event/render owner responsive:
    /// one drain pass handles a bounded slice of background commands/messages,
    /// then schedules another wakeup instead of monopolizing the UI path.
    pub runtime_work_remaining: bool,
}

impl<Bridge, Message> SurfaceRuntime<Bridge, Message>
where
    Bridge: RuntimeBridge<Message>,
{
    pub(super) fn finish_command_outcome(&mut self, outcome: CommandOutcome) -> CommandOutcome {
        self.refresh_if_requested(outcome.surface_refresh_requested);
        if outcome.surface_refresh_requested {
            self.repaint_requested = true;
        }
        if outcome.exit_requested {
            self.exit_requested = true;
        }
        outcome
    }

    fn refresh_if_requested(&mut self, requested: bool) {
        if requested {
            self.refresh();
        }
    }
}
