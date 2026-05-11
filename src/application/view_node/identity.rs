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
                reserve_child_identity_capacity(children, ids);
                for child in children {
                    child.collect_reserved_ids(child_scope, ids);
                }
            }
            ViewNodeKind::Stack { children } => {
                reserve_child_identity_capacity(children, ids);
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

fn reserve_child_identity_capacity<Message>(children: &[ViewNode<Message>], ids: &mut Vec<NodeId>) {
    let mut reserved = 0;
    let mut nested_reserved = 0;
    for child in children {
        reserved += child.reserved_identity_capacity_hint();
        nested_reserved += usize::from(child.has_reserved_descendant_identity);
    }
    ids.reserve(reserved + nested_reserved);
}

impl<Message> ViewNode<Message> {
    fn reserved_identity_capacity_hint(&self) -> usize {
        if !self.has_reserved_identity {
            return 0;
        }
        match &self.kind {
            ViewNodeKind::Runtime(_) => 1 + usize::from(self.id.is_some() || self.key.is_some()),
            _ => 1,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        application::{ROOT_KEY_SCOPE, column, row, row_key, text},
        runtime::{SurfaceNode, WidgetMessageMapper},
        widgets::{ButtonWidget, WidgetSizing},
    };

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

    #[test]
    fn reserved_id_collection_presizes_for_nested_child_identities() {
        let view: ViewNode<()> = column(
            (0..64)
                .map(|index| row_key(format!("row-{index}"), [text("action").id(10_000 + index)])),
        );
        let mut ids = Vec::new();

        view.collect_reserved_ids(ROOT_KEY_SCOPE, &mut ids);

        assert_eq!(ids.len(), 128);
        assert!(ids.capacity() >= 128);
    }

    #[test]
    fn reserved_id_collection_presizes_wrapped_runtime_identities() {
        let runtime = SurfaceNode::widget(
            ButtonWidget::new(
                80,
                "Runtime",
                WidgetSizing::fixed(crate::layout::Vector2::new(80.0, 24.0)),
            ),
            WidgetMessageMapper::none(),
        );
        let view: ViewNode<()> = row([ViewNode::from(runtime).id(90)]);
        let mut ids = Vec::new();

        view.collect_reserved_ids(ROOT_KEY_SCOPE, &mut ids);

        assert_eq!(ids, vec![90, 80]);
        assert!(ids.capacity() >= 2);
    }
}
