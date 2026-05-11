use super::{ViewNode, ViewNodeKind};
use crate::application::scoped_key_id;
use crate::layout::NodeId;

impl<Message> ViewNode<Message> {
    pub(super) fn collect_reserved_ids(&self, scope: u64, ids: &mut Vec<NodeId>) {
        if !self.has_reserved_identity_in_subtree() {
            return;
        }
        if self.has_reserved_identity {
            match &self.kind {
                ViewNodeKind::Runtime(node) => {
                    if let Some(id) = self.resolved_id(scope) {
                        ids.push(id);
                    }
                    ids.push(node.id());
                }
                _ => {
                    if let Some(id) = self.resolved_id(scope) {
                        ids.push(id);
                    }
                }
            }
        }
        if !self.has_reserved_descendant_identity {
            return;
        }
        let child_scope = self.child_scope(scope);
        match &self.kind {
            ViewNodeKind::Row { children, .. } | ViewNodeKind::Column { children, .. } => {
                ids.reserve(children.len());
                for child in children {
                    child.collect_reserved_ids(child_scope, ids);
                }
            }
            ViewNodeKind::Stack { children } => {
                ids.reserve(children.len());
                for child in children {
                    child.collect_reserved_ids(child_scope, ids);
                }
            }
            ViewNodeKind::Scroll { child } | ViewNodeKind::VirtualScroll { child, .. } => {
                child.collect_reserved_ids(child_scope, ids)
            }
            _ => {}
        }
    }

    pub(super) fn resolved_id(&self, scope: u64) -> Option<NodeId> {
        self.id
            .or_else(|| self.key.as_ref().map(|key| scoped_key_id(scope, key)))
    }

    fn child_scope(&self, parent_scope: u64) -> u64 {
        self.resolved_id(parent_scope).unwrap_or(parent_scope)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::{ROOT_KEY_SCOPE, column, row_key, text};

    #[test]
    fn reserved_id_collection_presizes_for_large_child_groups() {
        let view = column((0..64).map(|index| {
            row_key(
                format!("row-{index}"),
                Vec::<crate::application::ViewNode<()>>::new(),
            )
        }));
        let mut ids = Vec::new();

        view.collect_reserved_ids(ROOT_KEY_SCOPE, &mut ids);

        assert_eq!(ids.len(), 64);
        assert!(ids.capacity() >= 64);
    }

    #[test]
    fn reserved_id_collection_skips_unreserved_descendants() {
        let view: ViewNode<()> = row_key("row", [text("unreserved child")]);
        let mut ids = Vec::new();

        view.collect_reserved_ids(ROOT_KEY_SCOPE, &mut ids);

        assert_eq!(ids.len(), 1);
        assert!(ids.contains(&view.resolved_id(ROOT_KEY_SCOPE).unwrap()));
    }
}
