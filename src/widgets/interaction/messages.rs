use crate::{gui::types::Point, layout::Vector2};

use super::{PointerButton, PointerModifiers, WidgetInput};

/// Message emitted by a reusable button primitive.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ButtonMessage {
    /// The button was activated by pointer or keyboard input.
    Activate,
    /// The button received a secondary/right pointer click.
    SecondaryActivate {
        /// Pointer position where the secondary activation occurred.
        position: Point,
    },
    /// The button is being used as a primary-pointer drag surface.
    Drag(DragHandleMessage),
}

impl ButtonMessage {
    /// Return whether this message is a primary button activation.
    pub fn is_activate(self) -> bool {
        matches!(self, Self::Activate)
    }

    /// Return the secondary/right-click activation position, when present.
    pub fn secondary_position(self) -> Option<Point> {
        match self {
            Self::SecondaryActivate { position } => Some(position),
            _ => None,
        }
    }

    /// Return the drag lifecycle message, when present.
    pub fn drag_message(self) -> Option<DragHandleMessage> {
        match self {
            Self::Drag(message) => Some(message),
            _ => None,
        }
    }
}

/// Message emitted by a reusable badge or pill primitive.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum BadgeMessage {
    /// The badge was activated by pointer or keyboard input.
    Activate,
}

/// Message emitted by a reusable list-item primitive.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ListItemMessage {
    /// The item was invoked by pointer or keyboard input.
    Invoked,
}

/// Message emitted by a reusable interactive row primitive.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum InteractiveRowMessage {
    /// The row was activated by pointer or keyboard input.
    Activate,
    /// The row was activated by primary pointer input with modifier state.
    ActivateWithModifiers {
        /// Modifier state at release time.
        modifiers: PointerModifiers,
    },
    /// The row received a primary-button double activation.
    DoubleActivate,
    /// The row received a secondary/right pointer click.
    SecondaryActivate {
        /// Pointer position where the secondary activation occurred.
        position: Point,
    },
    /// The row is being used as a primary-pointer drag surface.
    Drag(DragHandleMessage),
    /// A primary-pointer drop landed inside this row.
    Drop,
    /// The row was hovered while another row drag was active.
    HoverDropTarget {
        /// Pointer position where the drop target hover occurred.
        position: Point,
    },
}

impl InteractiveRowMessage {
    /// Return modifier state when this message is an activation.
    ///
    /// Plain activation and double activation use default modifiers. This is
    /// useful for custom-painted row widgets that map Radiant's generic row
    /// interaction model into host-specific row actions.
    pub fn activation_modifiers(self) -> Option<PointerModifiers> {
        match self {
            Self::Activate | Self::DoubleActivate => Some(PointerModifiers::default()),
            Self::ActivateWithModifiers { modifiers } => Some(modifiers),
            _ => None,
        }
    }

    /// Return whether this message is any primary activation.
    pub fn is_activation(self) -> bool {
        self.activation_modifiers().is_some()
    }

    /// Return modifier state when this message is a single primary activation.
    ///
    /// This excludes double activation so applications can route ordinary row
    /// invocation separately from double-click actions such as rename, drill-in,
    /// or open-in-place flows.
    pub fn single_activation_modifiers(self) -> Option<PointerModifiers> {
        match self {
            Self::Activate => Some(PointerModifiers::default()),
            Self::ActivateWithModifiers { modifiers } => Some(modifiers),
            _ => None,
        }
    }

    /// Return whether this message is a single primary activation.
    pub fn is_single_activation(self) -> bool {
        self.single_activation_modifiers().is_some()
    }

    /// Return whether this message is a primary double activation.
    pub fn is_double_activation(self) -> bool {
        matches!(self, Self::DoubleActivate)
    }

    /// Return the secondary/right-click activation position, when present.
    pub fn secondary_position(self) -> Option<Point> {
        match self {
            Self::SecondaryActivate { position } => Some(position),
            _ => None,
        }
    }

