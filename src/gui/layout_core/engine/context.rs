//! Shared mutable context for one measure/layout evaluation.

use super::{
    CachedVirtualMetrics, DebugPrimitiveKind, LayoutDebugOptions, LayoutDebugPrimitive,
    LayoutDiagnostic, LayoutDiagnosticCode, LayoutOutput, LayoutState, LinearVirtualMetrics,
    MeasureCacheKey, OverflowInfo, ResolvedLinearWindow, VirtualWindowInfo, VirtualizationCacheKey,
};
use crate::gui::layout_core::constraints::Constraints;
use crate::gui::layout_core::model::{Insets, OverflowPolicy};
use crate::gui::layout_core::tree::NodeId;
use crate::gui::types::{Point, Rect, Vector2};
use std::collections::{BTreeSet, HashMap};

pub(super) struct LayoutContext<'a> {
    measured: HashMap<MeasureCacheKey, Vector2>,
    measured_by_node: HashMap<NodeId, Vector2>,
    cache: &'a mut HashMap<MeasureCacheKey, Vector2>,
    virtual_cache: &'a mut HashMap<VirtualizationCacheKey, CachedVirtualMetrics>,
    linear_windows: HashMap<NodeId, ResolvedLinearWindow>,
    measure_dirty: &'a BTreeSet<NodeId>,
    state: &'a LayoutState,
    debug_options: LayoutDebugOptions,
    debug_node_filter: Option<&'a BTreeSet<NodeId>>,
    pub(super) output: LayoutOutput,
}

impl<'a> LayoutContext<'a> {
    pub(super) fn new(
        cache: &'a mut HashMap<MeasureCacheKey, Vector2>,
        virtual_cache: &'a mut HashMap<VirtualizationCacheKey, CachedVirtualMetrics>,
        measure_dirty: &'a BTreeSet<NodeId>,
        state: &'a LayoutState,
        debug_options: LayoutDebugOptions,
        debug_node_filter: Option<&'a BTreeSet<NodeId>>,
    ) -> Self {
        Self {
            measured: HashMap::new(),
            measured_by_node: HashMap::new(),
            cache,
            virtual_cache,
            linear_windows: HashMap::new(),
            measure_dirty,
            state,
            debug_options,
            debug_node_filter,
            output: LayoutOutput::default(),
        }
    }

    pub(super) fn cached_measure(
        &self,
        key: MeasureCacheKey,
        node_id: NodeId,
        is_container: bool,
    ) -> Option<Vector2> {
        if self.measure_dirty.contains(&node_id) {
            return None;
        }
        if is_container && !self.measure_dirty.is_empty() {
            return None;
        }
        self.measured
            .get(&key)
            .copied()
            .or_else(|| self.cache.get(&key).copied())
    }

    pub(super) fn remember_measure(&mut self, key: MeasureCacheKey, value: Vector2) {
        self.measured.insert(key, value);
        self.cache.insert(key, value);
        self.measured_by_node.insert(key.node_id, value);
    }

    pub(super) fn cached_virtual_metrics(
        &self,
        key: VirtualizationCacheKey,
    ) -> Option<LinearVirtualMetrics> {
        self.virtual_cache
            .get(&key)
            .map(|entry| entry.metrics.clone())
    }

    pub(super) fn remember_virtual_metrics(
        &mut self,
        key: VirtualizationCacheKey,
        metrics: LinearVirtualMetrics,
        dependencies: BTreeSet<NodeId>,
    ) {
        self.virtual_cache.insert(
            key,
            CachedVirtualMetrics {
                metrics,
                dependencies,
            },
        );
    }

    pub(super) fn record_measured_size(&mut self, node_id: NodeId, value: Vector2) {
        self.measured_by_node.insert(node_id, value);
    }

    pub(super) fn set_linear_window(&mut self, node_id: NodeId, window: ResolvedLinearWindow) {
        self.linear_windows.insert(node_id, window);
    }

    pub(super) fn clear_linear_window(&mut self, node_id: NodeId) {
        self.linear_windows.remove(&node_id);
    }

