/// Build a row container with fill-slot children.
pub fn row<Message>(children: impl IntoIterator<Item = ViewNode<Message>>) -> ViewNode<Message> {
    ViewNode {
        kind: ViewNodeKind::Row {
            spacing: 8.0,
            children: children.into_iter().collect(),
        },
        id: None,
        key: None,
        sizing: None,
        slot: SlotBehavior::default(),
        padding: Insets::default(),
        align_main: None,
        align_cross: None,
        style: None,
        hoverable: false,
        input_only: false,
        text_wrap: None,
        text_align: None,
    }
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
    ViewNode {
        kind: ViewNodeKind::Column {
            spacing: 6.0,
            children: children.into_iter().collect(),
        },
        id: None,
        key: None,
        sizing: None,
        slot: SlotBehavior::default(),
        padding: Insets::default(),
        align_main: None,
        align_cross: None,
        style: None,
        hoverable: false,
        input_only: false,
        text_wrap: None,
        text_align: None,
    }
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
    ViewNode {
        kind: ViewNodeKind::Grid {
            columns,
            column_gap,
            row_gap,
            children: children.into_iter().collect(),
        },
        id: None,
        key: None,
        sizing: None,
        slot: SlotBehavior::default(),
        padding: Insets::default(),
        align_main: None,
        align_cross: None,
        style: None,
        hoverable: false,
        input_only: false,
        text_wrap: None,
        text_align: None,
    }
}

/// Build a stack container that overlays children in paint order.
pub fn stack<Message>(children: impl IntoIterator<Item = ViewNode<Message>>) -> ViewNode<Message> {
    ViewNode {
        kind: ViewNodeKind::Stack {
            children: children.into_iter().collect(),
        },
        id: None,
        key: None,
        sizing: None,
        slot: SlotBehavior::default(),
        padding: Insets::default(),
        align_main: None,
        align_cross: None,
        style: None,
        hoverable: false,
        input_only: false,
        text_wrap: None,
        text_align: None,
    }
}

/// Build a floating overlay panel in surface coordinates.
pub fn overlay_panel<Message>(
    label: impl Into<String>,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
) -> ViewNode<Message> {
    ViewNode {
        kind: ViewNodeKind::OverlayPanel {
            rect: crate::gui::types::Rect::from_min_size(
                crate::gui::types::Point::new(x, y),
                Vector2::new(width, height),
            ),
            label: Some(label.into()),
        },
        id: None,
        key: None,
        sizing: None,
        slot: SlotBehavior::default(),
        padding: Insets::default(),
        align_main: None,
        align_cross: None,
        style: None,
        hoverable: false,
        input_only: false,
        text_wrap: None,
        text_align: None,
    }
}

/// Build a floating drop marker in surface coordinates.
pub fn drop_marker<Message>(x: f32, y: f32, width: f32, height: f32) -> ViewNode<Message> {
    ViewNode {
        kind: ViewNodeKind::OverlayPanel {
            rect: crate::gui::types::Rect::from_min_size(
                crate::gui::types::Point::new(x, y),
                Vector2::new(width, height),
            ),
            label: None,
        },
        id: None,
        key: None,
        sizing: None,
        slot: SlotBehavior::default(),
        padding: Insets::default(),
        align_main: None,
        align_cross: None,
        style: Some(primary_style()),
        hoverable: false,
        input_only: false,
        text_wrap: None,
        text_align: None,
    }
}

/// Build a scroll viewport around one child view.
pub fn scroll<Message>(child: ViewNode<Message>) -> ViewNode<Message> {
    ViewNode {
        kind: ViewNodeKind::Scroll {
            child: Box::new(child),
        },
        id: None,
        key: None,
        sizing: None,
        slot: SlotBehavior::default(),
        padding: Insets::default(),
        align_main: None,
        align_cross: None,
        style: None,
        hoverable: false,
        input_only: false,
        text_wrap: None,
        text_align: None,
    }
}

/// Build a vertically virtualized scroll viewport around one child view.
pub fn virtual_scroll<Message>(child: ViewNode<Message>, overscan_px: f32) -> ViewNode<Message> {
    ViewNode {
        kind: ViewNodeKind::VirtualScroll {
            child: Box::new(child),
            overscan_px,
        },
        id: None,
        key: None,
        sizing: None,
        slot: SlotBehavior::default(),
        padding: Insets::default(),
        align_main: None,
        align_cross: None,
        style: None,
        hoverable: false,
        input_only: false,
        text_wrap: None,
        text_align: None,
    }
}

/// Build a scroll viewport containing a column projected from an iterator.
pub fn scroll_column<Message, Item>(
    items: impl IntoIterator<Item = Item>,
    project: impl FnMut(Item) -> ViewNode<Message>,
) -> ViewNode<Message> {
    scroll(column(items.into_iter().map(project).collect::<Vec<_>>()))
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
    virtual_scroll(column(items.into_iter().map(project).collect::<Vec<_>>()), overscan_px)
        .style(WidgetStyle::default())
        .fill_height()
}

/// Build a keyed list row with full-width, fixed-height defaults.
pub fn list_row<Message>(
    key: impl ToString,
    children: impl IntoIterator<Item = ViewNode<Message>>,
) -> ViewNode<Message> {
    row_key(key, children)
        .style(WidgetStyle::default())
        .hoverable()
        .fill_width()
        .height(52.0)
        .padding_x(18.0)
        .padding_y(10.0)
        .spacing(16.0)
}
