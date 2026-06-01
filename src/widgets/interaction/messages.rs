use crate::gui::types::Point;

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

impl TextInputMessage {
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
}

impl DragHandleMessage {
    /// Return this drag message's lifecycle phase.
    pub fn phase(self) -> DragHandlePhase {
        match self {
            Self::Started { .. } => DragHandlePhase::Started,
            Self::Moved { .. } => DragHandlePhase::Moved,
            Self::Ended { .. } => DragHandlePhase::Ended,
        }
    }

    /// Return this drag message's pointer position.
    pub fn position(self) -> Point {
        match self {
            Self::Started { position } | Self::Moved { position } | Self::Ended { position } => {
                position
            }
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
