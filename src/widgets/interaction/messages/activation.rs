use super::drag::DragHandleMessage;
use crate::{gui::types::Point, widgets::interaction::input::PointerModifiers};

/// Message emitted by a reusable button primitive.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ButtonMessage {
    /// The button was activated by pointer or keyboard input.
    Activate,
    /// The button was activated by primary pointer input with modifier state.
    ActivateWithModifiers {
        /// Modifier state at release time.
        modifiers: PointerModifiers,
    },
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
        matches!(self, Self::Activate | Self::ActivateWithModifiers { .. })
    }

    /// Return modifier state when this message is an activation.
    ///
    /// Keyboard activation and legacy plain activation use default modifiers.
    pub fn activation_modifiers(self) -> Option<PointerModifiers> {
        match self {
            Self::Activate => Some(PointerModifiers::default()),
            Self::ActivateWithModifiers { modifiers } => Some(modifiers),
            _ => None,
        }
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
    /// The row was hovered while a tracked drop target should be cleared.
    ClearDropTarget {
        /// Pointer position where the stale drop target should be cleared.
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

    /// Return the drop-target clear position, when present.
    pub fn clear_drop_position(self) -> Option<Point> {
        match self {
            Self::ClearDropTarget { position } => Some(position),
            _ => None,
        }
    }

    /// Return whether this message is a completed drop.
    pub fn is_drop(self) -> bool {
        matches!(self, Self::Drop)
    }
}
