//! Shared paint traversal context.

use crate::{
    gui::types::Rect,
    layout::{ContainerKind, LayoutOutput, NodeId},
    runtime::SurfaceContainer,
    theme::ThemeTokens,
    widgets::WidgetState,
};

pub(in crate::runtime::surface) struct SurfacePaintContext<'a> {
    pub(super) layout: &'a LayoutOutput,
    pub(super) theme: &'a ThemeTokens,
    pub(super) hovered_container: Option<NodeId>,
    pub(super) active_scroll_affordance: Option<NodeId>,
    pub(super) clip_rect: Option<Rect>,
}

impl<'a> SurfacePaintContext<'a> {
    pub(super) fn new(
        layout: &'a LayoutOutput,
        theme: &'a ThemeTokens,
        hovered_container: Option<NodeId>,
        active_scroll_affordance: Option<NodeId>,
    ) -> Self {
        Self {
            layout,
            theme,
            hovered_container,
            active_scroll_affordance,
            clip_rect: None,
        }
    }

    pub(super) fn container_state(&self, node_id: NodeId) -> WidgetState {
        WidgetState {
            hovered: self.hovered_container == Some(node_id),
            ..WidgetState::default()
        }
    }

    pub(super) fn clipped_to(&self, clip_rect: Rect) -> Self {
        let clip_rect = self
            .clip_rect
            .map(|current| current.clamp_to(clip_rect))
            .unwrap_or(clip_rect);
        Self {
            layout: self.layout,
            theme: self.theme,
            hovered_container: self.hovered_container,
            active_scroll_affordance: self.active_scroll_affordance,
            clip_rect: Some(clip_rect),
        }
    }

    pub(super) fn should_paint_node(&self, node_id: NodeId) -> bool {
        let Some(clip_rect) = self.clip_rect else {
            return true;
        };
        self.layout
            .rects
            .get(&node_id)
            .is_none_or(|rect| rects_overlap(*rect, clip_rect))
    }

    pub(super) fn child_is_past_ordered_clip<Message>(
        &self,
        container: &SurfaceContainer<Message>,
        child_id: NodeId,
    ) -> bool {
        let Some(clip_rect) = self.clip_rect else {
            return false;
        };
        let Some(rect) = self.layout.rects.get(&child_id) else {
            return false;
        };
        match container.policy.kind {
            ContainerKind::Column => rect.min.y >= clip_rect.max.y,
            ContainerKind::Row => rect.min.x >= clip_rect.max.x,
            _ => false,
        }
    }
}

fn rects_overlap(a: Rect, b: Rect) -> bool {
    a.width() > 0.0
        && a.height() > 0.0
        && b.width() > 0.0
        && b.height() > 0.0
        && a.min.x < b.max.x
        && a.max.x > b.min.x
        && a.min.y < b.max.y
        && a.max.y > b.min.y
}
