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

impl<Message> SurfaceNode<Message> {
    pub(super) fn append_paint(
        &self,
        layout: &LayoutOutput,
        theme: &ThemeTokens,
        plan: &mut SurfacePaintPlan,
        hovered_container: Option<NodeId>,
    ) {
        match self {
            Self::Container(container) => {
                if let Some(style) = container.style {
                    push_container_chrome(
                        &mut plan.primitives,
                        container.id,
                        layout,
                        theme,
                        style,
                        WidgetState {
                            hovered: hovered_container == Some(container.id),
                            ..WidgetState::default()
                        },
                    );
                }
                if container.policy.kind == ContainerKind::ScrollView {
                    if let Some(bounds) = layout.rects.get(&container.id).copied() {
                        push_clip_start(
                            &mut plan.primitives,
                            container.id,
                            scroll_content_clip_rect(container.id, layout, bounds),
                        );
                        for child in &container.children {
                            child
                                .child
                                .append_paint(layout, theme, plan, hovered_container);
                        }
                        push_clip_end(&mut plan.primitives, container.id);
                        if let Some(content_id) =
                            container.children.first().map(|child| child.child.id())
                        {
                            push_scroll_affordance(
                                &mut plan.primitives,
                                container.id,
                                content_id,
                                layout,
                                theme,
                            );
                        }
                    }
                } else {
                    for child in &container.children {
                        child
                            .child
                            .append_paint(layout, theme, plan, hovered_container);
                    }
                }
            }
            Self::Widget(widget) => {
                let Some(bounds) = layout.rects.get(&widget.id()).copied() else {
                    return;
                };
                widget
                    .widget_object()
                    .append_paint(&mut plan.primitives, bounds, layout, theme);
            }
            Self::Overlay(overlay) => {
                push_overlay_panel(
                    &mut plan.primitives,
                    overlay.id,
                    overlay.rect,
                    overlay.label.clone(),
                    theme,
                    overlay.style,
                );
            }
        }
    }
}
