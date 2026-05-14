/// Action availability for an editable tree or nested list surface.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct EditableTreeActions {
    /// Whether creating a child item under the focused parent is allowed.
    pub can_create_child: bool,
    /// Whether creating an item at the root of the editable tree is allowed.
    pub can_create_root: bool,
    /// Whether renaming the focused item is allowed.
    pub can_rename: bool,
    /// Whether deleting the focused item is allowed.
    pub can_delete: bool,
    /// Whether explicit restore for retained deletes is allowed.
    pub can_restore_retained: bool,
    /// Whether explicit purge for retained deletes is allowed.
    pub can_purge_retained: bool,
    /// Whether clearing the action history is allowed.
    pub can_clear_history: bool,
}
