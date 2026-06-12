/// Pointer button routed into a widget.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PointerButton {
    /// Primary/left pointer button.
    Primary,
    /// Secondary/right pointer button.
    Secondary,
    /// Auxiliary or middle pointer button.
    Auxiliary,
}

/// Modifier state captured with one pointer interaction.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PointerModifiers {
    /// Whether the platform command modifier is held.
    pub command: bool,
    /// Whether Shift is held.
    pub shift: bool,
    /// Whether Alt is held.
    pub alt: bool,
}
