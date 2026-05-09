use crate::gui::types::Rect;

/// Stable host-supplied identity for one virtual-list item.
///
/// The key is intentionally opaque to Radiant. Hosts can derive it from an ID,
/// path hash, database row id, or any other stable origin while Radiant keeps
/// retained windows and state overlays independent from domain concepts.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct VirtualListItemKey(pub u64);

/// Generic visual state carried by one materialized list item.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct VirtualListItemState {
    /// The item is part of the current selection.
    pub selected: bool,
    /// The item owns keyboard focus or caret state.
    pub focused: bool,
    /// The pointer is currently hovering the item.
    pub hovered: bool,
    /// The item is the current active drag/drop or operation target.
    pub active_target: bool,
    /// The item should render with disabled/unavailable treatment.
    pub disabled: bool,
    /// Transient operation state for retained row overlays.
    pub overlay: VirtualListItemOverlay,
}

/// Domain-neutral retained item overlay.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum VirtualListItemOverlay {
    /// No transient overlay is active.
    #[default]
    None,
    /// The item is waiting for a host-defined operation.
    Queued,
    /// The item is actively being processed by a host-defined operation.
    Active,
    /// The last host-defined operation completed successfully.
    Completed,
    /// The last host-defined operation failed.
    Failed,
}

impl VirtualListItemState {
    /// Return whether this state needs a retained overlay segment.
    pub fn requires_overlay(self) -> bool {
        self.selected
            || self.focused
            || self.hovered
            || self.active_target
            || self.disabled
            || self.overlay != VirtualListItemOverlay::None
    }
}

/// One materialized virtual-list item with stable identity and window geometry.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MaterializedVirtualListItem {
    /// Stable host-supplied item identity.
    pub key: VirtualListItemKey,
    /// Logical item index in the full list.
    pub index: usize,
    /// Window-space row bounds.
    pub rect: Rect,
    /// Domain-neutral retained item state.
    pub state: VirtualListItemState,
}

impl MaterializedVirtualListItem {
    /// Build one materialized item.
    pub fn new(key: VirtualListItemKey, index: usize, rect: Rect) -> Self {
        Self {
            key,
            index,
            rect,
            state: VirtualListItemState::default(),
        }
    }

    /// Attach retained item state.
    pub fn with_state(mut self, state: VirtualListItemState) -> Self {
        self.state = state;
        self
    }
}
