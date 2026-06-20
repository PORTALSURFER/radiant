use super::VirtualListProjection;

/// App-owned focus key memory for virtual lists that should follow selection
/// only when the selected item changes.
///
/// Pair this with [`super::VirtualListController::configure_and_focus_changed_optional`]
/// when manual scroll should remain authoritative until the app selection moves
/// to a different key.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VirtualListFollowState<Key> {
    pub(super) focus_key: Option<Key>,
}

/// App-owned focus key and current item index for a virtualized list.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VirtualListFocusTarget<Key> {
    /// Stable app-owned key for the focused item.
    pub key: Option<Key>,
    /// Current visible item index for the same key.
    pub index: Option<usize>,
}

/// Projection inputs and key-resolved focus for a current item slice.
///
/// Build this before mutably borrowing a [`super::VirtualListController`] when the
/// current projection slice borrows from the same host state as the controller.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VirtualListSliceFocus<Key> {
    pub(super) projection: VirtualListProjection,
    pub(super) focus: VirtualListFocusTarget<Key>,
}

impl<Key> VirtualListFocusTarget<Key> {
    /// Build a focus target from an app-owned key and current item index.
    pub const fn new(key: Option<Key>, index: Option<usize>) -> Self {
        Self { key, index }
    }

    /// Build an empty focus target.
    pub const fn none() -> Self {
        Self {
            key: None,
            index: None,
        }
    }
}

impl<Key: Clone> VirtualListFocusTarget<Key> {
    /// Build a focus target by locating a stable key inside a projected item slice.
    ///
    /// This keeps app code focused on its domain item projection while Radiant
    /// owns the common "key plus current index" shape used by virtual-list
    /// selection-follow. If the key is missing from the current slice, the
    /// returned target is empty so follow state clears instead of anchoring to a
    /// stale item.
    pub fn from_slice_by<Item>(
        items: &[Item],
        key: Option<Key>,
        mut matches_key: impl FnMut(&Item, &Key) -> bool,
    ) -> Self {
        let Some(key) = key else {
            return Self::none();
        };
        let index = items.iter().position(|item| matches_key(item, &key));
        match index {
            Some(index) => Self::new(Some(key), Some(index)),
            None => Self::none(),
        }
    }
}

impl<Key> VirtualListSliceFocus<Key> {
    /// Build slice-focus inputs from named projection inputs and a focus target.
    pub const fn new(
        projection: VirtualListProjection,
        focus: VirtualListFocusTarget<Key>,
    ) -> Self {
        Self { projection, focus }
    }

    /// Return the projection inputs.
    pub const fn projection(&self) -> VirtualListProjection {
        self.projection
    }

    /// Return the key-resolved focus target.
    pub const fn focus(&self) -> &VirtualListFocusTarget<Key> {
        &self.focus
    }

    /// Add context rows to the focus-follow guard band.
    pub fn with_context_rows(self, context_rows: usize) -> Self {
        Self {
            projection: self.projection.with_context_rows(context_rows),
            ..self
        }
    }

    /// Add one context row to the focus-follow guard band.
    pub fn with_context_row(self) -> Self {
        self.with_context_rows(1)
    }
}

impl<Key: Clone> VirtualListSliceFocus<Key> {
    /// Build slice-focus inputs by locating a stable key inside the current item
    /// slice and deriving projection item count from that same slice.
    pub fn from_slice_by<Item>(
        items: &[Item],
        viewport_len: usize,
        overscan: usize,
        guard_band: usize,
        key: Option<Key>,
        matches_key: impl FnMut(&Item, &Key) -> bool,
    ) -> Self {
        Self::new(
            VirtualListProjection::for_slice(items, viewport_len, overscan, guard_band),
            VirtualListFocusTarget::from_slice_by(items, key, matches_key),
        )
    }
}

impl<Key> Default for VirtualListFollowState<Key> {
    fn default() -> Self {
        Self::new()
    }
}

impl<Key> VirtualListFollowState<Key> {
    /// Build an empty follow state.
    pub const fn new() -> Self {
        Self { focus_key: None }
    }

    /// Return the last followed focus key.
    pub const fn focus_key(&self) -> Option<&Key> {
        self.focus_key.as_ref()
    }

    /// Clear the last followed focus key.
    pub fn clear(&mut self) {
        self.focus_key = None;
    }

    /// Remember the current focus key without triggering follow scrolling.
    ///
    /// Hosts can use this after a user manually scrolls and then selects an item
    /// that is already visible. The selection state still advances, but manual
    /// scroll remains authoritative until the next out-of-view focus change.
    pub fn remember_focus_key(&mut self, focus_key: Option<Key>) {
        self.focus_key = focus_key;
    }
}

impl<Key: PartialEq> VirtualListFollowState<Key> {
    pub(super) fn update_focus_key(&mut self, focus_key: Option<Key>) -> bool {
        if self.focus_key == focus_key {
            return false;
        }
        self.focus_key = focus_key;
        true
    }
}
