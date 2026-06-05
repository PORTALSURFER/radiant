use crate::gui::selection::SelectionSet;

/// Modifier state for an index-list selection request.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ListSelectionModifiers {
    /// Extend selection from the current anchor to the requested index.
    pub extend: bool,
    /// Toggle the requested index without replacing the rest of the selection.
    pub toggle: bool,
}

impl ListSelectionModifiers {
    /// Build empty selection modifiers.
    pub const fn new() -> Self {
        Self {
            extend: false,
            toggle: false,
        }
    }

    /// Build modifiers for range extension.
    pub const fn extend() -> Self {
        Self {
            extend: true,
            toggle: false,
        }
    }

    /// Build modifiers for membership toggle.
    pub const fn toggle() -> Self {
        Self {
            extend: false,
            toggle: true,
        }
    }

    /// Build modifiers from common range-extension and toggle flags.
    ///
    /// When both flags are true, range extension wins because this compact
    /// modifier type cannot represent additive range selection. Use
    /// [`ListSelectionIntent::from_extend_toggle`] with `select_with_intent`
    /// when additive range selection should preserve existing membership.
    pub const fn from_extend_toggle(extend: bool, toggle: bool) -> Self {
        if extend {
            Self::extend()
        } else if toggle {
            Self::toggle()
        } else {
            Self::new()
        }
    }
}

/// High-level selection request for one row in a list.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ListSelectionIntent {
    /// Replace selection with the requested row.
    #[default]
    Replace,
    /// Extend selection from the current anchor to the requested row.
    Extend,
    /// Toggle the requested row without replacing the rest of the selection.
    Toggle,
    /// Extend selection from the current anchor while preserving existing membership.
    ExtendPreservingExisting,
}

impl ListSelectionIntent {
    /// Build a selection intent from common range-extension and toggle flags.
    ///
    /// `extend && toggle` maps to additive range selection, matching common
    /// multi-select behavior where Shift+Command/Ctrl preserves previous
    /// membership while adding the anchor-to-target range.
    pub const fn from_extend_toggle(extend: bool, toggle: bool) -> Self {
        match (extend, toggle) {
            (true, true) => Self::ExtendPreservingExisting,
            (true, false) => Self::Extend,
            (false, true) => Self::Toggle,
            (false, false) => Self::Replace,
        }
    }

    /// Return the compact modifier form for intents representable by
    /// [`ListSelectionModifiers`].
    ///
    /// `ExtendPreservingExisting` maps to `Extend`; callers that need additive
    /// range semantics should use `select_with_intent`.
    pub const fn modifiers(self) -> ListSelectionModifiers {
        match self {
            Self::Replace => ListSelectionModifiers::new(),
            Self::Extend | Self::ExtendPreservingExisting => ListSelectionModifiers::extend(),
            Self::Toggle => ListSelectionModifiers::toggle(),
        }
    }
}

impl From<ListSelectionIntent> for ListSelectionModifiers {
    fn from(intent: ListSelectionIntent) -> Self {
        intent.modifiers()
    }
}

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

/// Reusable index-based focus, anchor, and multi-selection state for dense lists.
///
/// Hosts keep ownership of durable row identity. This type tracks logical row
/// indices so apps can map selected rows back to paths, database ids, or other
/// domain keys after filtering and sorting.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ListSelectionController {
    focused_index: Option<usize>,
    anchor_index: Option<usize>,
    selected_indices: Vec<usize>,
    revision: u64,
}

impl ListSelectionController {
    /// Build an empty selection controller.
    pub const fn new() -> Self {
        Self {
            focused_index: None,
            anchor_index: None,
            selected_indices: Vec::new(),
            revision: 0,
        }
    }

    /// Return the focused row index, if any.
    pub const fn focused_index(&self) -> Option<usize> {
        self.focused_index
    }

    /// Return the range-selection anchor index, if any.
    pub const fn anchor_index(&self) -> Option<usize> {
        self.anchor_index
    }

    /// Return sorted selected row indices.
    pub fn selected_indices(&self) -> &[usize] {
        &self.selected_indices
    }

    /// Return a monotonic revision bumped when selection membership changes.
    pub const fn revision(&self) -> u64 {
        self.revision
    }

