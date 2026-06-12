use crate::{
    gui::types::{Point, Rect, Vector2},
    widgets::interaction::input::{PointerButton, PointerModifiers, TextEditCommand, WidgetKey},
};

/// Backend-neutral interaction routed into a reusable widget primitive.
#[derive(Clone, Debug, PartialEq)]
pub enum WidgetInput {
    /// Pointer hover moved across the widget bounds.
    PointerMove {
        /// Pointer position in the widget host's logical coordinate space.
        position: Point,
    },
    /// Pointer modifier state changed while the pointer remains active.
    PointerModifiersChanged {
        /// Latest platform-neutral pointer modifier state.
        modifiers: PointerModifiers,
    },
    /// Primary or auxiliary pointer press started at the given point.
    PointerPress {
        /// Pointer position in the widget host's logical coordinate space.
        position: Point,
        /// Button that started the press.
        button: PointerButton,
        /// Modifier state at press time.
        modifiers: PointerModifiers,
    },
    /// Pointer button was pressed twice in quick succession at the given point.
    PointerDoubleClick {
        /// Pointer position in the widget host's logical coordinate space.
        position: Point,
        /// Button that completed the double-click.
        button: PointerButton,
        /// Modifier state at double-click time.
        modifiers: PointerModifiers,
    },
    /// Pointer press ended at the given point.
    PointerRelease {
        /// Pointer position in the widget host's logical coordinate space.
        position: Point,
        /// Button that ended the press.
        button: PointerButton,
        /// Modifier state at release time.
        modifiers: PointerModifiers,
    },
    /// Captured pointer release happened over this widget while another widget owned the press.
    PointerDrop {
        /// Pointer position in the widget host's logical coordinate space.
        position: Point,
        /// Button that ended the captured press.
        button: PointerButton,
        /// Modifier state at release time.
        modifiers: PointerModifiers,
    },
    /// Pointer wheel or trackpad scroll occurred over the widget.
    Wheel {
        /// Pointer position in the widget host's logical coordinate space.
        position: Point,
        /// Logical scroll delta. Positive values move content right/down.
        delta: Vector2,
        /// Modifier state at wheel time.
        modifiers: PointerModifiers,
    },
    /// Keyboard focus changed for the widget.
    FocusChanged(
        /// `true` when the widget gained keyboard focus.
        bool,
    ),
    /// One non-text navigation or activation key was pressed.
    KeyPress(WidgetKey),
    /// One printable character should be inserted into the widget value.
    Character(char),
    /// One higher-level text editing command should be routed to a text field.
    TextEdit(TextEditCommand),
}

impl WidgetInput {
    /// Build a pointer-move input at `position`.
    pub fn pointer_move(position: Point) -> Self {
        Self::PointerMove { position }
    }

    /// Build a pointer-press input with explicit button and modifiers.
    pub fn pointer_press(
        position: Point,
        button: PointerButton,
        modifiers: PointerModifiers,
    ) -> Self {
        Self::PointerPress {
            position,
            button,
            modifiers,
        }
    }

    /// Build a primary-button pointer press with no keyboard modifiers.
    pub fn primary_press(position: Point) -> Self {
        Self::pointer_press(
            position,
            PointerButton::Primary,
            PointerModifiers::default(),
        )
    }

    /// Build a pointer double-click input with explicit button and modifiers.
    pub fn pointer_double_click(
        position: Point,
        button: PointerButton,
        modifiers: PointerModifiers,
    ) -> Self {
        Self::PointerDoubleClick {
            position,
            button,
            modifiers,
        }
    }

    /// Build a primary-button pointer double-click with no keyboard modifiers.
    pub fn primary_double_click(position: Point) -> Self {
        Self::pointer_double_click(
            position,
            PointerButton::Primary,
            PointerModifiers::default(),
        )
    }

    /// Build a pointer-release input with explicit button and modifiers.
    pub fn pointer_release(
        position: Point,
        button: PointerButton,
        modifiers: PointerModifiers,
    ) -> Self {
        Self::PointerRelease {
            position,
            button,
            modifiers,
        }
    }

    /// Build a primary-button pointer release with no keyboard modifiers.
    pub fn primary_release(position: Point) -> Self {
        Self::pointer_release(
            position,
            PointerButton::Primary,
            PointerModifiers::default(),
        )
    }

    /// Build a captured pointer-drop input with explicit button and modifiers.
    pub fn pointer_drop(
        position: Point,
        button: PointerButton,
        modifiers: PointerModifiers,
    ) -> Self {
        Self::PointerDrop {
            position,
            button,
            modifiers,
        }
    }

