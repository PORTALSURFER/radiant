//! Bounded focus-scope membership collected with the committed traversal index.
use super::{SourceTraversalIndex, SurfaceNode};
use crate::{
    layout::{LayoutOutput, NodeId},
    runtime::FocusScope,
};
use std::collections::{HashMap, HashSet};

struct ScopeRecord {
    layout_id: NodeId,
    parent: Option<usize>,
    policy: FocusScope,
}
#[derive(Default)]
pub(in crate::runtime) struct SurfaceFocusScopes {
    records: Vec<ScopeRecord>,
    members: HashMap<NodeId, usize>,
    active: Option<usize>,
    invalid: bool,
}
impl SurfaceFocusScopes {
    pub(in crate::runtime) fn clear(&mut self) {
        self.records.clear();
        self.members.clear();
        self.active = None;
        self.invalid = false;
    }
    pub(in crate::runtime) fn enter<Message>(
        &mut self,
        node: &SurfaceNode<Message>,
    ) -> Option<usize> {
        let previous = self.active;
        if let Some(policy) = node.focus_scope() {
            if self.records.len() == 64 {
                self.invalid = true;
            } else {
                self.active = Some(self.records.len());
                self.records.push(ScopeRecord {
                    layout_id: layout_identity(node),
                    parent: previous,
                    policy,
                });
            }
        }
        if let Some(scope) = self.active
            && (self.members.len() == 65_536 || self.members.insert(node.id(), scope).is_some())
        {
            self.invalid = true;
        }
        previous
    }
    pub(in crate::runtime) fn leave(&mut self, previous: Option<usize>) {
        self.active = previous;
    }
    pub(in crate::runtime) fn qualify(&mut self, source: &SourceTraversalIndex) {
        if self.records.is_empty() {
            return;
        }
        if source.records.len() > 65_536 {
            self.invalid = true;
            return;
        }
        let mut seen = HashSet::new();
        self.invalid |= source
            .records
            .iter()
            .any(|record| !seen.insert(record.node_id));
    }
    pub(in crate::runtime) fn current(
        &self,
        node: Option<NodeId>,
        layout: &LayoutOutput,
    ) -> Result<Option<(usize, FocusScope)>, ()> {
        if self.invalid {
            return Err(());
        }
        let mut scope = node.and_then(|node| self.members.get(&node).copied());
        while let Some(index) = scope {
            let record = &self.records[index];
            if layout.rects.contains_key(&record.layout_id) {
                return Ok(Some((index, record.policy)));
            }
            scope = record.parent;
        }
        Ok(None)
    }
    pub(in crate::runtime) fn contains(&self, scope: usize, node: NodeId) -> bool {
        let mut candidate = self.members.get(&node).copied();
        while let Some(index) = candidate {
            if index == scope {
                return true;
            }
            candidate = self.records[index].parent;
        }
        false
    }
}
fn layout_identity<Message>(node: &SurfaceNode<Message>) -> NodeId {
    match node {
        SurfaceNode::Scene(scene) if !scene.has_layers() => layout_identity(&scene.base),
        _ => node.id(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::{IntoView, text};
    #[test]
    fn prepared_scope_metadata_survives_outer_lowering_and_explicit_override() {
        use crate::{application::ViewNode, runtime::FocusScopeBoundary};
        let prepared = text::<()>("scope")
            .id(1)
            .focus_scope(FocusScope::sequential())
            .into_node();
        let preserved = ViewNode::from(prepared.clone()).into_node();
        assert_eq!(
            preserved.source_metadata_handle().unwrap().focus_scope,
            Some(FocusScope::sequential())
        );
        let policy = FocusScope::spatial_grid().boundary(FocusScopeBoundary::Wrap);
        let replaced = ViewNode::from(prepared).focus_scope(policy).into_node();
        assert_eq!(
            replaced.source_metadata_handle().unwrap().focus_scope,
            Some(policy)
        );
    }

    #[test]
    fn omitted_scope_layout_and_duplicate_source_ids_do_not_gain_scope_authority() {
        let node = text::<()>("scope")
            .id(1)
            .focus_scope(FocusScope::sequential())
            .into_node();
        let mut scopes = SurfaceFocusScopes::default();
        let previous = scopes.enter(&node);
        scopes.leave(previous);
        let mut layout = LayoutOutput::default();
        assert!(matches!(scopes.current(Some(1), &layout), Ok(None)));
        layout.rects.insert(1, Default::default());
        assert!(matches!(scopes.current(Some(1), &layout), Ok(Some(_))));
        let mut source = SourceTraversalIndex::default();
        source.record_node(&node);
        source.record_node(&node);
        scopes.qualify(&source);
        assert_eq!(scopes.current(Some(1), &layout), Err(()));
        scopes.clear();
        assert_eq!(scopes.current(Some(1), &layout), Ok(None));
    }
}
