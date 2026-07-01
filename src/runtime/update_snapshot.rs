use crate::gui::types::Point;

/// Runtime-owned input snapshot attached to one host update.
///
/// The snapshot gives application reducers access to read-only runtime state
/// that belongs to the surface controller, such as the latest pointer position,
/// without forcing applications to mirror that state through their own messages.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct RuntimeUpdateSnapshot {
    current_pointer_position: Option<Point>,
}

impl RuntimeUpdateSnapshot {
    /// Build a snapshot with the latest pointer position known to the runtime.
    pub fn with_current_pointer_position(current_pointer_position: Option<Point>) -> Self {
        Self {
            current_pointer_position,
        }
    }

    /// Latest logical pointer position known to the runtime, in surface space.
    pub fn current_pointer_position(self) -> Option<Point> {
        self.current_pointer_position
    }
}
