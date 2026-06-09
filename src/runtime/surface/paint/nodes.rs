//! Surface-node paint traversal and node-specific paint helpers.

use super::SurfacePaintContext;
use crate::{
    gui::types::Rect,
    layout::{ContainerKind, LayoutOutput, NodeId, OverflowPolicy},
    runtime::{
        PaintPrimitive, SurfaceContainer, SurfaceNode, SurfaceOverlay,
        paint::{
            PaintClipEnd, PaintClipStart, SurfacePaintPlan, push_container_chrome,
            push_layout_debug_overlay_for_node, push_overlay_panel,
        },
    },
    theme::ThemeTokens,
    widgets::PaintBounds,
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

    pub(super) fn child_clip_rect(&self, context: &SurfacePaintContext<'_>) -> Option<Rect> {
        if self.children.is_empty()
            || self.is_scroll_view()
            || self.policy.overflow != OverflowPolicy::Clip
        {
            return None;
        }
        context
            .layout
            .rects
            .get(&self.id)
            .copied()
            .filter(|rect| rect.has_finite_positive_area())
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
            Self::Scene(scene) => {
                if context.should_paint_node(scene.base.id()) {
                    scene.base.append_paint_with_context(context, plan);
                }
                for layer in scene.ordered_layers() {
                    if let Some(input) = &layer.input
                        && context.should_paint_node(input.id())
                    {
                        input.append_paint_with_context(context, plan);
                    }
                    if context.should_paint_node(layer.node.id()) {
                        layer.node.append_paint_with_context(context, plan);
                    }
                }
                push_layout_debug_overlay_for_node(context.layout, plan, scene.id);
            }
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
                    push_layout_debug_overlay_for_node(context.layout, plan, container.id);
                } else if let Some(clip_rect) = container.child_clip_rect(context) {
                    plan.primitives
                        .push(PaintPrimitive::ClipStart(PaintClipStart {
                            node_id: container.id,
                            rect: clip_rect,
                        }));
                    let clipped_context = context.clipped_to(clip_rect);
                    for child in &container.children {
                        if clipped_context.child_is_past_ordered_clip(container, child.child.id()) {
                            break;
                        }
                        if clipped_context.should_paint_node(child.child.id()) {
                            child
                                .child
                                .append_paint_with_context(&clipped_context, plan);
                        }
                    }
                    push_layout_debug_overlay_for_node(context.layout, plan, container.id);
                    plan.primitives.push(PaintPrimitive::ClipEnd(PaintClipEnd {
                        node_id: container.id,
                    }));
                } else {
                    for child in &container.children {
                        if context.child_is_past_ordered_clip(container, child.child.id()) {
                            break;
                        }
                        if context.should_paint_node(child.child.id()) {
                            child.child.append_paint_with_context(context, plan);
                        }
                    }
                    push_layout_debug_overlay_for_node(context.layout, plan, container.id);
                }
            }
            Self::Widget(widget) => {
                let Some(bounds) = context.layout.rects.get(&widget.id()).copied() else {
                    return;
                };
                if !context.should_paint_node(widget.id()) {
                    return;
                }
                if widget.widget_object().common().paint.bounds == PaintBounds::ClipToRect {
                    plan.primitives
                        .push(PaintPrimitive::ClipStart(PaintClipStart {
                            node_id: widget.id(),
                            rect: bounds,
                        }));
                }
                widget.widget_object().append_paint(
                    &mut plan.primitives,
                    bounds,
                    context.layout,
                    context.theme,
                );
                push_layout_debug_overlay_for_node(context.layout, plan, widget.id());
                if widget.widget_object().common().paint.bounds == PaintBounds::ClipToRect {
                    plan.primitives.push(PaintPrimitive::ClipEnd(PaintClipEnd {
                        node_id: widget.id(),
                    }));
                }
            }
            Self::Overlay(overlay) => {
                overlay.append_paint(context, plan);
                push_layout_debug_overlay_for_node(context.layout, plan, overlay.id);
            }
            Self::FloatingLayer(layer) => {
                layer.container.append_chrome_paint(context, plan);
                for child in &layer.container.children {
                    if context.should_paint_node(child.child.id()) {
                        child.child.append_paint_with_context(context, plan);
                    }
                }
                push_layout_debug_overlay_for_node(context.layout, plan, layer.container.id);
            }
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
