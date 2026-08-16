//! Static two-pane split placement.

use super::super::LayoutContext;
use super::layout_node;
use crate::gui::layout_core::engine::LayoutDiagnosticCode;
use crate::gui::layout_core::tree::ContainerNode;
use crate::gui::layout_core::{SplitPaneRuntimeState, sanitize_runtime_ratio};
use crate::gui::panel::{SplitPaneLayout, SplitPaneLayoutParts, quantized_split_pane_rects};
use crate::gui::types::Rect;

pub(super) fn layout_split_pane(
    container: &ContainerNode,
    content: Rect,
    context: &mut LayoutContext,
) {
    let state = context.container_state_read(container.id);
    if container.children.len() != 2 {
        context.push_diagnostic(
            container.id,
            LayoutDiagnosticCode::SplitPaneChildCountMismatch,
            "split pane requires exactly two children",
        );
        for child in &container.children {
            layout_node(&child.child, content, context);
        }
        return;
    }

    let policy = container.policy.split_pane;
    let declarative = SplitPaneLayout::from_parts(SplitPaneLayoutParts {
        bounds: content,
        axis: policy.axis,
        ratio: policy.initial_ratio,
        divider_extent: policy.divider_extent,
        first_min_extent: policy.first_min_extent,
        second_min_extent: policy.second_min_extent,
    });
    let ratio = container
        .split_pane_runtime
        .and_then(|mode| {
            let ownership = mode.ownership();
            state
                .as_ref()
                .and_then(|state| state.downcast_ref::<SplitPaneRuntimeState>())
                .filter(|state| state.ownership == ownership)
                .map(|state| sanitize_runtime_ratio(state.ratio, declarative.ratio))
        })
        .unwrap_or(declarative.ratio);
    let resolved = SplitPaneLayout::from_parts(SplitPaneLayoutParts {
        bounds: content,
        axis: policy.axis,
        ratio,
        divider_extent: policy.divider_extent,
        first_min_extent: policy.first_min_extent,
        second_min_extent: policy.second_min_extent,
    });
    if !resolved.minima_satisfied {
        context.push_diagnostic(
            container.id,
            LayoutDiagnosticCode::SplitPaneMinimumsUnsatisfied,
            "split pane minimums were unsatisfied by the available bounds",
        );
    }
    let (first, _divider, second) = quantized_split_pane_rects(resolved);
    layout_node(&container.children[0].child, first, context);
    layout_node(&container.children[1].child, second, context);
}
