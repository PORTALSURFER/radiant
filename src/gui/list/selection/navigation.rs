/// Move an item index by a signed delta, clamped to the current list bounds.
///
/// This helper is intentionally stateless so hosts can keep durable selection
/// identity in paths, ids, or other domain keys while sharing the generic
/// keyboard-navigation rule.
pub fn list_index_after_delta(current: usize, delta: isize, total_items: usize) -> Option<usize> {
    if total_items == 0 {
        return None;
    }
    let last_index = total_items.saturating_sub(1);
    if delta.is_negative() {
        Some(current.saturating_sub(delta.unsigned_abs()).min(last_index))
    } else {
        Some(current.saturating_add(delta as usize).min(last_index))
    }
}

/// Map a normalized unit interval coordinate to a bounded list index.
///
/// This fits hit testing, randomized picks, scrub positions, and other
/// continuous inputs where the host owns item identity but the framework can
/// provide the shared edge behavior. Empty lists return `None`; finite values
/// outside `0.0..=1.0` clamp to the nearest edge; `NaN` maps to the first item.
pub fn unit_interval_index(unit: f32, total_items: usize) -> Option<usize> {
    if total_items == 0 {
        return None;
    }

    let normalized = if unit.is_nan() {
        0.0
    } else {
        unit.clamp(0.0, 1.0)
    };
    let index = (normalized * total_items as f32).floor() as usize;
    Some(index.min(total_items.saturating_sub(1)))
}

/// Move an item index by a signed delta, wrapping around current list bounds.
///
/// This helper fits menus, autocomplete popups, command palettes, and other
/// cyclic selection surfaces where pressing past either edge should continue
/// from the opposite edge.
pub fn cyclic_list_index_after_delta(
    current: usize,
    delta: isize,
    total_items: usize,
) -> Option<usize> {
    if total_items == 0 {
        return None;
    }
    let current = current % total_items;
    Some((current as isize + delta).rem_euclid(total_items as isize) as usize)
}

/// Prefix- or query-bound cyclic selection state for transient list completions.
///
/// Autocomplete popups, command palettes, dropdown searches, and similar
/// surfaces often need the same policy: keep cycling within the current query,
/// reset to the first item when the query changes, and clear state when there
/// are no visible items. Hosts still own item filtering and item identity.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CyclicListSelectionCycle {
    query_key: Option<String>,
    selected_index: usize,
}

impl CyclicListSelectionCycle {
    /// Build an empty cyclic selection cycle.
    pub const fn new() -> Self {
        Self {
            query_key: None,
            selected_index: 0,
        }
    }

    /// Return the query key this selection currently tracks, if any.
    pub fn query_key(&self) -> Option<&str> {
        self.query_key.as_deref()
    }

    /// Return the stored selected index before applying a visible item count.
    pub const fn stored_index(&self) -> usize {
        self.selected_index
    }

    /// Return the selected index for `query_key` and `total_items`.
    ///
    /// A matching query reuses the stored index modulo the current item count.
    /// A new query starts at the first item without mutating this controller so
    /// callers can compute display selection from immutable view state.
    pub fn selected_index(&self, query_key: &str, total_items: usize) -> Option<usize> {
        if total_items == 0 {
            return None;
        }
        if self.query_key.as_deref() == Some(query_key) {
            Some(self.selected_index % total_items)
        } else {
            Some(0)
        }
    }

    /// Return the selected index only when `query_key` is already active.
    ///
    /// This is useful for autocomplete surfaces that should display suggestions
    /// for a fresh query without preselecting the first option.
    pub fn active_selected_index(&self, query_key: &str, total_items: usize) -> Option<usize> {
        if self.query_key.as_deref() == Some(query_key) {
            self.selected_index(query_key, total_items)
        } else {
            None
        }
    }

    /// Move the selected index by `delta`, wrapping inside the current query.
    ///
    /// When `query_key` differs from the tracked query, movement starts from the
    /// first visible item before applying `delta`. Empty lists clear the cycle.
    pub fn move_selection(
        &mut self,
        query_key: impl Into<String>,
        delta: isize,
        total_items: usize,
    ) -> Option<usize> {
        if total_items == 0 {
            self.reset();
            return None;
        }
        let query_key = query_key.into();
        let current = self.selected_index(query_key.as_str(), total_items)?;
        let selected = cyclic_list_index_after_delta(current, delta, total_items)?;
        self.query_key = Some(query_key);
        self.selected_index = selected;
        Some(selected)
    }

    /// Move within the current query or select an edge item for a new query.
    ///
    /// A fresh positive or zero movement selects the first item; a fresh
    /// negative movement selects the last item. Once the query is active,
    /// movement wraps from the current selection like [`Self::move_selection`].
    pub fn move_selection_from_edge(
        &mut self,
        query_key: impl Into<String>,
        delta: isize,
        total_items: usize,
    ) -> Option<usize> {
        if total_items == 0 {
            self.reset();
            return None;
        }
        let query_key = query_key.into();
        if self.query_key.as_deref() == Some(query_key.as_str()) {
            return self.move_selection(query_key, delta, total_items);
        }
        let selected = if delta < 0 {
            total_items.saturating_sub(1)
        } else {
            0
        };
        self.query_key = Some(query_key);
        self.selected_index = selected;
        Some(selected)
    }

    /// Explicitly set the selected index for `query_key`.
    ///
    /// The stored index is normalized to the current item count. Empty lists
    /// clear the cycle.
    pub fn select(
        &mut self,
        query_key: impl Into<String>,
        index: usize,
        total_items: usize,
    ) -> Option<usize> {
        if total_items == 0 {
            self.reset();
            return None;
        }
        let selected = index % total_items;
        self.query_key = Some(query_key.into());
        self.selected_index = selected;
        Some(selected)
    }

    /// Clear tracked query and index.
    pub fn reset(&mut self) {
        self.query_key = None;
        self.selected_index = 0;
    }
}
