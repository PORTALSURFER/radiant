//! Declarative traversal policy attached to a view subtree.

/// Sequential behavior when traversal reaches a declared focus scope boundary.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum FocusScopeBoundary {
    /// Return `NoDestination` so the caller can choose an explicit fallback.
    #[default]
    Stop,
    /// Wrap to the opposite end of the same scope.
    Wrap,
}

/// Data-only traversal policy for one declarative subtree.
///
/// The closest declared ancestor governs traversal; scope membership never changes
/// application selection and does not grant offscreen materialization authority.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FocusScope {
    pub(crate) boundary: FocusScopeBoundary,
    pub(crate) spatial: bool,
}
impl FocusScope {
    /// Sequential traversal stopping at the subtree boundary.
    pub const fn sequential() -> Self {
        Self {
            boundary: FocusScopeBoundary::Stop,
            spatial: false,
        }
    }
    /// Sequential and nearest-geometry directional traversal in the subtree.
    pub const fn spatial_grid() -> Self {
        Self {
            boundary: FocusScopeBoundary::Stop,
            spatial: true,
        }
    }
    /// Select whether sequential traversal stops or wraps at the boundary.
    pub const fn boundary(mut self, boundary: FocusScopeBoundary) -> Self {
        self.boundary = boundary;
        self
    }
}
