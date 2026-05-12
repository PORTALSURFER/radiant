//! ScrollView layout including optional linear virtualization.

use super::super::cache::{
    LinearVirtualMetrics, ResolvedLinearWindow, VirtualizationCacheKey,
    virtualization_policy_fingerprint,
};
use super::super::helpers::LayoutAxis;
use super::super::{LayoutContext, LayoutDiagnosticCode, VirtualWindowInfo};
use super::layout_node;
use super::scroll_cache::collect_virtual_metric_dependencies;
use super::scroll_helpers::{
    clamp_scroll_offset, compute_virtual_window, cursor_before_first, record_window_debug,
    sanitize_overscan,
};
use super::scroll_linear::{build_linear_metrics, known_linear_main_extent, metrics_is_valid};
use crate::gui::layout_core::constraints::Constraints;
use crate::gui::layout_core::model::{ContainerKind, VirtualizationAxis};
use crate::gui::layout_core::tree::{ContainerNode, LayoutNode, SlotChild};
use crate::gui::types::{Point, Rect, Vector2};
use std::sync::Arc;

/// Layout a scroll container and optionally virtualize large linear child lists.
pub(super) fn layout_scroll_view(
    container: &ContainerNode,
    content: Rect,
    context: &mut LayoutContext,
) {
    let Some(child) = container.children.first() else {
        return;
    };
    let slot = child.slot;
    let viewport_w = (content.width() - slot.margin.left - slot.margin.right).max(0.0);
    let viewport_h = (content.height() - slot.margin.top - slot.margin.bottom).max(0.0);
    let measured = virtual_fixed_content_size(container, &child.child, viewport_w, viewport_h)
        .unwrap_or_else(|| {
            super::super::measure::measure_node(&child.child, slot.constraints, context)
        });
    let width = measured.x.max(viewport_w);
    let height = measured.y.max(viewport_h);
    let max_x = (width - viewport_w).max(0.0);
    let max_y = (height - viewport_h).max(0.0);

    let requested = context.scroll_offset(container.id);
    let (clamped, clamped_offset) = clamp_scroll_offset(requested, max_x, max_y);
    if clamped {
        context.push_diagnostic(
            container.id,
            LayoutDiagnosticCode::InvalidScrollOffsetClamped,
            "scroll offset was out of bounds and was clamped",
        );
    }

    let origin = Point::new(
        content.min.x + slot.margin.left - clamped_offset.x,
        content.min.y + slot.margin.top - clamped_offset.y,
    );
    let rect = Rect::from_min_size(origin, Vector2::new(width, height));
    context.record_slot_margin(child.child.id(), rect, slot.margin);

    let viewport_rect = Rect::from_min_size(
        Point::new(
            content.min.x + slot.margin.left,
            content.min.y + slot.margin.top,
        ),
        Vector2::new(viewport_w, viewport_h),
    );
    context.record_viewport_bounds(container.id, viewport_rect);

    if !layout_virtualized_child(
        container,
        child,
        rect,
        viewport_rect,
        clamped_offset,
        context,
    ) {
        layout_node(&child.child, rect, context);
    }

    if width > viewport_w || height > viewport_h {
        context.record_overflow(
            container.id,
            container.policy.overflow,
            width > viewport_w,
            height > viewport_h,
        );
    }
}

fn virtual_fixed_content_size(
    container: &ContainerNode,
    child: &LayoutNode,
    viewport_w: f32,
    viewport_h: f32,
) -> Option<Vector2> {
    let policy = container.policy.virtualization?;
    if !policy.enabled {
        return None;
    }
    let LayoutNode::Container(content) = child else {
        return None;
    };
    let horizontal = match (content.policy.kind, policy.axis) {
        (ContainerKind::Row, VirtualizationAxis::Horizontal) => true,
        (ContainerKind::Column, VirtualizationAxis::Vertical) => false,
        _ => return None,
    };
    let main = known_linear_main_extent(content, policy.axis)?;
    Some(if horizontal {
        Vector2::new(main, viewport_h)
    } else {
        Vector2::new(viewport_w, main)
    })
}

