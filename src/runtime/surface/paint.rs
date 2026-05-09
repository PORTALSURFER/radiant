use super::*;
use crate::{
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
        }
    }

    fn container_state(&self, node_id: NodeId) -> WidgetState {
        WidgetState {
            hovered: self.hovered_container == Some(node_id),
            ..WidgetState::default()
        }
    }
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
    ) -> bool {
        let Some(bounds) = context.layout.rects.get(&self.id).copied() else {
            return false;
        };
        push_clip_start(
            &mut plan.primitives,
            self.id,
            scroll_content_clip_rect(self.id, context.layout, bounds),
        );
        true
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
                    if container.begin_scroll_clip(context, plan) {
                        for child in &container.children {
                            child.child.append_paint_with_context(context, plan);
                        }
                        container.end_scroll_clip(plan);
                        container.append_scroll_affordance(context, plan);
                    }
                } else {
                    for child in &container.children {
                        child.child.append_paint_with_context(context, plan);
                    }
                }
            }
            Self::Widget(widget) => {
                let Some(bounds) = context.layout.rects.get(&widget.id()).copied() else {
                    return;
                };
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
}
