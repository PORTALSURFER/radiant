/// Scope of a runtime repaint request.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum RepaintScope {
    /// Repaint after refreshing the declarative surface projection and layout.
    Surface,
    /// Repaint the current paint plan without refreshing declarative state.
    PaintOnly,
}

impl RepaintScope {
    /// Merge two repaint scopes, preferring the scope that preserves correctness.
    ///
    /// A surface repaint includes paint-only work, so any mixed batch must be
    /// treated as a surface repaint.
    pub const fn merge(self, other: Self) -> Self {
        match (self, other) {
            (Self::Surface, _) | (_, Self::Surface) => Self::Surface,
            (Self::PaintOnly, Self::PaintOnly) => Self::PaintOnly,
        }
    }

    /// Return whether this repaint can reuse the current declarative surface.
    pub const fn is_paint_only(self) -> bool {
        matches!(self, Self::PaintOnly)
    }
}
