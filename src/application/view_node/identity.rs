use super::{ViewNode, ViewNodeKind};
use crate::application::scoped_key_id;
use crate::layout::NodeId;
use std::fmt;

/// Owned application-facing identity for declarative view continuity.
///
/// A continuity key is scoped by its keyed or explicitly identified parent
/// during view lowering. Keeping the key as a distinct type prevents it from
/// being confused with numeric runtime ids or other application identities
/// while retaining ergonomic construction from ordinary strings.
#[derive(Clone, Debug, Default, Eq, Hash, PartialEq)]
pub struct ContinuityKey(String);

impl ContinuityKey {
    /// Construct a continuity key from owned or borrowed string data.
    pub fn new(key: impl Into<String>) -> Self {
        Self(key.into())
    }

    /// Borrow the stable key text.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ContinuityKey {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl From<&str> for ContinuityKey {
    fn from(key: &str) -> Self {
        Self::new(key)
    }
}

impl From<&String> for ContinuityKey {
    fn from(key: &String) -> Self {
        Self::new(key.clone())
    }
}

impl From<String> for ContinuityKey {
    fn from(key: String) -> Self {
        Self(key)
    }
}

#[cfg(test)]
#[path = "identity/tests.rs"]
mod tests;

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
            ViewNodeKind::Scene { base, layers, .. } => {
                base.collect_reserved_ids(child_scope, ids);
                for layer in layers {
                    if let Some(input) = layer.input.as_ref() {
                        input.collect_reserved_ids(child_scope, ids);
                    }
                    layer.view.collect_reserved_ids(child_scope, ids);
                }
            }
            ViewNodeKind::Container { children, .. } => {
                reserve_child_identity_capacity(children, ids);
                for child in children {
                    child.collect_reserved_ids(child_scope, ids);
                }
            }
            ViewNodeKind::Scroll { child } | ViewNodeKind::VirtualScroll { child, .. } => {
                child.collect_reserved_ids(child_scope, ids)
            }
            ViewNodeKind::FloatingLayer { child, .. } => {
                child.collect_reserved_ids(child_scope, ids)
            }
            _ => {}
        }
        for layer in &self.overlay_layers {
            if let Some(input) = layer.input.as_ref() {
                input.collect_reserved_ids(child_scope, ids);
            }
            layer.view.collect_reserved_ids(child_scope, ids);
        }
    }

    pub(super) fn resolved_id(&self, scope: u64) -> Option<NodeId> {
        self.id.or_else(|| {
            self.key
                .as_ref()
                .map(|key| scoped_key_id(scope, key.as_str()))
        })
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
