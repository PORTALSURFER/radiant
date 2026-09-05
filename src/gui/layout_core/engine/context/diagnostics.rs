use super::LayoutContext;
use crate::gui::layout_core::engine::{
    DebugPrimitiveKind, LayoutDebugPrimitive, LayoutDiagnostic, LayoutDiagnosticCode, OverflowInfo,
    VirtualWindowInfo,
};
use crate::gui::layout_core::model::{Insets, OverflowPolicy};
use crate::gui::layout_core::tree::LayoutNode;
use crate::gui::layout_core::tree::NodeId;
use crate::gui::types::Rect;
use std::borrow::Cow;

impl<'a> LayoutContext<'a> {
    pub(crate) fn record_overflow(
        &mut self,
        node_id: NodeId,
        policy: OverflowPolicy,
        x: bool,
        y: bool,
    ) {
        self.output.overflowed.insert(node_id);
        self.output
            .overflow_flags
            .insert(node_id, OverflowInfo { x, y, policy });
        self.push_diagnostic(
            node_id,
            LayoutDiagnosticCode::OverflowOccurred,
            "node overflowed available space",
        );
        if self.debug_options.show_overflow
            && let Some(rect) = self.output.rects.get(&node_id).copied()
        {
            self.record_debug(node_id, DebugPrimitiveKind::OverflowMarker, rect);
        }
    }

    pub(crate) fn record_node_bounds(&mut self, node_id: NodeId, rect: Rect) {
        if self.debug_options.show_bounds {
            self.record_debug(node_id, DebugPrimitiveKind::NodeBounds, rect);
        }
        if self.debug_options.show_measured
            && let Some(measured) = self.measured_by_node.get(&node_id).copied()
        {
            let measured_rect = Rect::from_min_size(
                rect.min,
                crate::gui::types::Vector2::new(
                    measured.x.max(0.0).min(rect.width().max(0.0)),
                    measured.y.max(0.0).min(rect.height().max(0.0)),
                ),
            );
            self.record_debug(node_id, DebugPrimitiveKind::MeasuredBounds, measured_rect);
        }
    }

    pub(crate) fn record_content_bounds(&mut self, node_id: NodeId, rect: Rect) {
        if self.debug_options.show_padding {
            self.record_debug(node_id, DebugPrimitiveKind::ContentBounds, rect);
        }
    }

    pub(crate) fn record_slot_margin(&mut self, node_id: NodeId, child_rect: Rect, margin: Insets) {
        if !self.debug_options.show_margins {
            return;
        }
        if let Some(margin_rect) = crate::gui::layout_core::validated_geometry::checked_inset_rect(
            child_rect,
            Insets {
                left: -margin.left,
                right: -margin.right,
                top: -margin.top,
                bottom: -margin.bottom,
            },
        ) {
            self.record_debug(node_id, DebugPrimitiveKind::SlotMargin, margin_rect);
        }
    }

    pub(crate) fn record_viewport_bounds(&mut self, node_id: NodeId, rect: Rect) {
        if crate::gui::layout_core::validated_geometry::ValidatedRect::new(rect).is_none() {
            return;
        }
        self.output.viewport_bounds.insert(node_id, rect);
        self.record_debug(node_id, DebugPrimitiveKind::ViewportBounds, rect);
    }

    pub(crate) fn record_scrollbar_placement(
        &mut self,
        node_id: NodeId,
        placement: crate::gui::layout_core::ScrollbarPlacement,
    ) {
        self.output.scrollbar_placements.insert(node_id, placement);
    }

    pub(crate) fn record_virtual_window_bounds(&mut self, node_id: NodeId, rect: Rect) {
        self.record_debug(node_id, DebugPrimitiveKind::VirtualWindowBounds, rect);
    }

    pub(crate) fn record_culled_region(&mut self, node_id: NodeId, rect: Rect) {
        self.record_debug(node_id, DebugPrimitiveKind::CulledRegion, rect);
    }

    pub(crate) fn record_virtual_window_info(&mut self, node_id: NodeId, info: VirtualWindowInfo) {
        self.output.virtual_windows.insert(node_id, info);
    }

    pub(crate) fn record_layout_visit(&mut self) {
        self.output.stats.laid_out_nodes += 1;
        self.output.stats.materialized_nodes += 1;
    }

    pub(crate) fn record_omitted_node(&mut self, node_id: NodeId) {
        self.output.record_omitted_node(node_id);
    }

    pub(crate) fn omit_subtree(&mut self, node: &LayoutNode) {
        self.output.record_omitted_node(node.id());
        if let LayoutNode::Container(container) = node {
            for child in &container.children {
                self.omit_subtree(&child.child);
            }
        }
    }

    pub(crate) fn record_measure_miss(&mut self) {
        self.output.stats.measured_nodes += 1;
    }

    pub(crate) fn push_diagnostic(
        &mut self,
        node_id: NodeId,
        code: LayoutDiagnosticCode,
        message: impl Into<Cow<'static, str>>,
    ) {
        let diagnostic = LayoutDiagnostic {
            node_id,
            code,
            message: message.into(),
        };
        if let Some(trace) = self.fragment_trace.as_mut() {
            trace.diagnostic(&diagnostic);
        }
        self.output.diagnostics.push(diagnostic);
    }

    fn record_debug(&mut self, node_id: NodeId, kind: DebugPrimitiveKind, rect: Rect) {
        if !self.debug_options.enabled {
            return;
        }
        if crate::gui::layout_core::validated_geometry::ValidatedRect::new(rect).is_none() {
            return;
        }
        if let Some(filter) = self.debug_node_filter
            && !filter.contains(&node_id)
        {
            return;
        }
        self.output.debug_primitives.push(LayoutDebugPrimitive {
            node_id,
            kind,
            rect,
        });
    }
}