    /// Return whether the requested row index is currently selected.
    pub fn is_selected(&self, index: usize) -> bool {
        self.selected_indices.binary_search(&index).is_ok()
    }

    /// Clear focus, anchor, and selected membership.
    pub fn clear(&mut self) {
        self.focused_index = None;
        self.anchor_index = None;
        self.replace_selection(Vec::new());
    }

    /// Clamp focus, anchor, and selected membership to the current item count.
    pub fn clamp_to_len(&mut self, total_items: usize) {
        if self.focused_index.is_some_and(|index| index >= total_items) {
            self.focused_index = None;
        }
        if self.anchor_index.is_some_and(|index| index >= total_items) {
            self.anchor_index = self.focused_index;
        }
        let original_len = self.selected_indices.len();
        self.selected_indices.retain(|index| *index < total_items);
        if self.selected_indices.len() != original_len {
            self.bump_revision();
        }
    }

    /// Move focus without changing selection membership.
    pub fn focus(&mut self, index: usize, total_items: usize) -> bool {
        if index >= total_items {
            return false;
        }
        self.focused_index = Some(index);
        self.anchor_index.get_or_insert(index);
        true
    }

    /// Apply a pointer or keyboard selection request for one row.
    pub fn select(
        &mut self,
        index: usize,
        total_items: usize,
        modifiers: ListSelectionModifiers,
    ) -> bool {
        if index >= total_items {
            return false;
        }
        self.focused_index = Some(index);
        if modifiers.extend {
            let anchor = self.anchor_index.unwrap_or(index).min(total_items - 1);
            self.anchor_index = Some(anchor);
            self.select_range(anchor, index, false);
        } else if modifiers.toggle {
            self.anchor_index = Some(index);
            self.toggle_index(index);
        } else {
            self.anchor_index = Some(index);
            self.replace_selection(vec![index]);
        }
        true
    }

    /// Apply a high-level pointer or keyboard selection intent for one row.
    pub fn select_with_intent(
        &mut self,
        index: usize,
        total_items: usize,
        intent: ListSelectionIntent,
    ) -> bool {
        match intent {
            ListSelectionIntent::ExtendPreservingExisting => {
                self.extend_preserving_existing(index, total_items)
            }
            _ => self.select(index, total_items, intent.modifiers()),
        }
    }

    /// Extend selection from the current anchor while preserving existing membership.
    pub fn extend_preserving_existing(&mut self, index: usize, total_items: usize) -> bool {
        if index >= total_items {
            return false;
        }
        self.focused_index = Some(index);
        let anchor = self.anchor_index.unwrap_or(index).min(total_items - 1);
        self.anchor_index = Some(anchor);
        self.select_range(anchor, index, true);
        true
    }

    /// Select every row in the current item range.
    pub fn select_all(&mut self, total_items: usize) {
        let selected = (0..total_items).collect::<Vec<_>>();
        if self.focused_index.is_none() && total_items > 0 {
            self.focused_index = Some(0);
        }
        self.anchor_index = self.focused_index;
        self.replace_selection(selected);
    }

    fn select_range(&mut self, anchor: usize, index: usize, preserve_existing: bool) {
        let start = anchor.min(index);
        let end = anchor.max(index);
        if preserve_existing {
            let mut selected = self.selected_indices.clone();
            selected.extend(start..=end);
            self.replace_selection(selected);
        } else {
            self.replace_selection((start..=end).collect());
        }
    }

    fn toggle_index(&mut self, index: usize) {
        match self.selected_indices.binary_search(&index) {
            Ok(position) => {
                self.selected_indices.remove(position);
                self.bump_revision();
            }
            Err(position) => {
                self.selected_indices.insert(position, index);
                self.bump_revision();
            }
        }
    }

    fn replace_selection(&mut self, mut selected: Vec<usize>) {
        selected.sort_unstable();
        selected.dedup();
        if self.selected_indices != selected {
            self.selected_indices = selected;
            self.bump_revision();
        }
    }

    fn bump_revision(&mut self) {
        self.revision = self.revision.wrapping_add(1);
    }
}

