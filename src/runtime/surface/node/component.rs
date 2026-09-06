//! Bounded comparison of two already Clone-qualified component declarations.

use super::SurfaceNode;
use crate::runtime::surface::revision::{InteractionLeafRevision, classify_interaction_leaf};
use crate::runtime::surface::source::source_metadata_matches;
use crate::runtime::surface::{application_container_kind_matches, application_node_kind};
use crate::runtime::{
    ExactChangedRoot, MAX_EXACT_CHANGED_ROOT_PATH_COMPONENTS, MAX_EXACT_CHANGED_ROOTS,
};

impl<Message> SurfaceNode<Message> {
    /// Produce equality evidence only. Runtime publication still validates each
    /// changed path under its request, generation and interaction authorities.
    pub(crate) fn compare_cached_component(
        &self,
        successor: &Self,
        node_limit: usize,
    ) -> (Option<Vec<ExactChangedRoot>>, usize) {
        let mut remaining = node_limit;
        let mut path = Vec::new();
        let mut changed = Vec::new();
        let admitted =
            visit(self, successor, &mut remaining, &mut path, &mut changed).map(|()| changed);
        (admitted, node_limit - remaining)
    }
}

fn visit<Message>(
    previous: &SurfaceNode<Message>,
    current: &SurfaceNode<Message>,
    remaining: &mut usize,
    path: &mut Vec<usize>,
    changed: &mut Vec<ExactChangedRoot>,
) -> Option<()> {
    // The comparison does not allocate a path for every unchanged child and
    // cannot recurse through an arbitrarily deep application declaration.
    *remaining = remaining.checked_sub(1)?;
    if path.len() > 128 || previous.id() != current.id() {
        return None;
    }
    let previous_source = previous.source_metadata_handle()?;
    let current_source = current.source_metadata_handle()?;
    if !source_metadata_matches(&previous_source, &current_source) {
        return None;
    }
    match (previous, current) {
        (SurfaceNode::Widget(previous), SurfaceNode::Widget(current)) => {
            match classify_interaction_leaf(previous, current) {
                InteractionLeafRevision::Reject => return None,
                InteractionLeafRevision::Interaction if !path.is_empty() => {
                    if changed.len() >= MAX_EXACT_CHANGED_ROOTS
                        || changed
                            .iter()
                            .map(|root| root.child_path.len())
                            .sum::<usize>()
                            + path.len()
                            > MAX_EXACT_CHANGED_ROOT_PATH_COMPONENTS
                    {
                        return None;
                    }
                    changed.push(ExactChangedRoot {
                        node_id: current.id(),
                        child_path: path.clone(),
                    });
                }
                // The enclosing lowering receipt compares the root itself.
                InteractionLeafRevision::Interaction | InteractionLeafRevision::Unchanged => {}
            }
        }
        (SurfaceNode::Container(previous_container), SurfaceNode::Container(current_container)) => {
            if !application_container_kind_matches(
                &application_node_kind(previous),
                &application_node_kind(current),
            ) {
                return None;
            }
            for (index, (previous, current)) in previous_container
                .children
                .iter()
                .zip(&current_container.children)
                .enumerate()
            {
                if previous.slot != current.slot {
                    return None;
                }
                path.push(index);
                visit(&previous.child, &current.child, remaining, path, changed)?;
                path.pop();
            }
        }
        _ => return None,
    }
    Some(())
}
