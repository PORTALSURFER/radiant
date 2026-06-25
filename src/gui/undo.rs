use super::input::{KeyCode, KeyPress};

const DEFAULT_UNDO_CAPACITY: usize = 128;

/// Standard undo or redo intent resolved from application input.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UndoRedoIntent {
    /// Restore the previous registered state.
    Undo,
    /// Restore the next state previously produced by undo.
    Redo,
}

impl UndoRedoIntent {
    /// Resolve conventional platform undo/redo shortcuts.
    ///
    /// Radiant treats the normalized command modifier as Ctrl on Windows/Linux
    /// and Command on macOS. Redo accepts both Command+Y and Command+Shift+Z.
    pub fn from_key_press(press: KeyPress) -> Option<Self> {
        if !press.command || press.alt {
            return None;
        }
        match (press.key, press.shift) {
            (KeyCode::Z, false) => Some(Self::Undo),
            (KeyCode::Z, true) | (KeyCode::Y, _) => Some(Self::Redo),
            _ => None,
        }
    }
}

/// One state snapshot registered with an undo history.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UndoCheckpoint<State> {
    /// User-facing action label associated with the checkpoint.
    pub label: String,
    /// Optional key used to coalesce repeated edits from one continuous gesture.
    pub merge_key: Option<String>,
    /// State to restore when this checkpoint is applied.
    pub state: State,
}

impl<State> UndoCheckpoint<State> {
    /// Build a checkpoint from a label and restorable state.
    pub fn new(label: impl Into<String>, state: State) -> Self {
        Self {
            label: label.into(),
            merge_key: None,
            state,
        }
    }

    /// Attach a merge key for coalescing repeated gesture updates.
    pub fn with_merge_key(mut self, merge_key: impl Into<String>) -> Self {
        self.merge_key = Some(merge_key.into());
        self
    }
}

/// Result of applying an undo or redo operation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UndoTransition<State> {
    /// Label of the action that was undone or redone.
    pub label: String,
    /// State restored by the operation.
    pub state: State,
}

/// Bounded snapshot history for application-owned GUI state.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UndoHistory<State> {
    undo: Vec<UndoCheckpoint<State>>,
    redo: Vec<UndoCheckpoint<State>>,
    capacity: usize,
}

impl<State> Default for UndoHistory<State> {
    fn default() -> Self {
        Self::new()
    }
}

impl<State> UndoHistory<State> {
    /// Build an empty history with Radiant's default capacity.
    pub const fn new() -> Self {
        Self {
            undo: Vec::new(),
            redo: Vec::new(),
            capacity: DEFAULT_UNDO_CAPACITY,
        }
    }

    /// Build an empty history with a caller-defined maximum undo depth.
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            capacity: capacity.max(1),
            ..Self::new()
        }
    }

    /// Return whether an undo operation is available.
    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }

    /// Return whether a redo operation is available.
    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }

    /// Return the number of undo checkpoints currently retained.
    pub fn undo_len(&self) -> usize {
        self.undo.len()
    }

    /// Return the number of redo checkpoints currently retained.
    pub fn redo_len(&self) -> usize {
        self.redo.len()
    }

    /// Return whether a new change with `merge_key` can merge into the latest
    /// undo checkpoint without creating a new checkpoint or clearing redo.
    pub fn can_coalesce_change(&self, merge_key: &str) -> bool {
        self.redo.is_empty()
            && self
                .undo
                .last()
                .and_then(|checkpoint| checkpoint.merge_key.as_deref())
                == Some(merge_key)
    }

    /// Remove all undo and redo checkpoints.
    pub fn clear(&mut self) {
        self.undo.clear();
        self.redo.clear();
    }

    /// Register a pre-change state as an undoable action.
    ///
    /// Registering a new action clears redo history, matching standard editor
    /// semantics after a user branches away from an undone state.
    pub fn register(&mut self, label: impl Into<String>, before: State) {
        self.undo.push(UndoCheckpoint::new(label, before));
        self.redo.clear();
        self.enforce_capacity();
    }

    /// Register a checkpoint and coalesce it with the previous checkpoint when
    /// both share the same merge key.
    ///
    /// Coalescing keeps the earliest pre-change snapshot, which makes repeated
    /// live updates from one continuous gesture undo as a single action.
    pub fn register_coalescing(
        &mut self,
        label: impl Into<String>,
        merge_key: impl Into<String>,
        before: State,
    ) {
        let merge_key = merge_key.into();
        if self
            .undo
            .last()
            .and_then(|checkpoint| checkpoint.merge_key.as_deref())
            == Some(merge_key.as_str())
        {
            self.redo.clear();
            return;
        }
        self.undo
            .push(UndoCheckpoint::new(label, before).with_merge_key(merge_key));
        self.redo.clear();
        self.enforce_capacity();
    }

    fn enforce_capacity(&mut self) {
        let overflow = self.undo.len().saturating_sub(self.capacity);
        if overflow > 0 {
            self.undo.drain(0..overflow);
        }
    }
}