    /// Build a primary-button pointer drop with no keyboard modifiers.
    pub fn primary_drop(position: Point) -> Self {
        Self::pointer_drop(
            position,
            PointerButton::Primary,
            PointerModifiers::default(),
        )
    }

    /// Build a wheel or trackpad-scroll input with explicit modifiers.
    pub fn wheel(position: Point, delta: Vector2, modifiers: PointerModifiers) -> Self {
        Self::Wheel {
            position,
            delta,
            modifiers,
        }
    }

    /// Build a wheel or trackpad-scroll input with no keyboard modifiers.
    pub fn plain_wheel(position: Point, delta: Vector2) -> Self {
        Self::wheel(position, delta, PointerModifiers::default())
    }

    /// Return the pointer position carried by this input, when it has one.
    pub fn pointer_position(&self) -> Option<Point> {
        match self {
            Self::PointerMove { position }
            | Self::PointerPress { position, .. }
            | Self::PointerDoubleClick { position, .. }
            | Self::PointerRelease { position, .. }
            | Self::PointerDrop { position, .. }
            | Self::Wheel { position, .. } => Some(*position),
            Self::PointerModifiersChanged { .. }
            | Self::FocusChanged(_)
            | Self::KeyPress(_)
            | Self::Character(_)
            | Self::TextEdit(_) => None,
        }
    }

    /// Return the pointer position for inputs that begin an uncaptured pointer interaction.
    ///
    /// Custom canvas and editor widgets can use this to ignore press,
    /// double-click, or wheel starts outside their bounds while still allowing
    /// captured movement and release events to finish an active interaction.
    pub fn pointer_start_position(&self) -> Option<Point> {
        match self {
            Self::PointerPress { position, .. }
            | Self::PointerDoubleClick { position, .. }
            | Self::Wheel { position, .. } => Some(*position),
            _ => None,
        }
    }

    /// Return whether this input begins a pointer interaction outside `bounds`.
    pub fn pointer_start_outside(&self, bounds: Rect) -> bool {
        self.pointer_start_position()
            .is_some_and(|position| !bounds.contains(position))
    }

    /// Return whether this input begins a pointer interaction inside `bounds`.
    pub fn pointer_start_inside(&self, bounds: Rect) -> bool {
        self.pointer_start_position()
            .is_some_and(|position| bounds.contains(position))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn widget_input_constructors_preserve_pointer_payloads() {
        let point = Point::new(12.0, 34.0);
        let modifiers = PointerModifiers {
            command: true,
            shift: true,
            alt: false,
        };

        assert_eq!(
            WidgetInput::pointer_move(point),
            WidgetInput::PointerMove { position: point }
        );
        assert_eq!(
            WidgetInput::pointer_press(point, PointerButton::Secondary, modifiers),
            WidgetInput::PointerPress {
                position: point,
                button: PointerButton::Secondary,
                modifiers,
            }
        );
        assert_eq!(
            WidgetInput::primary_release(point),
            WidgetInput::PointerRelease {
                position: point,
                button: PointerButton::Primary,
                modifiers: PointerModifiers::default(),
            }
        );
        assert_eq!(
            WidgetInput::primary_double_click(point),
            WidgetInput::PointerDoubleClick {
                position: point,
                button: PointerButton::Primary,
                modifiers: PointerModifiers::default(),
            }
        );
        assert_eq!(
            WidgetInput::plain_wheel(point, Vector2::new(0.0, -120.0)),
            WidgetInput::Wheel {
                position: point,
                delta: Vector2::new(0.0, -120.0),
                modifiers: PointerModifiers::default(),
            }
        );
    }

    #[test]
    fn widget_input_reports_pointer_position_by_event_family() {
        let point = Point::new(4.0, 8.0);
        let bounds = Rect::from_xy_size(0.0, 0.0, 10.0, 10.0);
        let outside = Point::new(20.0, 8.0);

        assert_eq!(
            WidgetInput::primary_press(point).pointer_position(),
            Some(point)
        );
        assert_eq!(
            WidgetInput::primary_press(point).pointer_start_position(),
            Some(point)
        );
        assert!(WidgetInput::primary_press(point).pointer_start_inside(bounds));
        assert!(WidgetInput::primary_press(outside).pointer_start_outside(bounds));

        assert_eq!(
            WidgetInput::primary_release(point).pointer_position(),
            Some(point)
        );
        assert_eq!(
            WidgetInput::primary_release(point).pointer_start_position(),
            None
        );
        assert_eq!(
            WidgetInput::KeyPress(WidgetKey::Enter).pointer_position(),
            None
        );
    }
}