    /// Return the drag lifecycle message, when present.
    pub fn drag_message(self) -> Option<DragHandleMessage> {
        match self {
            Self::Drag(message) => Some(message),
            _ => None,
        }
    }

    /// Return the drop-hover position, when present.
    pub fn hover_drop_position(self) -> Option<Point> {
        match self {
            Self::HoverDropTarget { position } => Some(position),
            _ => None,
        }
    }

    /// Return whether this message is a completed drop.
    pub fn is_drop(self) -> bool {
        matches!(self, Self::Drop)
    }
}

/// Message emitted by a transparent pointer interception primitive.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PointerShieldMessage {
    /// Pointer moved inside the shield.
    PointerMove {
        /// Pointer position in the widget host's logical coordinate space.
        position: Point,
    },
    /// Pointer press landed inside the shield.
    PointerPress {
        /// Pointer position in the widget host's logical coordinate space.
        position: Point,
        /// Button that started the press.
        button: PointerButton,
        /// Modifier state at press time.
        modifiers: PointerModifiers,
    },
    /// Pointer release landed inside the shield.
    PointerRelease {
        /// Pointer position in the widget host's logical coordinate space.
        position: Point,
        /// Button that ended the press.
        button: PointerButton,
        /// Modifier state at release time.
        modifiers: PointerModifiers,
    },
    /// Captured pointer release landed inside the shield.
    PointerDrop {
        /// Pointer position in the widget host's logical coordinate space.
        position: Point,
        /// Button that ended the captured press.
        button: PointerButton,
        /// Modifier state at release time.
        modifiers: PointerModifiers,
    },
    /// Wheel input landed inside the shield.
    Wheel {
        /// Pointer position in the widget host's logical coordinate space.
        position: Point,
        /// Wheel delta in logical units.
        delta: Vector2,
        /// Modifier state at wheel time.
        modifiers: PointerModifiers,
    },
}

/// Message emitted by a reusable selectable primitive.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SelectableMessage {
    /// Selection state changed to the provided value.
    SelectionChanged {
        /// New selected value after the interaction completed.
        selected: bool,
    },
}

/// Message emitted by a reusable toggle primitive.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ToggleMessage {
    /// The toggle value changed to the provided checked state.
    ValueChanged {
        /// New boolean value after the interaction completed.
        checked: bool,
    },
}

/// Message emitted by a reusable text-input primitive.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum TextInputMessage {
    /// The visible text value changed immediately.
    Changed {
        /// Updated single-line text value.
        value: String,
    },
    /// The current text value was committed by submit intent.
    Submitted {
        /// Submitted single-line text value.
        value: String,
    },
    /// The current text value requested host-defined completion.
    CompletionRequested {
        /// Current single-line text value at completion time.
        value: String,
    },
}

/// High-level kind for a reusable text-input message.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TextInputMessageKind {
    /// The visible text value changed immediately.
    Changed,
    /// The current text value was committed by submit intent.
    Submitted,
    /// The current text value requested host-defined completion.
    CompletionRequested,
}

/// Borrowed parts of a text-input message.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct TextInputMessageParts<'a> {
    /// High-level message kind.
    pub kind: TextInputMessageKind,
    /// Text value carried by the message.
    pub value: &'a str,
}

impl TextInputMessage {
    /// Return the high-level kind of this input event.
    pub fn kind(&self) -> TextInputMessageKind {
        match self {
            Self::Changed { .. } => TextInputMessageKind::Changed,
            Self::Submitted { .. } => TextInputMessageKind::Submitted,
            Self::CompletionRequested { .. } => TextInputMessageKind::CompletionRequested,
        }
    }

