//! Bounded scrollbar edit output with a separate concise offset projection.
use crate::widgets::interaction::{EditEvent, EditTransaction, SliderEditBatch};

/// One to three ordered scrollbar edit events from one admitted input sample.
/// Lifecycle-only boundaries have no concise offset change, including a no-op cancel.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ScrollbarEditBatch {
    events: SliderEditBatch,
    offset_change: Option<f32>,
}

impl ScrollbarEditBatch {
    /// Return the ordered events sharing one edit transaction.
    pub fn events(&self) -> &[EditEvent<f32>] {
        self.events.events()
    }
    /// Return the transaction shared by the events.
    pub const fn transaction(&self) -> EditTransaction {
        self.events.transaction()
    }
    /// Return the effective offset update or meaningful rollback for a concise callback.
    pub const fn offset_change(&self) -> Option<f32> {
        self.offset_change
    }
    pub(crate) fn new(events: &[EditEvent<f32>], offset_change: Option<f32>) -> Option<Self> {
        Some(Self {
            events: SliderEditBatch::from_events(events)?,
            offset_change,
        })
    }
}
