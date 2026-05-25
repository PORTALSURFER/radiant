use super::SurfaceRuntime;
use crate::runtime::RuntimeBridge;

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
    /// Requested native DPI scale override from host-visible runtime commands.
    pub dpi_scale_override: Option<crate::theme::DpiScale>,
    /// Requested native-window logical viewport size from host-visible runtime commands.
    pub window_logical_size: Option<crate::layout::Vector2>,
}

impl CommandOutcome {
    pub(in crate::runtime) fn merge(&mut self, other: Self) {
        self.messages_dispatched += other.messages_dispatched;
        self.repaint_requested |= other.repaint_requested;
        self.paint_only_requested |= other.paint_only_requested;
        self.surface_repaint_requested |= other.surface_repaint_requested;
        self.surface_refresh_requested |= other.surface_refresh_requested;
        self.exit_requested |= other.exit_requested;
        self.runtime_work_remaining |= other.runtime_work_remaining;
        self.dpi_scale_override = other.dpi_scale_override.or(self.dpi_scale_override);
        self.window_logical_size = other.window_logical_size.or(self.window_logical_size);
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