    pub(super) fn linear_window(&self, node_id: NodeId) -> Option<ResolvedLinearWindow> {
        self.linear_windows.get(&node_id).cloned()
    }

    pub(super) fn normalize_constraints(
        &mut self,
        node_id: NodeId,
        constraints: Constraints,
    ) -> Constraints {
        let mut min_w = constraints.min_w;
        let mut max_w = constraints.max_w;
        let mut min_h = constraints.min_h;
        let mut max_h = constraints.max_h;

        if !min_w.is_finite() {
            min_w = 0.0;
            self.push_diagnostic(
                node_id,
                LayoutDiagnosticCode::NegativeSizeClamped,
                "min width was non-finite and was clamped",
            );
        }
        if !max_w.is_finite() {
            max_w = f32::INFINITY;
            self.push_diagnostic(
                node_id,
                LayoutDiagnosticCode::NegativeSizeClamped,
                "max width was non-finite and was clamped",
            );
        }
        if !min_h.is_finite() {
            min_h = 0.0;
            self.push_diagnostic(
                node_id,
                LayoutDiagnosticCode::NegativeSizeClamped,
                "min height was non-finite and was clamped",
            );
        }
        if !max_h.is_finite() {
            max_h = f32::INFINITY;
            self.push_diagnostic(
                node_id,
                LayoutDiagnosticCode::NegativeSizeClamped,
                "max height was non-finite and was clamped",
            );
        }

        if min_w < 0.0 {
            min_w = 0.0;
            self.push_diagnostic(
                node_id,
                LayoutDiagnosticCode::NegativeSizeClamped,
                "negative minimum width was clamped",
            );
        }
        if min_h < 0.0 {
            min_h = 0.0;
            self.push_diagnostic(
                node_id,
                LayoutDiagnosticCode::NegativeSizeClamped,
                "negative minimum height was clamped",
            );
        }
        if max_w < min_w {
            max_w = min_w;
            self.push_diagnostic(
                node_id,
                LayoutDiagnosticCode::ConstraintContradiction,
                "width constraints were contradictory and were normalized",
            );
        }
        if max_h < min_h {
            max_h = min_h;
            self.push_diagnostic(
                node_id,
                LayoutDiagnosticCode::ConstraintContradiction,
                "height constraints were contradictory and were normalized",
            );
        }

        Constraints {
            min_w,
            max_w,
            min_h,
            max_h,
        }
    }

    pub(super) fn clamp_width(
        &mut self,
        node_id: NodeId,
        constraints: Constraints,
        value: f32,
    ) -> f32 {
        self.clamp_axis(node_id, constraints, value, true)
    }

    pub(super) fn clamp_height(
        &mut self,
        node_id: NodeId,
        constraints: Constraints,
        value: f32,
    ) -> f32 {
        self.clamp_axis(node_id, constraints, value, false)
    }

    pub(super) fn clamp_main(
        &mut self,
        node_id: NodeId,
        horizontal: bool,
        constraints: Constraints,
        value: f32,
    ) -> f32 {
        if horizontal {
            self.clamp_width(node_id, constraints, value)
        } else {
            self.clamp_height(node_id, constraints, value)
        }
    }

    pub(super) fn clamp_cross(
        &mut self,
        node_id: NodeId,
        horizontal: bool,
        constraints: Constraints,
        value: f32,
    ) -> f32 {
        if horizontal {
            self.clamp_height(node_id, constraints, value)
        } else {
            self.clamp_width(node_id, constraints, value)
        }
    }

    pub(super) fn scroll_offset(&self, node_id: NodeId) -> Vector2 {
        self.state.scroll_offset(node_id)
    }

    pub(super) fn record_overflow(
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
        if self.debug_options.show_overflow {
            if let Some(rect) = self.output.rects.get(&node_id).copied() {
                self.record_debug(node_id, DebugPrimitiveKind::OverflowMarker, rect);
            }
        }
    }