fn layout_virtualized_child(
    container: &ContainerNode,
    child: &SlotChild,
    child_rect: Rect,
    viewport_rect: Rect,
    offset: Vector2,
    context: &mut LayoutContext,
) -> bool {
    let Some(policy) = container.policy.virtualization else {
        return false;
    };
    if !policy.enabled {
        return false;
    }

    let LayoutNode::Container(content_container) = &child.child else {
        context.push_diagnostic(
            container.id,
            LayoutDiagnosticCode::VirtualizationPolicyIgnored,
            "virtualization requires a container child",
        );
        return false;
    };

    let horizontal = match (content_container.policy.kind, policy.axis) {
        (ContainerKind::Row, VirtualizationAxis::Horizontal) => true,
        (ContainerKind::Column, VirtualizationAxis::Vertical) => false,
        _ => {
            context.push_diagnostic(
                container.id,
                LayoutDiagnosticCode::VirtualizationPolicyIgnored,
                "virtualization supports Row/Horizontal and Column/Vertical only",
            );
            return false;
        }
    };

    let axis = LayoutAxis::from_horizontal(horizontal);
    let available_main = axis.main_extent(child_rect).max(0.0);
    let available_cross = axis.cross_extent(child_rect).max(0.0);

    let viewport_main_size = axis.main_extent(viewport_rect);
    let viewport_main_start = if horizontal { offset.x } else { offset.y };

    let constraints = if horizontal {
        Constraints::new(0.0, available_main, 0.0, available_cross)
    } else {
        Constraints::new(0.0, available_cross, 0.0, available_main)
    };
    let metrics = cached_or_build_metrics(content_container, constraints, policy.axis, context);
    if !metrics_is_valid(&metrics, content_container.children.len()) {
        context.push_diagnostic(
            container.id,
            LayoutDiagnosticCode::VirtualizationSpanResolutionFallback,
            "virtualization spans were invalid and full layout fallback was used",
        );
        return false;
    }

    let (overscan_px, overscan_clamped) = sanitize_overscan(policy.overscan_px);
    if overscan_clamped {
        context.push_diagnostic(
            container.id,
            LayoutDiagnosticCode::VirtualizationWindowClamped,
            "virtualization overscan was non-finite or negative and was clamped",
        );
    }
    let (window_start, window_end, first, last_exclusive, bounds_clamped) = compute_virtual_window(
        &metrics,
        viewport_main_start,
        viewport_main_size,
        overscan_px,
    );
    if bounds_clamped {
        context.push_diagnostic(
            container.id,
            LayoutDiagnosticCode::VirtualizationWindowClamped,
            "virtualization window bounds were clamped",
        );
    }

    if first >= last_exclusive {
        context.push_diagnostic(
            container.id,
            LayoutDiagnosticCode::VirtualizationAlignmentFallback,
            "virtualization window was empty after alignment resolution",
        );
        return false;
    }

    let first_before_margin =
        first_before_margin(content_container.children.as_slice(), first, horizontal);
    let cursor_main_start = cursor_before_first(first_before_margin, first, &metrics);
    context.set_linear_window(
        child.child.id(),
        ResolvedLinearWindow {
            first,
            last_exclusive,
            cursor_main_start,
            metrics: Arc::clone(&metrics),
        },
    );
    layout_node(&child.child, child_rect, context);
    context.clear_linear_window(child.child.id());

    record_window_debug(
        container.id,
        child_rect,
        horizontal,
        window_start,
        window_end,
        context,
    );
    context.record_virtual_window_info(
        container.id,
        VirtualWindowInfo {
            total_children: content_container.children.len(),
            first_index: first,
            last_index_exclusive: last_exclusive,
            culled_before: first,
            culled_after: content_container
                .children
                .len()
                .saturating_sub(last_exclusive),
            viewport_main_start,
            viewport_main_end: viewport_main_start + viewport_main_size,
            window_main_start: window_start,
            window_main_end: window_end,
            resolved_total_main: metrics.total_main,
            alignment_mode: content_container.policy.align_main,
        },
    );
    true
}

fn cached_or_build_metrics(
    content: &ContainerNode,
    constraints: Constraints,
    axis: VirtualizationAxis,
    context: &mut LayoutContext,
) -> Arc<LinearVirtualMetrics> {
    let key = VirtualizationCacheKey::new(
        content.id,
        constraints,
        axis,
        content.children.len(),
        virtualization_policy_fingerprint(content),
    );
    if let Some(metrics) = context.cached_virtual_metrics(key) {
        return metrics;
    }

    let metrics = Arc::new(build_linear_metrics(content, constraints, axis, context));
    let mut dependencies = Vec::with_capacity(content.children.len().saturating_add(1));
    collect_virtual_metric_dependencies(content, &mut dependencies);
    context.remember_virtual_metrics(key, Arc::clone(&metrics), dependencies);
    metrics
}

fn first_before_margin(children: &[SlotChild], first: usize, horizontal: bool) -> f32 {
    if first >= children.len() {
        return 0.0;
    }
    if horizontal {
        children[first].slot.margin.left
    } else {
        children[first].slot.margin.top
    }
}
