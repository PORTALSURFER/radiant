//! Linear virtualization path for scroll-view layout.

use super::super::super::cache::{
    LinearVirtualMetrics, ResolvedLinearWindow, VirtualizationCacheKey,
    virtualization_policy_fingerprint,
};
use super::super::super::helpers::LayoutAxis;
use super::super::super::{LayoutContext, LayoutDiagnosticCode, VirtualWindowInfo};
use super::super::layout_node;
use super::super::scroll_cache::collect_virtual_metric_dependencies;
use super::super::scroll_helpers::{
    compute_virtual_window, cursor_before_first, record_window_debug, sanitize_overscan,
};
use super::super::scroll_linear::{build_linear_metrics, metrics_is_valid};
use crate::gui::layout_core::constraints::Constraints;
use crate::gui::layout_core::model::{ContainerKind, VirtualizationAxis};
use crate::gui::layout_core::tree::{ContainerNode, LayoutNode, SlotChild};
use crate::gui::types::{Rect, Vector2};
use std::sync::Arc;

pub(super) fn has_invalid_content_margin(
    container: &ContainerNode,
    child: &LayoutNode,
    available: Rect,
) -> bool {
    let Some(policy) = container.policy.virtualization else {
        return false;
    };
    if !policy.enabled {
        return false;
    }
    let LayoutNode::Container(content_container) = child else {
        return false;
    };
    match (content_container.policy.kind, policy.axis) {
        (ContainerKind::Row, VirtualizationAxis::Horizontal)
        | (ContainerKind::Column, VirtualizationAxis::Vertical) => {
            content_container.children.iter().any(|child| {
                crate::gui::layout_core::validated_geometry::checked_margin_geometry(
                    available,
                    child.slot.margin,
                )
                .is_none()
            })
        }
        _ => false,
    }
}

pub(super) fn layout_virtualized_child(
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
    let physical_viewport_main_start = if horizontal { offset.x } else { offset.y };

    let constraints = if horizontal {
        Constraints::new(0.0, available_main, 0.0, available_cross)
    } else {
        Constraints::new(0.0, available_cross, 0.0, available_main)
    };
    let Some(metrics) =
        cached_or_build_metrics(content_container, constraints, policy.axis, context)
    else {
        context.push_diagnostic(
            container.id,
            LayoutDiagnosticCode::VirtualizationSpanResolutionFallback,
            "virtualization spans were invalid and full layout fallback was used",
        );
        return false;
    };

    let (overscan_px, overscan_clamped) = sanitize_overscan(policy.overscan_px);
    if overscan_clamped {
        context.push_diagnostic(
            container.id,
            LayoutDiagnosticCode::VirtualizationWindowClamped,
            "virtualization overscan was non-finite or negative and was clamped",
        );
    }
    let rtl = horizontal && context.direction() == crate::gui::layout_core::WritingDirection::Rtl;
    let logical_viewport_main_start = if rtl {
        // Scroll offsets remain physical. Convert the visible physical slice
        // to the logical span used by the source-order metrics.
        (metrics.total_main - physical_viewport_main_start - viewport_main_size).max(0.0)
    } else {
        physical_viewport_main_start
    };
    let window = compute_virtual_window(
        &metrics,
        logical_viewport_main_start,
        viewport_main_size,
        overscan_px,
    );
    if window.clamped {
        context.push_diagnostic(
            container.id,
            LayoutDiagnosticCode::VirtualizationWindowClamped,
            "virtualization window bounds were clamped",
        );
    }

    if window.first >= window.last_exclusive {
        context.push_diagnostic(
            container.id,
            LayoutDiagnosticCode::VirtualizationAlignmentFallback,
            "virtualization window was empty after alignment resolution",
        );
        return false;
    }

    let first_before_margin = first_before_margin(
        content_container.children.as_slice(),
        window.first,
        horizontal,
        rtl,
    );
    let cursor_main_start = cursor_before_first(first_before_margin, window.first, &metrics);
    context.set_linear_window(
        child.child.id(),
        ResolvedLinearWindow {
            first: window.first,
            last_exclusive: window.last_exclusive,
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
        window.start,
        window.end,
        metrics.total_main,
        context.direction(),
        context,
    );
    context.record_virtual_window_info(
        container.id,
        VirtualWindowInfo {
            total_children: content_container.children.len(),
            first_index: window.first,
            last_index_exclusive: window.last_exclusive,
            culled_before: window.first,
            culled_after: content_container
                .children
                .len()
                .saturating_sub(window.last_exclusive),
            viewport_main_start: physical_viewport_main_start,
            viewport_main_end: physical_viewport_main_start + viewport_main_size,
            window_main_start: window.start,
            window_main_end: window.end,
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
) -> Option<Arc<LinearVirtualMetrics>> {
    let expected_len = content.children.len();
    let key = VirtualizationCacheKey::new(
        content.id,
        constraints,
        axis,
        expected_len,
        virtualization_policy_fingerprint(content),
        context.direction(),
    );
    if let Some(metrics) = context.cached_virtual_metrics(key) {
        if metrics_is_valid(&metrics, expected_len) {
            return Some(metrics);
        }
        context.discard_virtual_metrics(key);
    }

    let metrics = Arc::new(build_linear_metrics(content, constraints, axis, context));
    if !metrics_is_valid(&metrics, expected_len) {
        return None;
    }
    let mut dependencies = Vec::with_capacity(expected_len.saturating_add(1));
    collect_virtual_metric_dependencies(content, &mut dependencies);
    context.remember_virtual_metrics(key, Arc::clone(&metrics), dependencies);
    Some(metrics)
}

fn first_before_margin(children: &[SlotChild], first: usize, horizontal: bool, rtl: bool) -> f32 {
    if first >= children.len() {
        return 0.0;
    }
    if horizontal {
        if rtl {
            children[first].slot.margin.right
        } else {
            children[first].slot.margin.left
        }
    } else {
        children[first].slot.margin.top
    }
}