    pub(super) fn record_node_bounds(&mut self, node_id: NodeId, rect: Rect) {
        if self.debug_options.show_bounds {
            self.record_debug(node_id, DebugPrimitiveKind::NodeBounds, rect);
        }
        if self.debug_options.show_measured {
            if let Some(measured) = self.measured_by_node.get(&node_id).copied() {
                let measured_rect = Rect::from_min_size(
                    rect.min,
                    Vector2::new(
                        measured.x.max(0.0).min(rect.width().max(0.0)),
                        measured.y.max(0.0).min(rect.height().max(0.0)),
                    ),
                );
                self.record_debug(node_id, DebugPrimitiveKind::MeasuredBounds, measured_rect);
            }
        }
    }

    pub(super) fn record_content_bounds(&mut self, node_id: NodeId, rect: Rect) {
        if self.debug_options.show_padding {
            self.record_debug(node_id, DebugPrimitiveKind::ContentBounds, rect);
        }
    }

    pub(super) fn record_slot_margin(&mut self, node_id: NodeId, child_rect: Rect, margin: Insets) {
        if !self.debug_options.show_margins {
            return;
        }
        let margin_rect = Rect::from_min_max(
            Point::new(
                child_rect.min.x - margin.left,
                child_rect.min.y - margin.top,
            ),
            Point::new(
                child_rect.max.x + margin.right,
                child_rect.max.y + margin.bottom,
            ),
        );
        self.record_debug(node_id, DebugPrimitiveKind::SlotMargin, margin_rect);
    }

    pub(super) fn record_viewport_bounds(&mut self, node_id: NodeId, rect: Rect) {
        self.record_debug(node_id, DebugPrimitiveKind::ViewportBounds, rect);
    }

    pub(super) fn record_virtual_window_bounds(&mut self, node_id: NodeId, rect: Rect) {
        self.record_debug(node_id, DebugPrimitiveKind::VirtualWindowBounds, rect);
    }

    pub(super) fn record_culled_region(&mut self, node_id: NodeId, rect: Rect) {
        self.record_debug(node_id, DebugPrimitiveKind::CulledRegion, rect);
    }

    pub(super) fn record_virtual_window_info(&mut self, node_id: NodeId, info: VirtualWindowInfo) {
        self.output.virtual_windows.insert(node_id, info);
    }

    pub(super) fn record_layout_visit(&mut self) {
        self.output.stats.laid_out_nodes += 1;
        self.output.stats.materialized_nodes += 1;
    }

    pub(super) fn record_measure_miss(&mut self) {
        self.output.stats.measured_nodes += 1;
    }

    pub(super) fn push_diagnostic(
        &mut self,
        node_id: NodeId,
        code: LayoutDiagnosticCode,
        message: impl Into<String>,
    ) {
        self.output.diagnostics.push(LayoutDiagnostic {
            node_id,
            code,
            message: message.into(),
        });
    }

    fn clamp_axis(
        &mut self,
        node_id: NodeId,
        constraints: Constraints,
        value: f32,
        is_width: bool,
    ) -> f32 {
        let normalized = self.normalize_constraints(node_id, constraints);
        let sanitized = if !value.is_finite() {
            self.push_diagnostic(
                node_id,
                LayoutDiagnosticCode::NegativeSizeClamped,
                "non-finite size was clamped",
            );
            0.0
        } else if value < 0.0 {
            self.push_diagnostic(
                node_id,
                LayoutDiagnosticCode::NegativeSizeClamped,
                "negative size was clamped",
            );
            0.0
        } else {
            value
        };

        if is_width {
            normalized.clamp_w(sanitized)
        } else {
            normalized.clamp_h(sanitized)
        }
    }

    fn record_debug(&mut self, node_id: NodeId, kind: DebugPrimitiveKind, rect: Rect) {
        if !self.debug_options.enabled {
            return;
        }
        if let Some(filter) = self.debug_node_filter {
            if !filter.contains(&node_id) {
                return;
            }
        }
        self.output.debug_primitives.push(LayoutDebugPrimitive {
            node_id,
            kind,
            rect,
        });
    }
}
