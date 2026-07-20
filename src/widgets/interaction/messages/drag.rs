use crate::gui::types::Point;

/// Message emitted by a reusable drag handle primitive.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum DragHandleMessage {
    /// Primary pointer drag started.
    Started {
        /// Pointer position where the primary press began.
        origin: Point,
        /// Current pointer position when the drag crossed its start threshold.
        position: Point,
    },
    /// Captured pointer moved while the drag is active.
    Moved {
        /// Pointer position in the widget host's logical coordinate space.
        position: Point,
    },
    /// Primary pointer drag ended or was released.
    Ended {
        /// Pointer position in the widget host's logical coordinate space.
        position: Point,
    },
    /// Primary pointer double-clicked the drag handle.
    DoubleActivate {
        /// Pointer position in the widget host's logical coordinate space.
        position: Point,
    },
    /// Active drag was cancelled before a normal release.
    Cancelled {
        /// Last known pointer position in the widget host's logical coordinate space.
        position: Point,
    },
}

/// Lifecycle phase for a reusable drag-handle interaction.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum DragHandlePhase {
    /// Primary pointer drag started.
    Started,
    /// Captured pointer moved while the drag is active.
    Moved,
    /// Primary pointer drag ended or was released.
    Ended,
    /// Primary pointer double-clicked the drag handle.
    DoubleActivate,
    /// Active drag was cancelled before a normal release.
    Cancelled,
}

impl DragHandlePhase {
    /// Return a stable lowercase diagnostic label for this drag phase.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Started => "started",
            Self::Moved => "moved",
            Self::Ended => "ended",
            Self::DoubleActivate => "double_activate",
            Self::Cancelled => "cancelled",
        }
    }
}

impl DragHandleMessage {
    /// Build a drag-start message at `position`.
    pub const fn started(position: Point) -> Self {
        Self::Started {
            origin: position,
            position,
        }
    }

    /// Build a threshold-crossing drag start from `origin` to `position`.
    pub const fn started_from(origin: Point, position: Point) -> Self {
        Self::Started { origin, position }
    }

    /// Build an active drag-motion message at `position`.
    pub const fn moved(position: Point) -> Self {
        Self::Moved { position }
    }

    /// Build a drag-ended message at `position`.
    pub const fn ended(position: Point) -> Self {
        Self::Ended { position }
    }

    /// Build a double-activation message at `position`.
    pub const fn double_activate(position: Point) -> Self {
        Self::DoubleActivate { position }
    }

    /// Build a drag-cancelled message at `position`.
    pub const fn cancelled(position: Point) -> Self {
        Self::Cancelled { position }
    }

    /// Return this drag message's lifecycle phase.
    pub fn phase(self) -> DragHandlePhase {
        match self {
            Self::Started { .. } => DragHandlePhase::Started,
            Self::Moved { .. } => DragHandlePhase::Moved,
            Self::Ended { .. } => DragHandlePhase::Ended,
            Self::DoubleActivate { .. } => DragHandlePhase::DoubleActivate,
            Self::Cancelled { .. } => DragHandlePhase::Cancelled,
        }
    }

    /// Return this drag message's pointer position.
    pub fn position(self) -> Point {
        match self {
            Self::Started { position, .. }
            | Self::Moved { position }
            | Self::Ended { position }
            | Self::DoubleActivate { position }
            | Self::Cancelled { position } => position,
        }
    }

    /// Return the pointer position when this message starts an interaction.
    pub fn started_position(self) -> Option<Point> {
        match self {
            Self::Started { position, .. } => Some(position),
            _ => None,
        }
    }

    /// Return the primary-press origin when this message starts an interaction.
    pub fn started_origin(self) -> Option<Point> {
        match self {
            Self::Started { origin, .. } => Some(origin),
            _ => None,
        }
    }

    /// Return the pointer position when this message is active drag motion.
    pub fn moved_position(self) -> Option<Point> {
        match self {
            Self::Moved { position } => Some(position),
            _ => None,
        }
    }

    /// Return the pointer position when this message ends an interaction.
    pub fn ended_position(self) -> Option<Point> {
        match self {
            Self::Ended { position } => Some(position),
            _ => None,
        }
    }

    /// Return the pointer position when this message ends or cancels an interaction.
    pub fn finished_position(self) -> Option<Point> {
        match self {
            Self::Ended { position } | Self::Cancelled { position } => Some(position),
            _ => None,
        }
    }

