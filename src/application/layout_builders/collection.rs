//! Shared child collection helpers for layout builders.

use crate::application::{ViewNode, view_node::KeyedIdentity};
use std::hash::Hash;

/// Supplies a stable, typed identity for an item in a repeated view.
pub trait Keyed {
    /// Domain key used to retain the item's view identity across reorderings.
    type Key: Eq + Hash + ?Sized + 'static;

    /// Return the item's stable domain key.
    fn key(&self) -> &Self::Key;
}

impl<Item: Keyed + ?Sized> Keyed for &Item {
    type Key = Item::Key;

    fn key(&self) -> &Self::Key {
        (*self).key()
    }
}

impl<Item: Keyed + ?Sized> Keyed for &mut Item {
    type Key = Item::Key;

    fn key(&self) -> &Self::Key {
        (**self).key()
    }
}

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

/// Build child roots for a repeated collection using the item's [`Keyed`] key.
pub fn for_each<Items, Item, Message, Project>(
    items: Items,
    mut project: Project,
) -> Children<Message>
where
    Items: IntoIterator<Item = Item>,
    Item: Keyed,
    Project: FnMut(Item) -> ViewNode<Message>,
{
    items
        .into_iter()
        .map(|item| {
            let key = item.key();
            let identity = KeyedIdentity::from_key(key);
            project(item).with_inferred_keyed_identity(identity)
        })
        .collect()
}