/// Reusable focus, anchor, and multi-selection state for lists keyed by stable row identity.
///
/// Hosts pass the current ordered visible keys into operations that need list
/// order, while this controller keeps durable selection identity in generic
/// keys such as paths, database ids, document ids, or item keys.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct KeyedListSelection<K> {
    focused_key: Option<K>,
    anchor_key: Option<K>,
    selected_keys: SelectionSet<K>,
    revision: u64,
}

impl<K> KeyedListSelection<K> {
    /// Build an empty keyed list selection.
    pub const fn new() -> Self {
        Self {
            focused_key: None,
            anchor_key: None,
            selected_keys: SelectionSet::new(),
            revision: 0,
        }
    }

    /// Return the focused row key, if any.
    pub fn focused_key(&self) -> Option<&K> {
        self.focused_key.as_ref()
    }

    /// Return the range-selection anchor key, if any.
    pub fn anchor_key(&self) -> Option<&K> {
        self.anchor_key.as_ref()
    }

    /// Return sorted selected row keys.
    pub fn selected_keys(&self) -> &[K] {
        self.selected_keys.as_slice()
    }

    /// Return the selected row count.
    pub fn selected_count(&self) -> usize {
        self.selected_keys.len()
    }

    /// Return whether the keyed selection is empty.
    pub fn is_empty(&self) -> bool {
        self.selected_keys.is_empty()
    }

    /// Return a monotonic revision bumped when selection membership changes.
    pub const fn revision(&self) -> u64 {
        self.revision
    }
}

impl<K> KeyedListSelection<K>
where
    K: Ord,
{
    /// Return whether `key` is currently selected.
    pub fn is_selected(&self, key: &K) -> bool {
        self.selected_keys.contains(key)
    }
}