    /// Return whether this drag message starts an interaction.
    pub fn is_started(self) -> bool {
        matches!(self.phase(), DragHandlePhase::Started)
    }

    /// Return whether this drag message is active drag motion.
    pub fn is_moved(self) -> bool {
        matches!(self.phase(), DragHandlePhase::Moved)
    }

    /// Return whether this drag message ends an interaction.
    pub fn is_ended(self) -> bool {
        matches!(self.phase(), DragHandlePhase::Ended)
    }

    /// Return whether this drag message ends or cancels an interaction.
    pub fn is_finished(self) -> bool {
        matches!(
            self.phase(),
            DragHandlePhase::Ended | DragHandlePhase::Cancelled
        )
    }

    /// Return whether this drag handle received a primary double activation.
    pub fn is_double_activate(self) -> bool {
        matches!(self.phase(), DragHandlePhase::DoubleActivate)
    }

    /// Return whether this drag was cancelled before release.
    pub fn is_cancelled(self) -> bool {
        matches!(self.phase(), DragHandlePhase::Cancelled)
    }
}

#[cfg(test)]
mod tests {
    use crate::gui::types::Point;

    use super::{DragHandleMessage, DragHandlePhase};

    #[test]
    fn drag_handle_phase_exposes_stable_diagnostic_labels() {
        assert_eq!(DragHandlePhase::Started.as_str(), "started");
        assert_eq!(DragHandlePhase::Moved.as_str(), "moved");
        assert_eq!(DragHandlePhase::Ended.as_str(), "ended");
        assert_eq!(DragHandlePhase::DoubleActivate.as_str(), "double_activate");
        assert_eq!(DragHandlePhase::Cancelled.as_str(), "cancelled");
    }

    #[test]
    fn drag_handle_message_classifies_terminal_drag_phases() {
        let ended = DragHandleMessage::Ended {
            position: Point::new(10.0, 20.0),
        };
        let cancelled = DragHandleMessage::Cancelled {
            position: Point::new(30.0, 40.0),
        };
        let moved = DragHandleMessage::Moved {
            position: Point::new(50.0, 60.0),
        };

        assert!(ended.is_finished());
        assert!(cancelled.is_finished());
        assert!(!moved.is_finished());
        assert_eq!(ended.finished_position(), Some(Point::new(10.0, 20.0)));
        assert_eq!(cancelled.finished_position(), Some(Point::new(30.0, 40.0)));
        assert_eq!(moved.finished_position(), None);
    }

    #[test]
    fn drag_handle_message_constructors_preserve_variant_semantics() {
        let start = Point::new(1.0, 2.0);
        let threshold = Point::new(2.0, 3.0);
        let move_to = Point::new(3.0, 4.0);
        let end = Point::new(5.0, 6.0);
        let double = Point::new(7.0, 8.0);
        let cancel = Point::new(9.0, 10.0);

        assert_eq!(
            DragHandleMessage::started(start),
            DragHandleMessage::Started {
                origin: start,
                position: start,
            }
        );
        assert_eq!(
            DragHandleMessage::started_from(start, threshold),
            DragHandleMessage::Started {
                origin: start,
                position: threshold,
            }
        );
        assert_eq!(
            DragHandleMessage::moved(move_to),
            DragHandleMessage::Moved { position: move_to }
        );
        assert_eq!(
            DragHandleMessage::ended(end),
            DragHandleMessage::Ended { position: end }
        );
        assert_eq!(
            DragHandleMessage::double_activate(double),
            DragHandleMessage::DoubleActivate { position: double }
        );
        assert_eq!(
            DragHandleMessage::cancelled(cancel),
            DragHandleMessage::Cancelled { position: cancel }
        );
        assert!(DragHandleMessage::started(start).is_started());
        assert_eq!(
            DragHandleMessage::started_from(start, threshold).started_origin(),
            Some(start)
        );
        assert_eq!(
            DragHandleMessage::started_from(start, threshold).started_position(),
            Some(threshold)
        );
        assert!(DragHandleMessage::moved(move_to).is_moved());
        assert!(DragHandleMessage::ended(end).is_ended());
        assert!(DragHandleMessage::double_activate(double).is_double_activate());
        assert!(DragHandleMessage::cancelled(cancel).is_cancelled());
    }
}