/// Build child roots for a repeated collection using an explicit key extractor.
pub fn for_each_by<Items, Item, Key, Extract, Project, Message>(
    items: Items,
    mut extract: Extract,
    mut project: Project,
) -> Children<Message>
where
    Items: IntoIterator<Item = Item>,
    Key: Eq + Hash + 'static,
    Extract: FnMut(&Item) -> Key,
    Project: FnMut(Item) -> ViewNode<Message>,
{
    items
        .into_iter()
        .map(|item| {
            let key = extract(&item);
            let identity = KeyedIdentity::from_key(&key);
            project(item).with_inferred_keyed_identity(identity)
        })
        .collect()
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::{IntoView, column, overlays, row, scene, text};
    use crate::layout::{LayoutNode, NodeId};
    use std::hash::{Hash, Hasher};

    struct Item {
        id: u32,
        label: &'static str,
    }

    impl Keyed for Item {
        type Key = u32;

        fn key(&self) -> &Self::Key {
            &self.id
        }
    }

    struct NonCloneKey(u32);

    impl PartialEq for NonCloneKey {
        fn eq(&self, other: &Self) -> bool {
            self.0 == other.0
        }
    }

    impl Eq for NonCloneKey {}

    impl Hash for NonCloneKey {
        fn hash<H: Hasher>(&self, state: &mut H) {
            self.0.hash(state);
        }
    }

    struct NonCloneItem {
        key: NonCloneKey,
    }

    impl Keyed for NonCloneItem {
        type Key = NonCloneKey;

        fn key(&self) -> &Self::Key {
            &self.key
        }
    }

    struct CollisionKey(u32);

    impl PartialEq for CollisionKey {
        fn eq(&self, other: &Self) -> bool {
            self.0 == other.0
        }
    }

    impl Eq for CollisionKey {}

    impl Hash for CollisionKey {
        fn hash<H: Hasher>(&self, state: &mut H) {
            1_u8.hash(state);
        }
    }

    struct CollisionItem {
        key: CollisionKey,
    }

    #[derive(Eq, PartialEq)]
    struct AlphaKey(u32);

    impl Hash for AlphaKey {
        fn hash<H: Hasher>(&self, state: &mut H) {
            1_u8.hash(state);
        }
    }

    #[derive(Eq, PartialEq)]
    struct BetaKey(u32);

    impl Hash for BetaKey {
        fn hash<H: Hasher>(&self, state: &mut H) {
            1_u8.hash(state);
        }
    }

    struct DualItem {
        alpha: AlphaKey,
        beta: BetaKey,
    }

    fn ids(node: &LayoutNode, output: &mut Vec<NodeId>) {
        output.push(node.id());
        if let LayoutNode::Container(container) = node {
            for child in &container.children {
                ids(&child.child, output);
            }
        }
    }

    fn projected(view: crate::application::ViewNode<()>) -> Vec<NodeId> {
        let mut output = Vec::new();
        ids(&view.into_surface().layout_node(), &mut output);
        output
    }

    fn assert_ambiguous(view: crate::application::ViewNode<()>) {
        let panic = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| projected(view)))
            .expect_err("ambiguous keyed identity should reject projection");
        let message = panic
            .downcast_ref::<&str>()
            .copied()
            .or_else(|| panic.downcast_ref::<String>().map(String::as_str));
        assert_eq!(message, Some("ambiguous keyed identity"));
    }

    fn items(order: &[u32]) -> Vec<Item> {
        order
            .iter()
            .map(|id| Item {
                id: *id,
                label: "item",
            })
            .collect()
    }

    #[test]
    fn keyed_iteration_accepts_borrowed_items_and_preserves_reordered_roots() {
        let before = projected(column(for_each(items(&[1, 2, 3]).iter(), |item| {
            text(item.label)
        })));
        let after = projected(column(for_each(items(&[3, 1, 2]).iter(), |item| {
            text(item.label)
        })));

        assert_eq!(before[1], after[2]);
        assert_eq!(before[2], after[3]);
        assert_eq!(before[3], after[1]);
    }

    #[test]
    fn explicit_extractor_matches_keyed_identity() {
        let keyed = projected(column(for_each(items(&[1, 2]), |item| text(item.label))));
        let extracted = projected(column(for_each_by(
            items(&[1, 2]).iter(),
            |item| item.id,
            |item| text(item.label),
        )));

        assert_eq!(&keyed[1..], &extracted[1..]);
    }

    #[test]
    fn keyed_identity_includes_key_type_and_root_kind() {
        let u32_id = projected(column(for_each(items(&[7]), |item| text(item.label))))[1];
        let alpha_id = projected(column(for_each_by(
            [DualItem {
                alpha: AlphaKey(7),
                beta: BetaKey(7),
            }],
            |item| AlphaKey(item.alpha.0),
            |_| text("item"),
        )))[1];
        let beta_id = projected(column(for_each_by(
            [DualItem {
                alpha: AlphaKey(7),
                beta: BetaKey(7),
            }],
            |item| BetaKey(item.beta.0),
            |_| text("item"),
        )))[1];
        let container_id = projected(column(for_each(items(&[7]), |item| {
            row([text(item.label)])
        })))[1];

        assert_ne!(alpha_id, beta_id);
        assert_ne!(u32_id, container_id);
    }

    #[test]
    fn duplicate_keys_reject_projection_deterministically() {
        assert_ambiguous(column(for_each(
            [Item { id: 1, label: "a" }, Item { id: 1, label: "b" }],
            |item| row([text(item.label)]),
        )));
        assert_ambiguous(column(for_each(
            [Item { id: 1, label: "a" }, Item { id: 1, label: "b" }],
            |item| row([text(item.label)]),
        )));
    }

    #[test]
    fn explicit_identity_overrides_inferred_key() {
        let ids = projected(column(for_each(items(&[1]), |item| {
            text(item.label).id(9001)
        })));
        assert_eq!(ids[1], 9001);

        let keyed = projected(column(for_each(items(&[1]), |item| {
            text(item.label).key("explicit")
        })));
        assert_eq!(
            keyed[1],
            crate::application::scoped_key_id(keyed[0], "explicit")
        );

        let suppressed = projected(column(for_each(
            [
                Item {
                    id: 1,
                    label: "explicit",
                },
                Item {
                    id: 1,
                    label: "inferred",
                },
            ],
            |item| {
                if item.label == "explicit" {
                    text(item.label).key("explicit")
                } else {
                    text(item.label)
                }
            },
        )));
        assert_eq!(suppressed.len(), 3);
    }

    #[test]
    fn keyed_iteration_supports_non_clone_items_and_keys() {
        let items = [NonCloneItem {
            key: NonCloneKey(1),
        }];
        let ids = projected(column(for_each(items.iter(), |_| text("item"))));
        assert_eq!(ids.len(), 2);
    }

    #[test]
    fn unequal_same_hash_keys_reject_in_both_orders() {
        assert_ambiguous(column(for_each_by(
            [
                CollisionItem {
                    key: CollisionKey(1),
                },
                CollisionItem {
                    key: CollisionKey(2),
                },
            ],
            |item| CollisionKey(item.key.0),
            |_| text("item"),
        )));
        assert_ambiguous(column(for_each_by(
            [
                CollisionItem {
                    key: CollisionKey(2),
                },
                CollisionItem {
                    key: CollisionKey(1),
                },
            ],
            |item| CollisionKey(item.key.0),
            |_| text("item"),
        )));
    }

    #[test]
    fn view_local_overlay_collisions_reject_projection() {
        fn overlay_scene(order: &[u32]) -> crate::application::ViewNode<()> {
            let items = order
                .iter()
                .map(|id| CollisionItem {
                    key: CollisionKey(*id),
                })
                .collect::<Vec<_>>();
            scene(
                text("base").overlays(overlays().floating(column(for_each_by(
                    items.iter(),
                    |item| CollisionKey(item.key.0),
                    |_| text("item"),
                )))),
            )
            .into_view()
        }

        assert_ambiguous(overlay_scene(&[1, 2]));
        assert_ambiguous(overlay_scene(&[2, 1]));
    }

    #[test]
    fn separate_keyed_collections_with_same_parent_scope_reject() {
        let first = for_each([Item { id: 1, label: "a" }], |item| text(item.label));
        let second = for_each([Item { id: 1, label: "b" }], |item| text(item.label));
        assert_ambiguous(column(first.into_iter().chain(second)));
    }
}
