use super::{SurfaceNode, SurfaceWidget, WidgetPath};
use crate::{
    gui::types::Rect,
    widgets::{WidgetId, WidgetInput, WidgetOutput},
};
use std::collections::HashMap;

pub(in crate::runtime) enum WidgetDispatchResult<Message> {
    NoOutput,
    UnmappedOutput,
    Message(Message),
}

impl<Message> SurfaceNode<Message> {
    pub(super) fn synchronize_widget_state_from_paths(
        &mut self,
        stateful_widget_order: &[WidgetId],
        current_paths: &HashMap<WidgetId, WidgetPath>,
        previous: &Self,
        previous_paths: &HashMap<WidgetId, WidgetPath>,
    ) {
        for widget_id in stateful_widget_order {
            let Some(current_path) = current_paths.get(widget_id) else {
                continue;
            };
            let Some(previous_path) = previous_paths.get(widget_id) else {
                continue;
            };
            let Some(previous_widget) = previous
                .find_widget_at_path(previous_path.as_slice())
                .filter(|widget| widget.id() == *widget_id)
            else {
                continue;
            };
            let Some(current_widget) = self
                .find_widget_mut_at_path(current_path.as_slice())
                .filter(|widget| widget.id() == *widget_id)
            else {
                continue;
            };
            current_widget
                .widget_object_mut()
                .synchronize_from_previous(previous_widget.widget_object());
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

    pub(super) fn handle_input_at_path(
        &mut self,
        widget_id: WidgetId,
        child_path: &[usize],
        bounds: Rect,
        input: WidgetInput,
    ) -> Option<WidgetOutput> {
        self.find_widget_mut_at_path(child_path)
            .filter(|widget| widget.id() == widget_id)
            .and_then(|widget| widget.handle_input(widget_id, bounds, input))
    }

    pub(super) fn dispatch_input_at_path(
        &mut self,
        widget_id: WidgetId,
        child_path: &[usize],
        bounds: Rect,
        input: WidgetInput,
    ) -> Option<WidgetDispatchResult<Message>> {
        self.find_widget_mut_at_path(child_path)
            .filter(|widget| widget.id() == widget_id)
            .map(|widget| widget.dispatch_input(widget_id, bounds, input))
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
            Self::Widget(widget) if widget.id() == widget_id => {
                widget.dispatch_output(widget_id, output.clone())
            }
            Self::Widget(_) => None,
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

    pub(super) fn find_widget_at_path(
        &self,
        child_path: &[usize],
    ) -> Option<&SurfaceWidget<Message>> {
        match (self, child_path.split_first()) {
            (Self::Widget(widget), None) => Some(widget),
            (Self::Container(container), Some((child_index, remaining_path))) => container
                .children
                .get(*child_index)?
                .child
                .find_widget_at_path(remaining_path),
            _ => None,
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

    pub(super) fn find_widget_mut_at_path(
        &mut self,
        child_path: &[usize],
    ) -> Option<&mut SurfaceWidget<Message>> {
        match (self, child_path.split_first()) {
            (Self::Widget(widget), None) => Some(widget),
            (Self::Container(container), Some((child_index, remaining_path))) => container
                .children
                .get_mut(*child_index)?
                .child
                .find_widget_mut_at_path(remaining_path),
            _ => None,
        }
    }
}

#[cfg(test)]
#[path = "input/tests.rs"]
mod tests;
