use super::SurfaceRuntime;
use crate::{
    gui::types::Rect,
    runtime::{RuntimeBridge, SurfaceWidget, WidgetDispatchResult},
    widgets::{CompositionSample, WidgetId, WidgetInput},
};

impl<Bridge, Message> SurfaceRuntime<Bridge, Message>
where
    Bridge: RuntimeBridge<Message>,
{
    pub(super) fn dispatch_surface_input(
        &mut self,
        widget_id: WidgetId,
        bounds: Rect,
        input: WidgetInput,
    ) -> Option<WidgetDispatchResult<Message>> {
        let Some(child_path) = self.traversal.widgets.paths.current.get(&widget_id) else {
            return self
                .surface
                .dispatch_widget_input_message(widget_id, bounds, input);
        };
        self.surface
            .dispatch_widget_input_message_at_path(widget_id, child_path, bounds, input)
    }

    pub(super) fn dispatch_surface_composition_sample(
        &mut self,
        widget_id: WidgetId,
        sample: CompositionSample,
    ) -> Option<(WidgetDispatchResult<Message>, bool)> {
        if let Some(child_path) = self.traversal.widgets.paths.current.get(&widget_id) {
            self.surface
                .dispatch_widget_composition_sample_message_at_path(widget_id, child_path, sample)
        } else {
            self.surface
                .dispatch_widget_composition_sample_message(widget_id, sample)
        }
    }

    pub(super) fn dispatch_surface_pointer_capture_cancelled(
        &mut self,
        widget_id: WidgetId,
        bounds: Rect,
    ) -> Option<WidgetDispatchResult<Message>> {
        let Some(child_path) = self.traversal.widgets.paths.current.get(&widget_id) else {
            return self
                .surface
                .dispatch_widget_pointer_capture_cancelled_message(widget_id, bounds);
        };
        self.surface
            .dispatch_widget_pointer_capture_cancelled_message_at_path(
                widget_id, child_path, bounds,
            )
    }

    pub(super) fn surface_widget(&self, widget_id: WidgetId) -> Option<&SurfaceWidget<Message>> {
        self.traversal
            .widgets
            .paths
            .current
            .get(&widget_id)
            .and_then(|child_path| self.surface.find_widget_at_path(widget_id, child_path))
            .or_else(|| self.surface.find_widget(widget_id))
    }

    pub(super) fn surface_widget_mut(
        &mut self,
        widget_id: WidgetId,
    ) -> Option<&mut SurfaceWidget<Message>> {
        let surface = &mut self.surface;
        if let Some(child_path) = self.traversal.widgets.paths.current.get(&widget_id) {
            return surface.find_widget_mut_at_path(widget_id, child_path);
        }
        surface.find_widget_mut(widget_id)
    }
}
