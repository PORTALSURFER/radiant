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
