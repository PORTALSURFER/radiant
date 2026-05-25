//! Backend-neutral pointer cursor requests.

/// Pointer cursor shape requested by a widget for a specific hit-tested point.
///
/// The runtime treats this as a semantic request, not a backend type. Native
/// adapters map it to the closest platform cursor available.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum WidgetCursor {
    /// Platform default cursor.
    #[default]
    Default,
    /// Link or clickable hand cursor.
    Pointer,
    /// Text insertion or selection cursor.
    Text,
    /// Crosshair cursor.
    Crosshair,
    /// Open-hand grab cursor.
    Grab,
    /// Closed-hand grabbing cursor.
    Grabbing,
    /// Move cursor.
    Move,
    /// Horizontal resize cursor where either side may move.
    ResizeHorizontal,
    /// Resize cursor for a left edge.
    ResizeLeft,
    /// Resize cursor for a right edge.
    ResizeRight,
    /// Vertical resize cursor where either side may move.
    ResizeVertical,
    /// Operation is unavailable at this point.
    NotAllowed,
}
