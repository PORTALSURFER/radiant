use crate::{
    application::{ViewNode, ViewNodeKind, primary_style},
    layout::Vector2,
    widgets::WidgetStyle,
};

/// Build a row container with fill-slot children.
pub fn row<Message>(children: impl IntoIterator<Item = ViewNode<Message>>) -> ViewNode<Message> {
    let (children, has_reserved_descendant_identity) = collect_children(children);
    ViewNode::new(ViewNodeKind::Row {
        spacing: 8.0,
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
        spacing: 6.0,
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
    grid_with_gaps(children, columns, 8.0, 8.0)
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

/// Build a floating overlay panel in surface coordinates.
pub fn overlay_panel<Message>(
    label: impl Into<String>,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
) -> ViewNode<Message> {
    ViewNode::new(ViewNodeKind::OverlayPanel {
        rect: crate::gui::types::Rect::from_min_size(
            crate::gui::types::Point::new(x, y),
            Vector2::new(width, height),
        ),
        label: Some(label.into()),
    })
}

/// Build a floating drop marker in surface coordinates.
pub fn drop_marker<Message>(x: f32, y: f32, width: f32, height: f32) -> ViewNode<Message> {
    ViewNode::new(ViewNodeKind::OverlayPanel {
        rect: crate::gui::types::Rect::from_min_size(
            crate::gui::types::Point::new(x, y),
            Vector2::new(width, height),
        ),
        label: None,
    })
    .style(primary_style())
}

/// Build a scroll viewport around one child view.
pub fn scroll<Message>(child: ViewNode<Message>) -> ViewNode<Message> {
    let has_reserved_descendant_identity = child.has_reserved_identity_in_subtree();
    ViewNode::new(ViewNodeKind::Scroll {
        child: Box::new(child),
    })
    .with_reserved_descendant_identity(has_reserved_descendant_identity)
}

/// Build a vertically virtualized scroll viewport around one child view.
pub fn virtual_scroll<Message>(child: ViewNode<Message>, overscan_px: f32) -> ViewNode<Message> {
    let has_reserved_descendant_identity = child.has_reserved_identity_in_subtree();
    ViewNode::new(ViewNodeKind::VirtualScroll {
        child: Box::new(child),
        overscan_px,
    })
    .with_reserved_descendant_identity(has_reserved_descendant_identity)
}

/// Build a scroll viewport containing a column projected from an iterator.
pub fn scroll_column<Message, Item>(
    items: impl IntoIterator<Item = Item>,
    project: impl FnMut(Item) -> ViewNode<Message>,
) -> ViewNode<Message> {
    scroll(column(items.into_iter().map(project)))
}

/// Build a scrollable vertical list with stable intrinsic-height rows.
pub fn list<Message, Item>(
    items: impl IntoIterator<Item = Item>,
    project: impl FnMut(Item) -> ViewNode<Message>,
) -> ViewNode<Message> {
    scroll_column(items, project)
        .style(WidgetStyle::default())
        .fill_height()
}

/// Build a vertically virtualized list with stable intrinsic-height rows.
pub fn virtual_list<Message, Item>(
    items: impl IntoIterator<Item = Item>,
    project: impl FnMut(Item) -> ViewNode<Message>,
    overscan_px: f32,
) -> ViewNode<Message> {
    virtual_scroll(column(items.into_iter().map(project)), overscan_px)
        .style(WidgetStyle::default())
        .fill_height()
}

/// Build a keyed list row with full-width, fixed-height defaults.
pub fn list_row<Message>(
    key: impl ToString,
    children: impl IntoIterator<Item = ViewNode<Message>>,
) -> ViewNode<Message> {
    apply_list_row_chrome(row_key(key, children))
}

/// Build a list row with a direct numeric id instead of a string key.
///
/// Prefer this for large numeric collections when the caller already owns
/// stable item ids; it avoids per-row key string allocation during projection.
pub fn list_row_id<Message>(
    id: u64,
    children: impl IntoIterator<Item = ViewNode<Message>>,
) -> ViewNode<Message> {
    apply_list_row_chrome(row(children).id(id))
}

fn apply_list_row_chrome<Message>(row: ViewNode<Message>) -> ViewNode<Message> {
    row.style(WidgetStyle::default())
        .hoverable()
        .fill_width()
        .height(52.0)
        .padding_x(18.0)
        .padding_y(10.0)
        .spacing(16.0)
}

fn collect_children<Message>(
    children: impl IntoIterator<Item = ViewNode<Message>>,
) -> (Vec<ViewNode<Message>>, bool) {
    let mut has_reserved_descendant_identity = false;
    let children = children
        .into_iter()
        .inspect(|child| {
            has_reserved_descendant_identity |= child.has_reserved_identity_in_subtree();
        })
        .collect();
    (children, has_reserved_descendant_identity)
}
