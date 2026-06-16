use crate::{
    runtime::{RuntimeBridge, SurfaceRuntime, empty_paint_plan_for_layout},
    theme::ThemeTokens,
};

use super::{DevtoolsNodeSnapshot, DevtoolsOverlayOptions, DevtoolsSnapshot, overlay};

impl<Bridge, Message> SurfaceRuntime<Bridge, Message>
where
    Bridge: RuntimeBridge<Message>,
{
    /// Return a backend-neutral devtools snapshot for the current runtime frame.
    pub fn devtools_snapshot(&self) -> DevtoolsSnapshot {
        self.devtools_snapshot_with_theme(&ThemeTokens::default())
    }

    /// Return a backend-neutral devtools snapshot using a caller-provided theme.
    pub fn devtools_snapshot_with_theme(&self, theme: &ThemeTokens) -> DevtoolsSnapshot {
        let mut plan = empty_paint_plan_for_layout(self.context().layout, theme);
        self.base_paint_plan_into(theme, &mut plan);
        DevtoolsSnapshot {
            viewport: self.context().viewport,
            root: self
                .surface()
                .root()
                .devtools_snapshot_node(self.pointer_capture(), self.context().layout),
            selected_node_id: self
                .pointer_capture()
                .or_else(|| self.focused_widget())
                .or_else(|| self.hovered_widget())
                .or_else(|| self.hovered_container())
                .or_else(|| self.hovered_scroll_affordance()),
            paint: plan.stats(),
            diagnostics: self.runtime_diagnostics(),
        }
    }

    /// Configure runtime devtools overlay paint.
    pub fn set_devtools_overlay_options(&mut self, options: DevtoolsOverlayOptions) {
        self.devtools_overlay = options;
        self.repaint_requested |= options.enabled;
    }

    /// Return current runtime devtools overlay paint options.
    pub const fn devtools_overlay_options(&self) -> DevtoolsOverlayOptions {
        self.devtools_overlay
    }

    pub(in crate::runtime) fn append_devtools_overlay_paint(
        &self,
        theme: &ThemeTokens,
        primitives: &mut Vec<crate::runtime::PaintPrimitive>,
    ) {
        if !self.devtools_overlay.enabled {
            return;
        }

        let snapshot = self.devtools_snapshot_with_theme(theme);
        overlay::append_devtools_overlay(&snapshot, theme, primitives);
    }
}

impl DevtoolsSnapshot {
    /// Return the currently selected devtools node, when present.
    pub fn selected_node(&self) -> Option<&DevtoolsNodeSnapshot> {
        self.selected_node_id
            .and_then(|node_id| self.root.find_node(node_id))
    }

    pub(super) fn selected_node_bounds(&self) -> Option<crate::gui::types::Rect> {
        self.selected_node().and_then(|node| node.bounds)
    }
}
