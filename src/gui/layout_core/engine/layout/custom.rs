//! Custom layout-policy placement.

use super::super::{LayoutContext, round_rect};
use crate::gui::layout_core::policy::LayoutPolicy;
use crate::gui::layout_core::policy::{ChildDisposition, PlaceChildren, PlaceChildrenError};
use crate::gui::layout_core::tree::ContainerNode;
use crate::gui::types::Rect;

pub(super) fn layout_custom(
    container: &ContainerNode,
    layout_policy: &dyn LayoutPolicy,
    bounds: Rect,
    context: &mut LayoutContext,
) {
    let mut dispositions = vec![None; container.children.len()];
    let mut errors = Vec::new();
    {
        let mut children =
            PlaceChildren::new(container.children.len(), &mut dispositions, &mut errors);
        layout_policy.place(&mut children, bounds);
    }

    for error in errors {
        let code = match error {
            PlaceChildrenError::InvalidIndex { .. } => {
                crate::gui::layout_core::engine::LayoutDiagnosticCode::CustomLayoutInvalidChildIndex
            }
            PlaceChildrenError::InvalidRect { .. } => {
                crate::gui::layout_core::engine::LayoutDiagnosticCode::CustomLayoutInvalidPlacement
            }
            PlaceChildrenError::DuplicateDisposition { .. } => {
                crate::gui::layout_core::engine::LayoutDiagnosticCode::CustomLayoutDuplicatePlacement
            }
        };
        let message = match error {
            PlaceChildrenError::InvalidIndex { .. } => {
                "custom layout policy requested an invalid child index"
            }
            PlaceChildrenError::InvalidRect { .. } => {
                "custom layout policy supplied an invalid child rectangle"
            }
            PlaceChildrenError::DuplicateDisposition { .. } => {
                "custom layout policy supplied a duplicate child disposition"
            }
        };
        context.push_diagnostic(container.id, code, message);
    }

    for (index, child) in container.children.iter().enumerate() {
        match dispositions[index] {
            Some(ChildDisposition::Placed(rect)) => {
                super::layout_node(&child.child, round_rect(rect), context);
            }
            Some(ChildDisposition::Omitted(_)) => {
                context.record_omitted_node(child.child.id());
            }
            None => {
                context.record_omitted_node(child.child.id());
                context.push_diagnostic(
                    container.id,
                    crate::gui::layout_core::engine::LayoutDiagnosticCode::CustomLayoutChildUnresolved,
                    "custom layout policy did not resolve a declared child",
                );
            }
        }
    }
}