impl<State> UndoHistory<State>
where
    State: Clone,
{
    /// Restore the previous checkpoint and retain the current state for redo.
    pub fn undo(&mut self, current: &State) -> Option<UndoTransition<State>> {
        let checkpoint = self.undo.pop()?;
        self.redo.push(UndoCheckpoint {
            label: checkpoint.label.clone(),
            merge_key: checkpoint.merge_key.clone(),
            state: current.clone(),
        });
        Some(UndoTransition {
            label: checkpoint.label,
            state: checkpoint.state,
        })
    }

    /// Restore the next redo checkpoint and retain the current state for undo.
    pub fn redo(&mut self, current: &State) -> Option<UndoTransition<State>> {
        let checkpoint = self.redo.pop()?;
        self.undo.push(UndoCheckpoint {
            label: checkpoint.label.clone(),
            merge_key: checkpoint.merge_key.clone(),
            state: current.clone(),
        });
        Some(UndoTransition {
            label: checkpoint.label,
            state: checkpoint.state,
        })
    }
}

impl<State> UndoHistory<State>
where
    State: Clone + PartialEq,
{
    /// Register a change only when the before and after states differ.
    pub fn register_change(
        &mut self,
        label: impl Into<String>,
        before: State,
        after: &State,
    ) -> bool {
        if before == *after {
            return false;
        }
        self.register(label, before);
        true
    }

    /// Register a changed state with a merge key for repeated live updates.
    pub fn register_change_coalescing(
        &mut self,
        label: impl Into<String>,
        merge_key: impl Into<String>,
        before: State,
        after: &State,
    ) -> bool {
        if before == *after {
            return false;
        }
        self.register_coalescing(label, merge_key, before);
        true
    }

    /// Apply a mutation and register it as one undoable action if it changes state.
    pub fn apply(
        &mut self,
        label: impl Into<String>,
        state: &mut State,
        mutate: impl FnOnce(&mut State),
    ) -> bool {
        let before = state.clone();
        mutate(state);
        self.register_change(label, before, state)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn undo_history_restores_previous_state_and_redoes_current_state() {
        let mut history = UndoHistory::new();
        let mut value = 1;

        assert!(history.apply("increment", &mut value, |value| *value += 1));
        assert_eq!(value, 2);
        assert!(history.can_undo());
        assert!(!history.can_redo());

        let undo = history.undo(&value).expect("undo should be available");
        assert_eq!(undo.label, "increment");
        value = undo.state;
        assert_eq!(value, 1);
        assert!(history.can_redo());

        let redo = history.redo(&value).expect("redo should be available");
        value = redo.state;
        assert_eq!(value, 2);
    }

    #[test]
    fn registering_new_change_clears_redo_branch() {
        let mut history = UndoHistory::new();
        let mut value = 1;

        history.apply("first", &mut value, |value| *value = 2);
        value = history.undo(&value).unwrap().state;
        assert!(history.can_redo());

        history.apply("branch", &mut value, |value| *value = 3);
        assert!(!history.can_redo());
        assert_eq!(history.undo_len(), 1);
    }

    #[test]
    fn unchanged_mutations_are_not_registered() {
        let mut history = UndoHistory::new();
        let mut value = 1;

        assert!(!history.apply("no-op", &mut value, |_| {}));
        assert!(!history.can_undo());
    }

    #[test]
    fn undo_history_enforces_capacity() {
        let mut history = UndoHistory::with_capacity(2);
        let mut value = 0;

        history.apply("one", &mut value, |value| *value = 1);
        history.apply("two", &mut value, |value| *value = 2);
        history.apply("three", &mut value, |value| *value = 3);

        assert_eq!(history.undo_len(), 2);
        assert_eq!(history.undo(&value).unwrap().state, 2);
        assert_eq!(history.undo(&2).unwrap().state, 1);
        assert!(history.undo(&1).is_none());
    }

    #[test]
    fn coalesced_changes_keep_earliest_snapshot() {
        let mut history = UndoHistory::new();
        let mut value = 0;

        assert!(!history.can_coalesce_change("drag:1"));
        let before = value;
        value = 1;
        assert!(history.register_change_coalescing("drag", "drag:1", before, &value));
        assert!(history.can_coalesce_change("drag:1"));
        assert!(!history.can_coalesce_change("drag:2"));
        let before = value;
        value = 2;
        assert!(history.register_change_coalescing("drag", "drag:1", before, &value));

        assert_eq!(history.undo_len(), 1);
        assert_eq!(history.undo(&value).unwrap().state, 0);
        assert!(!history.can_coalesce_change("drag:1"));
    }

    #[test]
    fn undo_redo_intent_resolves_standard_shortcuts() {
        assert_eq!(
            UndoRedoIntent::from_key_press(KeyPress::with_command(KeyCode::Z)),
            Some(UndoRedoIntent::Undo)
        );
        assert_eq!(
            UndoRedoIntent::from_key_press(KeyPress {
                key: KeyCode::Z,
                command: true,
                control: false,
                shift: true,
                alt: false,
            }),
            Some(UndoRedoIntent::Redo)
        );
        assert_eq!(
            UndoRedoIntent::from_key_press(KeyPress::with_command(KeyCode::Y)),
            Some(UndoRedoIntent::Redo)
        );
        assert_eq!(
            UndoRedoIntent::from_key_press(KeyPress::new(KeyCode::Z)),
            None
        );
    }
}
