//! Shared child collection helpers for layout builders.

use crate::application::ViewNode;

/// Declarative child-list builder for containers with optional children.
///
/// Use this when a row, column, grid, stack, or other container has a small
/// number of named children and one or more optional branches. It keeps the
/// container call site readable without introducing an app-local temporary
/// vector or a layout-specific optional widget.
pub struct Children<Message> {
    children: Vec<ViewNode<Message>>,
}

impl<Message> Children<Message> {
    /// Build an empty child list.
    pub fn new() -> Self {
        Self {
            children: Vec::new(),
        }
    }

    /// Build an empty child list with reserved capacity.
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            children: Vec::with_capacity(capacity),
        }
    }

    /// Add one child.
    pub fn push(mut self, child: ViewNode<Message>) -> Self {
        self.children.push(child);
        self
    }

    /// Add one child when it exists.
    pub fn push_opt(mut self, child: Option<ViewNode<Message>>) -> Self {
        if let Some(child) = child {
            self.children.push(child);
        }
        self
    }

    /// Add one lazily constructed child when `condition` is true.
    pub fn push_if(mut self, condition: bool, child: impl FnOnce() -> ViewNode<Message>) -> Self {
        if condition {
            self.children.push(child());
        }
        self
    }

    /// Return the number of collected children.
    pub fn len(&self) -> usize {
        self.children.len()
    }

    /// Return whether no children have been collected.
    pub fn is_empty(&self) -> bool {
        self.children.is_empty()
    }
}

impl<Message> Default for Children<Message> {
    fn default() -> Self {
        Self::new()
    }
}

impl<Message> IntoIterator for Children<Message> {
    type Item = ViewNode<Message>;
    type IntoIter = std::vec::IntoIter<ViewNode<Message>>;

    fn into_iter(self) -> Self::IntoIter {
        self.children.into_iter()
    }
}

impl<Message> FromIterator<ViewNode<Message>> for Children<Message> {
    fn from_iter<T: IntoIterator<Item = ViewNode<Message>>>(iter: T) -> Self {
        Self {
            children: iter.into_iter().collect(),
        }
    }
}

/// Build a declarative child list for row, column, grid, stack, and similar
/// container builders.
pub fn children<Message>() -> Children<Message> {
    Children::new()
}

pub(super) fn collect_children<Message>(
    children: impl IntoIterator<Item = ViewNode<Message>>,
) -> (Vec<ViewNode<Message>>, bool) {
    let mut has_reserved_descendant_identity = false;
    let children = children.into_iter();
    let mut collected = Vec::with_capacity(children.size_hint().0);
    for child in children {
        if !has_reserved_descendant_identity && child.has_reserved_identity_in_subtree() {
            has_reserved_descendant_identity = true;
        }
        collected.push(child);
    }
    (collected, has_reserved_descendant_identity)
}
