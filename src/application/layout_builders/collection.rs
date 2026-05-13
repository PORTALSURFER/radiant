//! Shared child collection helpers for layout builders.

use crate::application::ViewNode;

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
