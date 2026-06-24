use super::SurfaceRuntime;
use crate::{
    gui::types::Point,
    layout::NodeId,
    runtime::{RuntimeBridge, SurfaceWidget},
    widgets::{PointerCapturePolicy, WidgetCursor, WidgetId, WidgetInput},
};

impl<Bridge, Message> SurfaceRuntime<Bridge, Message>
where
    Bridge: RuntimeBridge<Message>,
{
    /// Return the first projected widget whose laid-out bounds contain `point`.
    pub fn widget_at(&self, point: Point) -> Option<WidgetId> {
        self.traversal
            .widgets
            .pointer
            .visible()
            .iter()
            .rev()
            .copied()
            .find(|widget_id| self.widget_contains_point(*widget_id, point))
    }

    pub(super) fn widget_at_for_input(
        &self,
        point: Point,
        input: &WidgetInput,
    ) -> Option<WidgetId> {
        self.traversal
            .widgets
            .pointer
            .visible()
            .iter()
            .rev()
            .copied()
            .find(|widget_id| {
                self.widget_contains_point(*widget_id, point)
                    && self.widget_accepts_pointer_input(*widget_id, input)
            })
    }

    pub(super) fn pointer_widget_at_for_move(&self, point: Point) -> Option<WidgetId> {
        let input = WidgetInput::PointerMove { position: point };
        self.stable_hovered_widget_at(point, &input)
            .or_else(|| self.widget_at_for_input(point, &input))
    }

    /// Return the cursor requested by the active pointer target at `point`.
    pub fn cursor_at(&self, point: Point) -> WidgetCursor {
        self.interaction
            .pointer
            .capture
            .or_else(|| self.pointer_widget_at_for_move(point))
            .and_then(|widget_id| {
                let bounds = *self.layout.rects.get(&widget_id)?;
                self.surface_widget(widget_id)
                    .and_then(|widget| widget.cursor_for_point(bounds, point))
            })
            .unwrap_or_default()
    }

    pub(super) fn styled_container_at(&self, point: Point) -> Option<NodeId> {
        self.traversal
            .containers
            .styled
            .visible()
            .iter()
            .rev()
            .copied()
            .find(|node_id| self.container_contains_point(*node_id, point))
    }

    pub(super) fn widget_clip_contains_point(&self, widget_id: WidgetId, point: Point) -> bool {
        self.traversal
            .widgets
            .paths
            .clip_ancestors
            .get(&widget_id)
            .is_none_or(|clip_nodes| {
                clip_nodes.as_slice().iter().all(|node_id| {
                    self.layout
                        .rects
                        .get(node_id)
                        .is_some_and(|rect| rect.contains(point))
                })
            })
    }

    pub(super) fn container_clip_contains_point(&self, node_id: NodeId, point: Point) -> bool {
        self.traversal
            .containers
            .clip_ancestors
            .get(&node_id)
            .is_none_or(|clip_nodes| {
                clip_nodes.as_slice().iter().all(|clip_node| {
                    self.layout
                        .rects
                        .get(clip_node)
                        .is_some_and(|rect| rect.contains(point))
                })
            })
    }

    pub(super) fn widget_suppresses_container_hover(&self, widget_id: Option<WidgetId>) -> bool {
        let Some(widget_id) = widget_id else {
            return false;
        };
        self.traversal
            .widgets
            .paths
            .container_hover_suppression
            .contains(&widget_id)
    }

    pub(super) fn widget_accepts_stable_pointer_move(&self, widget_id: WidgetId) -> bool {
        self.surface_widget(widget_id)
            .is_some_and(SurfaceWidget::accepts_pointer_move)
    }

    pub(super) fn widget_accepts_pointer_input(
        &self,
        widget_id: WidgetId,
        input: &WidgetInput,
    ) -> bool {
        self.surface_widget(widget_id)
            .is_some_and(|widget| widget.accepts_pointer_input(input))
    }

    pub(super) fn widget_allows_captured_pointer_pass_through(&self, widget_id: WidgetId) -> bool {
        self.widget_pointer_capture_policy(widget_id) == PointerCapturePolicy::PassThrough
    }

    pub(super) fn widget_pointer_capture_policy(
        &self,
        widget_id: WidgetId,
    ) -> PointerCapturePolicy {
        self.surface_widget(widget_id)
            .map(SurfaceWidget::pointer_capture_policy)
            .unwrap_or(PointerCapturePolicy::Exclusive)
    }

    pub(crate) fn widget_prefers_pointer_move_paint_only(&self, widget_id: WidgetId) -> bool {
        self.surface_widget(widget_id)
            .is_some_and(SurfaceWidget::prefers_pointer_move_paint_only)
    }

    fn stable_hovered_widget_at(&self, point: Point, input: &WidgetInput) -> Option<WidgetId> {
        let hovered = self.interaction.hover.widget?;
        if !self.widget_contains_point(hovered, point) {
            return None;
        }
        self.traversal
            .widgets
            .pointer
            .visible_after(hovered)
            .iter()
            .rev()
            .copied()
            .find(|widget_id| {
                self.widget_contains_point(*widget_id, point)
                    && self.widget_accepts_pointer_input(*widget_id, input)
            })
            .or_else(|| {
                self.widget_accepts_pointer_input(hovered, input)
                    .then_some(hovered)
            })
    }

    pub(in crate::runtime::controller) fn widget_contains_point(
        &self,
        widget_id: WidgetId,
        point: Point,
    ) -> bool {
        self.layout
            .rects
            .get(&widget_id)
            .is_some_and(|rect| rect.contains(point))
            && self.widget_clip_contains_point(widget_id, point)
    }

    fn container_contains_point(&self, node_id: NodeId, point: Point) -> bool {
        self.layout
            .rects
            .get(&node_id)
            .is_some_and(|rect| rect.contains(point))
            && self.container_clip_contains_point(node_id, point)
    }
}
