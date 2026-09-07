//! Exact neutral geometry receipt installation for multi-line editor widgets.

use super::SurfaceRuntime;
use crate::{gui::text_layout::editor::TextEditorGeometryReceipt, runtime::RuntimeBridge};

impl<Bridge, Message> SurfaceRuntime<Bridge, Message>
where
    Bridge: RuntimeBridge<Message>,
{
    /// Install a shared editor geometry receipt only on its current, unique widget.
    ///
    /// Hosts call this after admitting their current paint plan and before routing
    /// pointer, keyboard, or IME input. A retired, duplicate, or relaid-out target
    /// is rejected without changing widget state.
    pub fn install_text_editor_geometry(&mut self, receipt: TextEditorGeometryReceipt) -> bool {
        let widget_id = receipt.request().widget_id;
        if self
            .traversal
            .widgets
            .duplicate_widget_ids
            .contains(&widget_id)
            || !self
                .traversal
                .widgets
                .paths
                .current
                .contains_key(&widget_id)
        {
            return false;
        }
        let bounds = self.layout.rect_for(widget_id, self.viewport);
        if !bounds.is_finite() {
            return false;
        }
        let environment = self.surface.resolved_environment().clone();
        self.surface_widget_mut(widget_id).is_some_and(|widget| {
            widget
                .widget_object_mut_runtime()
                .install_text_editor_geometry(receipt, bounds, &environment)
        })
    }
}
