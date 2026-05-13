use super::{SurfaceWidget, UiSurface, WidgetPath};
use crate::widgets::WidgetId;

impl<Message> UiSurface<Message> {
    /// Find one projected widget by stable id.
    pub fn find_widget(&self, widget_id: WidgetId) -> Option<&SurfaceWidget<Message>> {
        self.root.find_widget(widget_id)
    }

    pub(in crate::runtime) fn find_widget_at_path(
        &self,
        widget_id: WidgetId,
        child_path: &WidgetPath,
    ) -> Option<&SurfaceWidget<Message>> {
        self.root
            .find_widget_at_path(child_path.as_slice())
            .filter(|widget| widget.id() == widget_id)
    }

    /// Find one projected widget by stable id for in-place runtime interaction.
    pub fn find_widget_mut(&mut self, widget_id: WidgetId) -> Option<&mut SurfaceWidget<Message>> {
        self.root.find_widget_mut(widget_id)
    }

    pub(in crate::runtime) fn find_widget_mut_at_path(
        &mut self,
        widget_id: WidgetId,
        child_path: &WidgetPath,
    ) -> Option<&mut SurfaceWidget<Message>> {
        self.root
            .find_widget_mut_at_path(child_path.as_slice())
            .filter(|widget| widget.id() == widget_id)
    }

    /// Return whether a projected widget can own runtime focus.
    pub fn is_focusable_widget(&self, widget_id: WidgetId) -> bool {
        self.find_widget(widget_id)
            .is_some_and(SurfaceWidget::is_focusable)
    }
}
