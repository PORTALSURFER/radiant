use super::*;
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
        current_paths: &HashMap<WidgetId, WidgetPath>,
        previous: &Self,
        previous_paths: &HashMap<WidgetId, WidgetPath>,
    ) {
        for (widget_id, current_path) in current_paths {
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
mod tests {
    use super::*;
    use crate::{
        gui::types::{Point, Vector2},
        widgets::{ButtonWidget, PointerButton, ScrollbarAxis, ScrollbarWidget, WidgetSizing},
    };

    #[test]
    fn dispatch_input_at_child_path_routes_without_tree_search() {
        let mut root: SurfaceNode<()> = SurfaceNode::column(
            1,
            0.0,
            vec![
                SurfaceChild::fill(SurfaceNode::widget(
                    ButtonWidget::new(10, "First", WidgetSizing::fixed(Vector2::new(80.0, 28.0))),
                    WidgetMessageMapper::none(),
                )),
                SurfaceChild::fill(SurfaceNode::widget(
                    ButtonWidget::new(20, "Second", WidgetSizing::fixed(Vector2::new(80.0, 28.0))),
                    WidgetMessageMapper::none(),
                )),
            ],
        );

        let result = root.dispatch_input_at_path(
            20,
            &[1],
            Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(80.0, 28.0)),
            WidgetInput::PointerMove {
                position: Point::new(8.0, 8.0),
            },
        );

        assert!(matches!(result, Some(WidgetDispatchResult::NoOutput)));
        assert!(
            root.find_widget(20)
                .expect("target widget exists")
                .widget()
                .common()
                .state
                .hovered
        );
        assert!(
            !root
                .find_widget(10)
                .expect("sibling widget exists")
                .widget()
                .common()
                .state
                .hovered
        );
    }

    #[test]
    fn find_widget_at_child_path_returns_only_the_target_leaf() {
        let root: SurfaceNode<()> = SurfaceNode::column(
            1,
            0.0,
            vec![
                SurfaceChild::fill(SurfaceNode::widget(
                    ButtonWidget::new(10, "First", WidgetSizing::fixed(Vector2::new(80.0, 28.0))),
                    WidgetMessageMapper::none(),
                )),
                SurfaceChild::fill(SurfaceNode::widget(
                    ButtonWidget::new(20, "Second", WidgetSizing::fixed(Vector2::new(80.0, 28.0))),
                    WidgetMessageMapper::none(),
                )),
            ],
        );

        assert_eq!(
            root.find_widget_at_path(&[1])
                .expect("target widget exists")
                .id(),
            20
        );
        assert!(root.find_widget_at_path(&[2]).is_none());
    }

    #[test]
    fn synchronize_widget_state_from_paths_preserves_state_after_reorder() {
        let mut previous: SurfaceNode<()> = SurfaceNode::column(
            1,
            0.0,
            vec![
                SurfaceChild::fill(SurfaceNode::widget(
                    ButtonWidget::new(10, "First", WidgetSizing::fixed(Vector2::new(80.0, 28.0))),
                    WidgetMessageMapper::none(),
                )),
                SurfaceChild::fill(SurfaceNode::widget(
                    ScrollbarWidget::new(
                        20,
                        ScrollbarAxis::Vertical,
                        WidgetSizing::fixed(Vector2::new(16.0, 100.0)),
                    ),
                    WidgetMessageMapper::none(),
                )),
            ],
        );
        let mut current: SurfaceNode<()> = SurfaceNode::column(
            1,
            0.0,
            vec![
                SurfaceChild::fill(SurfaceNode::widget(
                    ScrollbarWidget::new(
                        20,
                        ScrollbarAxis::Vertical,
                        WidgetSizing::fixed(Vector2::new(16.0, 100.0)),
                    ),
                    WidgetMessageMapper::none(),
                )),
                SurfaceChild::fill(SurfaceNode::widget(
                    ButtonWidget::new(10, "First", WidgetSizing::fixed(Vector2::new(80.0, 28.0))),
                    WidgetMessageMapper::none(),
                )),
            ],
        );

        let _ = previous.dispatch_input_at_path(
            20,
            &[1],
            Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(16.0, 100.0)),
            WidgetInput::PointerPress {
                position: Point::new(8.0, 8.0),
                button: PointerButton::Primary,
            },
        );

        let previous_paths = HashMap::from([
            (10, WidgetPath::from_slice(&[0])),
            (20, WidgetPath::from_slice(&[1])),
        ]);
        let current_paths = HashMap::from([
            (20, WidgetPath::from_slice(&[0])),
            (10, WidgetPath::from_slice(&[1])),
        ]);
        current.synchronize_widget_state_from_paths(&current_paths, &previous, &previous_paths);

        let moved = current
            .find_widget_at_path(&[0])
            .expect("moved widget exists")
            .widget()
            .as_any()
            .downcast_ref::<ScrollbarWidget>()
            .expect("moved widget stays a scrollbar");
        assert_eq!(moved.state.drag_grip_fraction, Some(0.08));
    }
}
