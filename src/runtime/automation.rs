//! Backend-neutral automation snapshot extraction for runtime surfaces.

use crate::{
    gui::automation::{GuiAutomationSnapshot, GuiAutomationTargetSnapshot},
    runtime::{RuntimeBridge, SurfaceRuntime},
};

impl<Bridge, Message> SurfaceRuntime<Bridge, Message>
where
    Bridge: RuntimeBridge<Message>,
{
    /// Return a serializable backend-neutral automation snapshot for the current surface.
    pub fn automation_snapshot(&self) -> GuiAutomationSnapshot {
        let viewport = self.context().viewport;
        let root = self
            .surface()
            .root()
            .automation_snapshot_node(self.context().layout);

        GuiAutomationSnapshot {
            schema_version: 2,
            viewport_width: viewport.width().max(0.0).round() as u32,
            viewport_height: viewport.height().max(0.0).round() as u32,
            root,
        }
    }

    /// Return a flattened, coordinate-bearing automation target snapshot for the
    /// current surface.
    pub fn automation_target_snapshot(&self) -> GuiAutomationTargetSnapshot {
        self.automation_snapshot().target_snapshot()
    }
}
