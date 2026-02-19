//! ScrollView layout including optional linear virtualization.

use super::super::measure::measure_node;
use super::super::{
    LayoutContext, LayoutDiagnosticCode, LinearVirtualMetrics, LinearVirtualWindow,
    VirtualWindowInfo, VirtualizationCacheKey,
};
use super::layout_node;
use super::scroll_helpers::{
    clamp_scroll_offset, compute_virtual_window, cursor_before_first, record_window_debug,
    sanitize_overscan,
};
use crate::gui::layout_core::constraints::Constraints;
use crate::gui::layout_core::model::{ContainerKind, SizeModeMain, VirtualizationAxis};
use crate::gui::layout_core::tree::{ContainerNode, LayoutNode, SlotChild};
use crate::gui::types::{Point, Rect, Vector2};

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
    let measured = measure_node(&child.child, slot.constraints, context);
    let viewport_w = (content.width() - slot.margin.left - slot.margin.right).max(0.0);
    let viewport_h = (content.height() - slot.margin.top - slot.margin.bottom).max(0.0);
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
    if !matches!(
        content_container.policy.align_main,
        crate::gui::layout_core::model::MainAlign::Start
    ) {
        context.push_diagnostic(
            container.id,
            LayoutDiagnosticCode::VirtualizationPolicyIgnored,
            "virtualization requires Start main-axis alignment",
        );
        return false;
    }

    if !virtualization_slots_supported(content_container.children.as_slice()) {
        context.push_diagnostic(
            container.id,
            LayoutDiagnosticCode::VirtualizationPolicyIgnored,
            "virtualization supports only Fixed/Intrinsic main-axis slots",
        );
        return false;
    }

    let viewport_main_size = if horizontal {
        viewport_rect.width()
    } else {
        viewport_rect.height()
    };
    let viewport_cross_size = if horizontal {
        viewport_rect.height()
    } else {
        viewport_rect.width()
    };
    let viewport_main_start = if horizontal { offset.x } else { offset.y };
    let list_constraints = Constraints::new(0.0, viewport_main_size, 0.0, viewport_cross_size);
    let metrics =
        cached_or_build_metrics(content_container, list_constraints, policy.axis, context);
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

    let first_before_margin =
        first_before_margin(content_container.children.as_slice(), first, horizontal);
    let cursor_main_start = cursor_before_first(first_before_margin, first, &metrics);
    context.set_linear_window(
        child.child.id(),
        LinearVirtualWindow {
            first,
            last_exclusive,
            cursor_main_start,
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
        },
    );
    true
}

fn cached_or_build_metrics(
    content: &ContainerNode,
    constraints: Constraints,
    axis: VirtualizationAxis,
    context: &mut LayoutContext,
) -> LinearVirtualMetrics {
    let key = VirtualizationCacheKey::new(content.id, constraints, axis, content.children.len());
    if !content
        .children
        .iter()
        .any(|entry| context.is_measure_dirty(entry.child.id()))
    {
        if let Some(metrics) = context.cached_virtual_metrics(key) {
            return metrics;
        }
    }
    let metrics = build_linear_metrics(
        content,
        constraints,
        matches!(axis, VirtualizationAxis::Horizontal),
        context,
    );
    context.remember_virtual_metrics(key, metrics.clone());
    metrics
}

fn build_linear_metrics(
    content: &ContainerNode,
    constraints: Constraints,
    horizontal: bool,
    context: &mut LayoutContext,
) -> LinearVirtualMetrics {
    let mut spans = Vec::with_capacity(content.children.len());
    let mut cursor = 0.0;
    let spacing = content.policy.spacing.max(0.0);
    let main_available = if horizontal {
        constraints.max_w
    } else {
        constraints.max_h
    };

    for (index, child) in content.children.iter().enumerate() {
        let measured = measure_node(&child.child, child.slot.constraints, context);
        let main =
            resolve_main_for_virtual(horizontal, child, measured, main_available, context).max(0.0);
        let before = if horizontal {
            child.slot.margin.left
        } else {
            child.slot.margin.top
        };
        let after = if horizontal {
            child.slot.margin.right
        } else {
            child.slot.margin.bottom
        };
        cursor += before;
        let start = cursor;
        let end = start + main;
        spans.push(super::super::VirtualSpan { start, end });
        cursor = end + after;
        if index + 1 < content.children.len() {
            cursor += spacing;
        }
    }

    LinearVirtualMetrics {
        spans,
        total_main: cursor.max(0.0),
    }
}

fn resolve_main_for_virtual(
    horizontal: bool,
    slot_child: &SlotChild,
    measured: Vector2,
    available_main: f32,
    context: &mut LayoutContext,
) -> f32 {
    let raw = match slot_child.slot.size_main {
        SizeModeMain::Fixed(value) => value,
        SizeModeMain::Intrinsic => {
            if horizontal {
                measured.x
            } else {
                measured.y
            }
        }
        SizeModeMain::Percent(percent) => available_main * percent.clamp(0.0, 1.0),
        SizeModeMain::Fill(_) => 0.0,
    };
    context.clamp_main(
        slot_child.child.id(),
        horizontal,
        slot_child.slot.constraints,
        raw,
    )
}

fn virtualization_slots_supported(children: &[SlotChild]) -> bool {
    children.iter().all(|child| {
        matches!(
            child.slot.size_main,
            SizeModeMain::Fixed(_) | SizeModeMain::Intrinsic
        )
    })
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
