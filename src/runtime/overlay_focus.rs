//! Declarative and raw-source policy evidence for transient overlay focus.

use std::{fmt, rc::Rc};

/// Focus behavior declared by one qualified transient overlay.
///
/// This is policy evidence only. The runtime applies activation, trapping, and
/// restoration after it has qualified the overlay source during projection.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum OverlayFocusPolicy {
    /// Do not give the overlay any focus authority.
    #[default]
    None,
    /// Restore a qualified prior focus owner when this overlay closes without
    /// changing ordinary traversal while it remains open.
    Restore,
    /// Activate focus within the overlay, trap traversal there, and restore a
    /// qualified prior focus owner when it closes.
    Modal,
}

/// Opaque UI-local continuity identity for one raw runtime overlay declaration.
///
/// Retain and reuse this owner across compatible raw surface projections. It
/// retains no widget, runtime, callback, or source-tree state.
#[derive(Clone)]
pub struct OverlayFocusOwner(Rc<()>);

impl OverlayFocusOwner {
    /// Allocate a fresh raw-overlay continuity identity.
    pub fn new() -> Self {
        Self(Rc::new(()))
    }
}

impl Default for OverlayFocusOwner {
    fn default() -> Self {
        Self::new()
    }
}

impl PartialEq for OverlayFocusOwner {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

impl Eq for OverlayFocusOwner {}

impl fmt::Debug for OverlayFocusOwner {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("OverlayFocusOwner")
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct OverlayFocusMarker {
    pub(crate) owner: OverlayFocusOwner,
    pub(crate) policy: OverlayFocusPolicy,
}
