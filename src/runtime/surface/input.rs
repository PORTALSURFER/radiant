use super::*;
use crate::{
    gui::types::Rect,
    widgets::{WidgetId, WidgetInput, WidgetOutput},
};
use std::collections::BTreeMap;

pub(in crate::runtime) enum WidgetDispatchResult<Message> {
    NoOutput,
    UnmappedOutput,
    Message(Message),
}

impl<Message> SurfaceNode<Message> {
    pub(super) fn collect_widgets<'a>(
        &'a self,
        widgets: &mut BTreeMap<WidgetId, &'a SurfaceWidget<Message>>,
    ) {
        match self {
            Self::Container(container) => {
                for child in &container.children {
                    child.child.collect_widgets(widgets);
                }
            }
            Self::Widget(widget) => {
                widgets.entry(widget.id()).or_insert(widget);
            }
            Self::Overlay(_) => {}
        }
    }

    pub(super) fn synchronize_widget_state_from(
        &mut self,
        previous_widgets: &BTreeMap<WidgetId, &SurfaceWidget<Message>>,
    ) {
        match self {
            Self::Container(container) => {
                for child in &mut container.children {
                    child.child.synchronize_widget_state_from(previous_widgets);
                }
            }
            Self::Widget(widget) => {
                if let Some(previous_widget) = previous_widgets.get(&widget.id()) {
                    widget
                        .widget_object_mut()
                        .synchronize_from_previous(previous_widget.widget_object());
                }
            }
            Self::Overlay(_) => {}
        }
    }

    pub(super) fn handle_input(
        &mut self,
        widget_id: WidgetId,
        bounds: Rect,
        input: WidgetInput,
    ) -> Option<WidgetOutput> {
        self.find_widget_mut(widget_id)
            .and_then(|widget| widget.handle_input(widget_id, bounds, input))
    }

    pub(super) fn dispatch_output(
        &self,
        widget_id: WidgetId,
        output: &WidgetOutput,
    ) -> Option<Message> {
        match self {
            Self::Container(container) => container
                .children
                .iter()
                .find_map(|child| child.child.dispatch_output(widget_id, output)),
            Self::Widget(widget) => widget.dispatch_output(widget_id, output.clone()),
            Self::Overlay(_) => None,
        }
    }

    pub(super) fn find_widget(&self, widget_id: WidgetId) -> Option<&SurfaceWidget<Message>> {
        match self {
            Self::Container(container) => container
                .children
                .iter()
                .find_map(|child| child.child.find_widget(widget_id)),
            Self::Widget(widget) => (widget.id() == widget_id).then_some(widget),
            Self::Overlay(_) => None,
        }
    }

    pub(super) fn find_widget_mut(
        &mut self,
        widget_id: WidgetId,
    ) -> Option<&mut SurfaceWidget<Message>> {
        match self {
            Self::Container(container) => container
                .children
                .iter_mut()
                .find_map(|child| child.child.find_widget_mut(widget_id)),
            Self::Widget(widget) => (widget.id() == widget_id).then_some(widget),
            Self::Overlay(_) => None,
        }
    }
}
