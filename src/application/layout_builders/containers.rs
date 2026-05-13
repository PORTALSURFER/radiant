//! Row, column, grid, and stack layout builders.

use super::collection::collect_children;
use crate::application::{ViewNode, ViewNodeKind};

/// Default main-axis gap for Radiant application row containers.
pub const DEFAULT_ROW_SPACING: f32 = 4.0;

/// Default main-axis gap for Radiant application column containers.
pub const DEFAULT_COLUMN_SPACING: f32 = 4.0;

/// Default gap for Radiant application grid containers.
pub const DEFAULT_GRID_GAP: f32 = 4.0;

/// Build a row container with fill-slot children.
pub fn row<Message>(children: impl IntoIterator<Item = ViewNode<Message>>) -> ViewNode<Message> {
    let (children, has_reserved_descendant_identity) = collect_children(children);
    ViewNode::new(ViewNodeKind::Row {
        spacing: DEFAULT_ROW_SPACING,
        children,
    })
    .with_reserved_descendant_identity(has_reserved_descendant_identity)
}

/// Build a keyed row container with fill-slot children.
pub fn row_key<Message>(
    key: impl ToString,
    children: impl IntoIterator<Item = ViewNode<Message>>,
) -> ViewNode<Message> {
    row(children).key(key)
}

/// Build a column container with fill-slot children.
pub fn column<Message>(children: impl IntoIterator<Item = ViewNode<Message>>) -> ViewNode<Message> {
    let (children, has_reserved_descendant_identity) = collect_children(children);
    ViewNode::new(ViewNodeKind::Column {
        spacing: DEFAULT_COLUMN_SPACING,
        children,
    })
    .with_reserved_descendant_identity(has_reserved_descendant_identity)
}

/// Build a keyed column container with fill-slot children.
pub fn column_key<Message>(
    key: impl ToString,
    children: impl IntoIterator<Item = ViewNode<Message>>,
) -> ViewNode<Message> {
    column(children).key(key)
}

/// Build a grid container with a fixed column count and default gaps.
pub fn grid<Message>(
    children: impl IntoIterator<Item = ViewNode<Message>>,
    columns: usize,
) -> ViewNode<Message> {
    grid_with_gaps(children, columns, DEFAULT_GRID_GAP, DEFAULT_GRID_GAP)
}

/// Build a grid container with a fixed column count and explicit gaps.
pub fn grid_with_gaps<Message>(
    children: impl IntoIterator<Item = ViewNode<Message>>,
    columns: usize,
    column_gap: f32,
    row_gap: f32,
) -> ViewNode<Message> {
    let (children, has_reserved_descendant_identity) = collect_children(children);
    ViewNode::new(ViewNodeKind::Grid {
        columns,
        column_gap,
        row_gap,
        children,
    })
    .with_reserved_descendant_identity(has_reserved_descendant_identity)
}

/// Build a stack container that overlays children in paint order.
pub fn stack<Message>(children: impl IntoIterator<Item = ViewNode<Message>>) -> ViewNode<Message> {
    let (children, has_reserved_descendant_identity) = collect_children(children);
    ViewNode::new(ViewNodeKind::Stack { children })
        .with_reserved_descendant_identity(has_reserved_descendant_identity)
}