impl<K> KeyedListSelection<K>
where
    K: Clone + Ord,
{
    /// Build a keyed list selection from host-owned selection parts.
    pub fn from_parts(
        focused_key: Option<K>,
        anchor_key: Option<K>,
        selected_keys: impl IntoIterator<Item = K>,
    ) -> Self {
        Self {
            focused_key,
            anchor_key,
            selected_keys: SelectionSet::from_items(selected_keys),
            revision: 0,
        }
    }

    /// Clear focus, anchor, and selected membership.
    pub fn clear(&mut self) {
        self.focused_key = None;
        self.anchor_key = None;
        if self.selected_keys.clear() {
            self.bump_revision();
        }
    }

    /// Remove selected keys that are not present in `ordered_keys`.
    pub fn retain_visible(&mut self, ordered_keys: &[K]) {
        let selected = self
            .selected_keys
            .as_slice()
            .iter()
            .filter(|key| ordered_keys.contains(key))
            .cloned()
            .collect::<Vec<_>>();
        if self.selected_keys.replace_items(selected) {
            self.bump_revision();
        }
        if self
            .focused_key
            .as_ref()
            .is_some_and(|key| !ordered_keys.contains(key))
        {
            self.focused_key = None;
        }
        if self
            .anchor_key
            .as_ref()
            .is_some_and(|key| !ordered_keys.contains(key))
        {
            self.anchor_key = self.focused_key.clone();
        }
    }

    /// Move focus without changing selection membership.
    pub fn focus(&mut self, key: K, ordered_keys: &[K]) -> bool {
        if !ordered_keys.contains(&key) {
            return false;
        }
        self.focused_key = Some(key.clone());
        self.anchor_key.get_or_insert(key);
        true
    }

    /// Move focus by `delta`, optionally extending selection.
    pub fn navigate(&mut self, delta: isize, ordered_keys: &[K], extend: bool) -> Option<K> {
        self.navigate_with(delta, ordered_keys, extend, false)
    }

    /// Move focus by `delta`, extending selection while preserving existing membership.
    pub fn navigate_preserving_existing(&mut self, delta: isize, ordered_keys: &[K]) -> Option<K> {
        self.navigate_with(delta, ordered_keys, true, true)
    }

    fn navigate_with(
        &mut self,
        delta: isize,
        ordered_keys: &[K],
        extend: bool,
        preserve_existing: bool,
    ) -> Option<K> {
        let current = self.focused_key.as_ref()?;
        let current_index = ordered_keys.iter().position(|key| key == current)?;
        let target_index = list_index_after_delta(current_index, delta, ordered_keys.len())?;
        if target_index == current_index {
            return None;
        }
        let target = ordered_keys[target_index].clone();
        let modifiers = if extend {
            ListSelectionModifiers::extend()
        } else {
            ListSelectionModifiers::new()
        };
        if preserve_existing {
            self.extend_preserving_existing(target.clone(), ordered_keys);
        } else {
            self.select(target.clone(), ordered_keys, modifiers);
        }
        Some(target)
    }

    /// Apply a pointer or keyboard selection request for one keyed row.
    pub fn select(
        &mut self,
        key: K,
        ordered_keys: &[K],
        modifiers: ListSelectionModifiers,
    ) -> bool {
        if !ordered_keys.contains(&key) {
            return false;
        }
        self.focused_key = Some(key.clone());
        if modifiers.extend {
            let anchor = self
                .anchor_key
                .as_ref()
                .filter(|anchor| ordered_keys.contains(anchor))
                .cloned()
                .unwrap_or_else(|| key.clone());
            self.anchor_key = Some(anchor.clone());
            self.select_range(&anchor, &key, ordered_keys, false);
        } else if modifiers.toggle {
            self.anchor_key = Some(key.clone());
            self.toggle_key(key);
        } else {
            self.anchor_key = Some(key.clone());
            self.replace_selection([key]);
        }
        true
    }

    /// Apply a high-level pointer or keyboard selection intent for one keyed row.
    pub fn select_with_intent(
        &mut self,
        key: K,
        ordered_keys: &[K],
        intent: ListSelectionIntent,
    ) -> bool {
        match intent {
            ListSelectionIntent::ExtendPreservingExisting => {
                self.extend_preserving_existing(key, ordered_keys)
            }
            _ => self.select(key, ordered_keys, intent.modifiers()),
        }
    }

    /// Extend selection from the current anchor while preserving existing membership.
    pub fn extend_preserving_existing(&mut self, key: K, ordered_keys: &[K]) -> bool {
        if !ordered_keys.contains(&key) {
            return false;
        }
        self.focused_key = Some(key.clone());
        let anchor = self
            .anchor_key
            .as_ref()
            .filter(|anchor| ordered_keys.contains(anchor))
            .cloned()
            .unwrap_or_else(|| key.clone());
        self.anchor_key = Some(anchor.clone());
        self.select_range(&anchor, &key, ordered_keys, true);
        true
    }

    /// Select every key in the current ordered list.
    pub fn select_all(&mut self, ordered_keys: &[K]) {
        if self.focused_key.is_none()
            && let Some(first) = ordered_keys.first()
        {
            self.focused_key = Some(first.clone());
        }
        self.anchor_key = self.focused_key.clone();
        self.replace_selection(ordered_keys.iter().cloned());
    }

    fn select_range(&mut self, anchor: &K, key: &K, ordered_keys: &[K], preserve_existing: bool) {
        let Some(anchor_index) = ordered_keys
            .iter()
            .position(|candidate| candidate == anchor)
        else {
            return;
        };
        let Some(key_index) = ordered_keys.iter().position(|candidate| candidate == key) else {
            return;
        };
        let start = anchor_index.min(key_index);
        let end = anchor_index.max(key_index);
        let range = ordered_keys[start..=end].to_vec();
        if preserve_existing {
            if self.selected_keys.extend_items(range) {
                self.bump_revision();
            }
        } else {
            self.replace_selection(range);
        }
    }

    fn toggle_key(&mut self, key: K) {
        let changed = if self.selected_keys.contains(&key) && self.selected_keys.len() > 1 {
            self.selected_keys.remove(&key)
        } else {
            self.selected_keys.insert(key)
        };
        if changed {
            self.bump_revision();
        }
    }

    fn replace_selection(&mut self, selected: impl IntoIterator<Item = K>) {
        if self.selected_keys.replace_items(selected) {
            self.bump_revision();
        }
    }

    fn bump_revision(&mut self) {
        self.revision = self.revision.wrapping_add(1);
    }
}
