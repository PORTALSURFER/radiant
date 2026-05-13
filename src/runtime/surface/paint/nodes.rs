//! Surface-node paint traversal and node-specific paint helpers.

use super::SurfacePaintContext;
use crate::{
    gui::types::Rect,
    layout::{ContainerKind, LayoutOutput, NodeId},
    runtime::{
        SurfaceContainer, SurfaceNode, SurfaceOverlay,
        paint::{
            SurfacePaintPlan, push_clip_end, push_clip_start, push_container_chrome,
            push_overlay_panel, push_scroll_affordance, scroll_content_clip_rect,
        },
    },
    theme::ThemeTokens,
};

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
            context.active_scroll_affordance == Some(self.id),
        );
    }
}

impl SurfaceOverlay {
    pub(in crate::runtime::surface) fn append_paint(
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
    pub(in crate::runtime::surface) fn append_paint(
        &self,
        layout: &LayoutOutput,
        theme: &ThemeTokens,
        plan: &mut SurfacePaintPlan,
        hovered_container: Option<NodeId>,
        active_scroll_affordance: Option<NodeId>,
    ) {
        let context =
            SurfacePaintContext::new(layout, theme, hovered_container, active_scroll_affordance);
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
