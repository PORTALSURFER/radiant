//! Static two-pane split placement.

use super::super::LayoutContext;
use super::layout_node;
use crate::gui::layout_core::engine::LayoutDiagnosticCode;
use crate::gui::layout_core::tree::ContainerNode;
use crate::gui::panel::{SplitPaneLayout, SplitPaneLayoutParts};
use crate::gui::types::Rect;

pub(super) fn layout_split_pane(
    container: &ContainerNode,
    content: Rect,
    context: &mut LayoutContext,
) {
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
    let resolved = SplitPaneLayout::from_parts(SplitPaneLayoutParts {
        bounds: content,
        axis: policy.axis,
        ratio: policy.initial_ratio,
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
    layout_node(&container.children[0].child, resolved.first, context);
    layout_node(&container.children[1].child, resolved.second, context);
}
