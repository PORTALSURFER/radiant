//! Typed edit reporting for runtime-owned scroll containers.
use super::{ScrollUpdate, SurfaceRuntime};
use crate::{
    gui::types::Vector2,
    layout::NodeId,
    runtime::{Command, CommandOutcome, RuntimeBridge},
    widgets::{EditEvent, EditTransaction},
};

/// One bounded, ordered edit batch from a runtime-owned scroll container.
#[derive(Clone, Copy, PartialEq)]
pub struct ScrollEditBatch {
    node_id: NodeId,
    events: [EditEvent<Vector2>; 3],
    len: u8,
    offset_update: Option<ScrollUpdate>,
}
impl std::fmt::Debug for ScrollEditBatch {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ScrollEditBatch")
            .field("node_id", &self.node_id)
            .field("events", &self.events())
            .field("offset_update", &self.offset_update)
            .finish()
    }
}

impl ScrollEditBatch {
    /// Scroll container whose admitted interaction produced this batch.
    pub const fn node_id(&self) -> NodeId {
        self.node_id
    }
    /// One to three ordered events sharing one edit transaction.
    pub fn events(&self) -> &[EditEvent<Vector2>] {
        &self.events[..usize::from(self.len)]
    }
    /// Transaction shared by the events.
    pub const fn transaction(&self) -> EditTransaction {
        self.events[0].transaction
    }
    /// Effective offset movement or rollback, excluding lifecycle-only boundaries.
    pub const fn offset_update(&self) -> Option<ScrollUpdate> {
        self.offset_update
    }
    pub(in crate::runtime::controller) fn new(
        node_id: NodeId,
        events: &[EditEvent<Vector2>],
        offset_update: Option<ScrollUpdate>,
    ) -> Option<Self> {
        let first = *events.first()?;
        if events.len() > 3
            || events
                .iter()
                .any(|event| event.transaction != first.transaction)
        {
            return None;
        }
        let mut stored = [first; 3];
        stored[..events.len()].copy_from_slice(events);
        Some(Self {
            node_id,
            events: stored,
            len: events.len() as u8,
            offset_update,
        })
    }
}

impl<Bridge, Message> SurfaceRuntime<Bridge, Message>
where
    Bridge: RuntimeBridge<Message>,
{
    pub(in crate::runtime::controller) fn report_scroll_edit(
        &mut self,
        batch: ScrollEditBatch,
        refresh_after_message: bool,
    ) {
        if !self.report_scroll_edit_if_mapped(batch, refresh_after_message)
            && let Some(update) = batch.offset_update()
        {
            self.report_scroll_update_with_refresh(update, refresh_after_message);
        }
    }
    pub(in crate::runtime::controller) fn report_scroll_edit_if_mapped(
        &mut self,
        batch: ScrollEditBatch,
        refresh_after_message: bool,
    ) -> bool {
        let Some(message) = self.surface.root().scroll_edit_message(batch) else {
            return false;
        };
        self.dispatch_scroll_edit_command(Command::Message(message), refresh_after_message);
        true
    }
    fn dispatch_scroll_edit_command(
        &mut self,
        command: Command<Message>,
        refresh_after_message: bool,
    ) {
        let mut deferred = false;
        if refresh_after_message {
            let outcome = self.execute_command(command);
            if !outcome.surface_refresh_requested {
                self.refresh();
            }
        } else {
            let mut outcome = CommandOutcome::default();
            if command.requires_fresh_surface_before_dispatch() {
                outcome.surface_refresh_requested = true;
            }
            self.execute_command_inner_deferred_refresh(command, &mut outcome);
            deferred = outcome.surface_refresh_requested;
            self.pending_input_command_outcome.merge(outcome);
        }
        self.repaint_requested |= !deferred;
    }
}
