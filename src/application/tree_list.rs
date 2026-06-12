use crate::{
    application::{View, column, scroll},
    widgets::DragHandleMessage,
};
use std::sync::Arc;

mod row;

use row::message_tree_list_row;

/// Named construction inputs for one visible tree-list row.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TreeListItemParts {
    /// Stable caller-owned item id.
    pub id: String,
    /// Zero-based visual depth.
    pub depth: usize,
    /// Row label.
    pub label: String,
}

/// One visible row in a compact tree list.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TreeListItem {
    /// Stable caller-owned item id.
    pub id: String,
    /// Zero-based visual depth.
    pub depth: usize,
    /// Row label.
    pub label: String,
    /// Whether this row can be expanded or collapsed.
    pub has_children: bool,
    /// Whether this branch is currently expanded.
    pub expanded: bool,
    /// Whether this row is currently selected.
    pub selected: bool,
    /// Whether this row should show a drag handle.
    pub draggable: bool,
    /// Whether this row is the current drop target.
    pub drop_target: bool,
}

impl TreeListItem {
    /// Build one visible tree-list row from named construction inputs.
    pub fn from_parts(parts: TreeListItemParts) -> Self {
        Self {
            id: parts.id,
            depth: parts.depth,
            label: parts.label,
            has_children: false,
            expanded: false,
            selected: false,
            draggable: false,
            drop_target: false,
        }
    }

    /// Build one visible tree-list row.
    pub fn new(id: impl ToString, depth: usize, label: impl Into<String>) -> Self {
        Self::from_parts(TreeListItemParts {
            id: id.to_string(),
            depth,
            label: label.into(),
        })
    }

    /// Mark the row as expandable or collapsible.
    pub fn branch(mut self, expanded: bool) -> Self {
        self.has_children = true;
        self.expanded = expanded;
        self
    }

    /// Mark the row as selected.
    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    /// Show a compact drag handle before the row label.
    pub fn draggable(mut self, draggable: bool) -> Self {
        self.draggable = draggable;
        self
    }

    /// Mark this row as the current drop target.
    pub fn drop_target(mut self, drop_target: bool) -> Self {
        self.drop_target = drop_target;
        self
    }
}

/// Build a compact tree list that emits select and toggle messages.
pub fn message_tree_list<Message: 'static>(
    items: impl IntoIterator<Item = TreeListItem>,
    select_message: impl Fn(String) -> Message + Send + Sync + 'static,
    toggle_message: impl Fn(String) -> Message + Send + Sync + 'static,
) -> View<Message>
where
    Message: Clone + Send + Sync + 'static,
{
    message_tree_list_with_drag(
        items,
        select_message,
        toggle_message,
        None::<fn(String) -> Message>,
        None::<fn(String, DragHandleMessage) -> Message>,
    )
}

/// Build a compact tree list with optional context and drag messages.
pub fn message_tree_list_with_drag<Message: 'static>(
    items: impl IntoIterator<Item = TreeListItem>,
    select_message: impl Fn(String) -> Message + Send + Sync + 'static,
    toggle_message: impl Fn(String) -> Message + Send + Sync + 'static,
    context_message: Option<impl Fn(String) -> Message + Send + Sync + 'static>,
    drag_message: Option<impl Fn(String, DragHandleMessage) -> Message + Send + Sync + 'static>,
) -> View<Message>
where
    Message: Clone + Send + Sync + 'static,
{
    let select_message = Arc::new(select_message) as Arc<dyn Fn(String) -> Message + Send + Sync>;
    let toggle_message = Arc::new(toggle_message) as Arc<dyn Fn(String) -> Message + Send + Sync>;
    let context_message = context_message.map(|context_message| {
        Arc::new(context_message) as Arc<dyn Fn(String) -> Message + Send + Sync>
    });
    let drag_message = drag_message.map(|drag_message| {
        Arc::new(drag_message) as Arc<dyn Fn(String, DragHandleMessage) -> Message + Send + Sync>
    });

    scroll(
        column(items.into_iter().map(|item| {
            message_tree_list_row(
                item,
                Arc::clone(&select_message),
                Arc::clone(&toggle_message),
                context_message.as_ref().map(Arc::clone),
                drag_message.as_ref().map(Arc::clone),
            )
        }))
        .fill_width()
        .spacing(1.0),
    )
    .fill_height()
}