    /// Return borrowed parts for reducers that need both kind and value.
    pub fn parts(&self) -> TextInputMessageParts<'_> {
        TextInputMessageParts {
            kind: self.kind(),
            value: self.value(),
        }
    }

    /// Return the text value carried by this input event.
    pub fn value(&self) -> &str {
        match self {
            Self::Changed { value }
            | Self::Submitted { value }
            | Self::CompletionRequested { value } => value.as_str(),
        }
    }

    /// Consume this input event and return its text value.
    pub fn into_value(self) -> String {
        match self {
            Self::Changed { value }
            | Self::Submitted { value }
            | Self::CompletionRequested { value } => value,
        }
    }

    /// Return whether this event is an immediate edit.
    pub fn is_changed(&self) -> bool {
        matches!(self, Self::Changed { .. })
    }

    /// Return whether this event is a submit/commit intent.
    pub fn is_submitted(&self) -> bool {
        matches!(self, Self::Submitted { .. })
    }

    /// Return whether this event requests host-defined completion.
    pub fn is_completion_requested(&self) -> bool {
        matches!(self, Self::CompletionRequested { .. })
    }
}

#[cfg(test)]
mod text_input_message_tests {
    use super::{TextInputMessage, TextInputMessageKind};

    #[test]
    fn text_input_message_parts_expose_kind_and_borrowed_value() {
        let message = TextInputMessage::Submitted {
            value: String::from("crate"),
        };

        let parts = message.parts();

        assert_eq!(parts.kind, TextInputMessageKind::Submitted);
        assert_eq!(parts.value, "crate");
        assert_eq!(message.value(), "crate");
    }

    #[test]
    fn text_input_message_kind_classifies_each_variant() {
        assert_eq!(
            TextInputMessage::Changed {
                value: String::from("a")
            }
            .kind(),
            TextInputMessageKind::Changed
        );
        assert_eq!(
            TextInputMessage::CompletionRequested {
                value: String::from("ab")
            }
            .kind(),
            TextInputMessageKind::CompletionRequested
        );
    }
}

#[cfg(test)]
mod drag_handle_phase_tests {
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
        let move_to = Point::new(3.0, 4.0);
        let end = Point::new(5.0, 6.0);
        let double = Point::new(7.0, 8.0);
        let cancel = Point::new(9.0, 10.0);

        assert_eq!(
            DragHandleMessage::started(start),
            DragHandleMessage::Started { position: start }
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
        assert!(DragHandleMessage::moved(move_to).is_moved());
        assert!(DragHandleMessage::ended(end).is_ended());
        assert!(DragHandleMessage::double_activate(double).is_double_activate());
        assert!(DragHandleMessage::cancelled(cancel).is_cancelled());
    }
}

/// Message emitted by a reusable scrollbar primitive.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ScrollbarMessage {
    /// The viewport offset changed to the provided normalized fraction.
    OffsetChanged {
        /// Clamped normalized viewport start in the inclusive range `0.0..=1.0`.
        offset_fraction: f32,
    },
}

/// Message emitted by a reusable slider primitive.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SliderMessage {
    /// The normalized slider value changed.
    ValueChanged {
        /// Clamped normalized value in the inclusive range `0.0..=1.0`.
        value: f32,
    },
}

/// Message emitted by a reusable drag handle primitive.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum DragHandleMessage {
    /// Primary pointer drag started.
    Started {
        /// Pointer position in the widget host's logical coordinate space.
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
        Self::Started { position }
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
            Self::Started { position }
            | Self::Moved { position }
            | Self::Ended { position }
            | Self::DoubleActivate { position }
            | Self::Cancelled { position } => position,
        }
    }

    /// Return the pointer position when this message starts an interaction.
    pub fn started_position(self) -> Option<Point> {
        match self {
            Self::Started { position } => Some(position),
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

/// Message emitted by a reusable canvas/custom-paint primitive.
#[derive(Clone, Debug, PartialEq)]
pub enum CanvasMessage {
    /// Backend-neutral interaction routed into the custom surface.
    Input {
        /// Routed widget input payload.
        input: WidgetInput,
    },
}

/// Message emitted by a retained GPU surface primitive.
#[derive(Clone, Debug, PartialEq)]
pub enum GpuSurfaceMessage {
    /// Backend-neutral interaction routed into the GPU surface widget.
    Input {
        /// Routed widget input payload.
        input: WidgetInput,
    },
}
