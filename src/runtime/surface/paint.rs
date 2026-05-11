use super::*;
use crate::{
    gui::types::Rect,
    layout::{ContainerKind, LayoutOutput, NodeId},
    runtime::paint::{
        SurfacePaintPlan, push_clip_end, push_clip_start, push_container_chrome,
        push_overlay_panel, push_scroll_affordance, scroll_content_clip_rect,
    },
    theme::ThemeTokens,
    widgets::WidgetState,
};

pub(super) struct SurfacePaintContext<'a> {
    layout: &'a LayoutOutput,
    theme: &'a ThemeTokens,
    hovered_container: Option<NodeId>,
    clip_rect: Option<Rect>,
}

impl<'a> SurfacePaintContext<'a> {
    fn new(
        layout: &'a LayoutOutput,
        theme: &'a ThemeTokens,
        hovered_container: Option<NodeId>,
    ) -> Self {
        Self {
            layout,
            theme,
            hovered_container,
            clip_rect: None,
        }
    }

    fn container_state(&self, node_id: NodeId) -> WidgetState {
        WidgetState {
            hovered: self.hovered_container == Some(node_id),
            ..WidgetState::default()
        }
    }

    fn clipped_to(&self, clip_rect: Rect) -> Self {
        let clip_rect = self
            .clip_rect
            .map(|current| current.clamp_to(clip_rect))
            .unwrap_or(clip_rect);
        Self {
            layout: self.layout,
            theme: self.theme,
            hovered_container: self.hovered_container,
            clip_rect: Some(clip_rect),
        }
    }

    fn should_paint_node(&self, node_id: NodeId) -> bool {
        let Some(clip_rect) = self.clip_rect else {
            return true;
        };
        self.layout
            .rects
            .get(&node_id)
            .is_none_or(|rect| rects_overlap(*rect, clip_rect))
    }

    fn child_is_past_ordered_clip<Message>(
        &self,
        container: &SurfaceContainer<Message>,
        child_id: NodeId,
    ) -> bool {
        let Some(clip_rect) = self.clip_rect else {
            return false;
        };
        if container.policy.kind != ContainerKind::Column {
            return false;
        }
        self.layout
            .rects
            .get(&child_id)
            .is_some_and(|rect| rect.min.y >= clip_rect.max.y)
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

impl<Message> SurfaceContainer<Message> {
    pub(super) fn append_chrome_paint(
        &self,
        context: &SurfacePaintContext<'_>,
        plan: &mut SurfacePaintPlan,
    ) {
        let Some(style) = self.style else {
            return;
        };
        push_container_chrome(
            &mut plan.primitives,
            self.id,
            context.layout,
            context.theme,
            style,
            context.container_state(self.id),
        );
    }

    pub(super) fn is_scroll_view(&self) -> bool {
        self.policy.kind == ContainerKind::ScrollView
    }

    pub(super) fn begin_scroll_clip(
        &self,
        context: &SurfacePaintContext<'_>,
        plan: &mut SurfacePaintPlan,
    ) -> Option<Rect> {
        let bounds = context.layout.rects.get(&self.id).copied()?;
        let clip_rect = scroll_content_clip_rect(self.id, context.layout, bounds);
        push_clip_start(&mut plan.primitives, self.id, clip_rect);
        Some(clip_rect)
    }

    pub(super) fn end_scroll_clip(&self, plan: &mut SurfacePaintPlan) {
        push_clip_end(&mut plan.primitives, self.id);
    }

    pub(super) fn append_scroll_affordance(
        &self,
        context: &SurfacePaintContext<'_>,
        plan: &mut SurfacePaintPlan,
    ) {
        let Some(content_id) = self.children.first().map(|child| child.child.id()) else {
            return;
        };
        push_scroll_affordance(
            &mut plan.primitives,
            self.id,
            content_id,
            context.layout,
            context.theme,
        );
    }
}

impl SurfaceOverlay {
    pub(super) fn append_paint(
        &self,
        context: &SurfacePaintContext<'_>,
        plan: &mut SurfacePaintPlan,
    ) {
        push_overlay_panel(
            &mut plan.primitives,
            self.id,
            self.rect,
            self.label.clone(),
            context.theme,
            self.style,
        );
    }
}

impl<Message> SurfaceNode<Message> {
    pub(super) fn append_paint(
        &self,
        layout: &LayoutOutput,
        theme: &ThemeTokens,
        plan: &mut SurfacePaintPlan,
        hovered_container: Option<NodeId>,
    ) {
        let context = SurfacePaintContext::new(layout, theme, hovered_container);
        self.append_paint_with_context(&context, plan);
    }

    fn append_paint_with_context(
        &self,
        context: &SurfacePaintContext<'_>,
        plan: &mut SurfacePaintPlan,
    ) {
        match self {
            Self::Container(container) => {
                container.append_chrome_paint(context, plan);
                if container.is_scroll_view() {
                    if let Some(clip_rect) = container.begin_scroll_clip(context, plan) {
                        let clipped_context = context.clipped_to(clip_rect);
                        for (index, child) in container.children.iter().enumerate() {
                            if index == 0 {
                                child.child.append_virtual_window_paint_for_scroll(
                                    container.id,
                                    &clipped_context,
                                    plan,
                                );
                            } else if clipped_context.should_paint_node(child.child.id()) {
                                child
                                    .child
                                    .append_paint_with_context(&clipped_context, plan);
                            }
                        }
                        container.end_scroll_clip(plan);
                        container.append_scroll_affordance(context, plan);
                    }
                } else {
                    for child in &container.children {
                        if context.child_is_past_ordered_clip(container, child.child.id()) {
                            break;
                        }
                        if context.should_paint_node(child.child.id()) {
                            child.child.append_paint_with_context(context, plan);
                        }
                    }
                }
            }
            Self::Widget(widget) => {
                let Some(bounds) = context.layout.rects.get(&widget.id()).copied() else {
                    return;
                };
                if !context.should_paint_node(widget.id()) {
                    return;
                }
                widget.widget_object().append_paint(
                    &mut plan.primitives,
                    bounds,
                    context.layout,
                    context.theme,
                );
            }
            Self::Overlay(overlay) => overlay.append_paint(context, plan),
        }
    }

    fn append_virtual_window_paint_for_scroll(
        &self,
        scroll_id: NodeId,
        context: &SurfacePaintContext<'_>,
        plan: &mut SurfacePaintPlan,
    ) {
        let Some(window) = context.layout.virtual_windows.get(&scroll_id) else {
            self.append_paint_with_context(context, plan);
            return;
        };
        let Self::Container(container) = self else {
            self.append_paint_with_context(context, plan);
            return;
        };

        container.append_chrome_paint(context, plan);
        let first = window.first_index.min(container.children.len());
        let last = window
            .last_index_exclusive
            .min(container.children.len())
            .max(first);
        for child in &container.children[first..last] {
            if context.child_is_past_ordered_clip(container, child.child.id()) {
                break;
            }
            if context.should_paint_node(child.child.id()) {
                child.child.append_paint_with_context(context, plan);
            }
        }
    }
}
