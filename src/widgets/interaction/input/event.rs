use crate::{
    gui::input::{InputSequenceRange, InputTimestamp},
    gui::types::{Point, Rect, Vector2},
    widgets::interaction::input::{
        KeyboardModifiers, PointerButton, PointerModifiers, TextEditCommand, WidgetKey,
    },
};

/// Backend-neutral interaction routed into a reusable widget primitive.
#[derive(Clone, Debug, PartialEq)]
pub enum WidgetInput {
    /// Pointer hover moved across the widget bounds.
    PointerMove {
        /// Pointer position in the widget host's logical coordinate space.
        position: Point,
        /// Modifier state captured with this pointer sample.
        modifiers: PointerModifiers,
        /// Optional timestamp captured at the native input boundary.
        timestamp: Option<InputTimestamp>,
        /// Optional opaque native sample sequence range.
        sequence_range: Option<InputSequenceRange>,
    },
    /// Pointer modifier state changed while the pointer remains active.
    PointerModifiersChanged {
        /// Latest platform-neutral pointer modifier state.
        modifiers: PointerModifiers,
        /// Optional timestamp captured at the native input boundary.
        timestamp: Option<InputTimestamp>,
    },
    /// Primary or auxiliary pointer press started at the given point.
    PointerPress {
        /// Pointer position in the widget host's logical coordinate space.
        position: Point,
        /// Button that started the press.
        button: PointerButton,
        /// Modifier state at press time.
        modifiers: PointerModifiers,
        /// Optional timestamp captured at the native input boundary.
        timestamp: Option<InputTimestamp>,
    },
    /// Pointer button was pressed twice in quick succession at the given point.
    PointerDoubleClick {
        /// Pointer position in the widget host's logical coordinate space.
        position: Point,
        /// Button that completed the double-click.
        button: PointerButton,
        /// Modifier state at double-click time.
        modifiers: PointerModifiers,
        /// Optional timestamp captured at the native input boundary.
        timestamp: Option<InputTimestamp>,
    },
    /// Pointer press ended at the given point.
    PointerRelease {
        /// Pointer position in the widget host's logical coordinate space.
        position: Point,
        /// Button that ended the press.
        button: PointerButton,
        /// Modifier state at release time.
        modifiers: PointerModifiers,
        /// Optional timestamp captured at the native input boundary.
        timestamp: Option<InputTimestamp>,
    },
    /// Captured pointer release happened over this widget while another widget owned the press.
    PointerDrop {
        /// Pointer position in the widget host's logical coordinate space.
        position: Point,
        /// Button that ended the captured press.
        button: PointerButton,
        /// Modifier state at release time.
        modifiers: PointerModifiers,
        /// Optional timestamp captured at the native input boundary.
        timestamp: Option<InputTimestamp>,
    },
    /// Pointer wheel or trackpad scroll occurred over the widget.
    Wheel {
        /// Pointer position in the widget host's logical coordinate space.
        position: Point,
        /// Logical scroll delta. Positive values move content right/down.
        delta: Vector2,
        /// Modifier state at wheel time.
        modifiers: PointerModifiers,
        /// Optional timestamp captured at the native input boundary.
        timestamp: Option<InputTimestamp>,
        /// Optional opaque native sample sequence range.
        sequence_range: Option<InputSequenceRange>,
    },
    /// Keyboard focus changed for the widget.
    FocusChanged(
        /// `true` when the widget gained keyboard focus.
        bool,
    ),
    /// One non-text navigation or activation key was pressed.
    KeyPress {
        /// Normalized key identity.
        key: WidgetKey,
        /// Keyboard modifiers captured with this key sample.
        modifiers: KeyboardModifiers,
        /// Whether this sample is a native key-repeat event.
        repeat: bool,
        /// Optional timestamp captured at the native input boundary.
        timestamp: Option<InputTimestamp>,
    },
    /// One printable character should be inserted into the widget value.
    Character {
        /// Character produced by the active keyboard layout.
        character: char,
        /// Optional timestamp captured at the native input boundary.
        timestamp: Option<InputTimestamp>,
    },
    /// One higher-level text editing command should be routed to a text field.
    TextEdit {
        /// Normalized text-edit command.
        command: TextEditCommand,
        /// Optional timestamp captured at the native input boundary.
        timestamp: Option<InputTimestamp>,
    },
}

impl WidgetInput {
    /// Build a pointer-move input at `position`.
    pub fn pointer_move(position: Point) -> Self {
        Self::pointer_move_with_metadata(position, PointerModifiers::default(), None, None)
    }

    /// Build a pointer-move input with native sample metadata.
    pub(crate) fn pointer_move_with_metadata(
        position: Point,
        modifiers: PointerModifiers,
        timestamp: Option<InputTimestamp>,
        sequence_range: Option<InputSequenceRange>,
    ) -> Self {
        Self::PointerMove {
            position,
            modifiers,
            timestamp,
            sequence_range,
        }
    }

    /// Build a pointer-press input with explicit button and modifiers.
    pub fn pointer_press(
        position: Point,
        button: PointerButton,
        modifiers: PointerModifiers,
    ) -> Self {
        Self::pointer_press_with_timestamp(position, button, modifiers, None)
    }

    /// Build a pointer-press input with an optional native input timestamp.
    pub(crate) fn pointer_press_with_timestamp(
        position: Point,
        button: PointerButton,
        modifiers: PointerModifiers,
        timestamp: Option<InputTimestamp>,
    ) -> Self {
        Self::PointerPress {
            position,
            button,
            modifiers,
            timestamp,
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
        Self::pointer_double_click_with_timestamp(position, button, modifiers, None)
    }

    /// Build a pointer double-click input with an optional native input timestamp.
    pub(crate) fn pointer_double_click_with_timestamp(
        position: Point,
        button: PointerButton,
        modifiers: PointerModifiers,
        timestamp: Option<InputTimestamp>,
    ) -> Self {
        Self::PointerDoubleClick {
            position,
            button,
            modifiers,
            timestamp,
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
        Self::pointer_release_with_timestamp(position, button, modifiers, None)
    }

    /// Build a pointer-release input with an optional native input timestamp.
    pub(crate) fn pointer_release_with_timestamp(
        position: Point,
        button: PointerButton,
        modifiers: PointerModifiers,
        timestamp: Option<InputTimestamp>,
    ) -> Self {
        Self::PointerRelease {
            position,
            button,
            modifiers,
            timestamp,
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
        Self::pointer_drop_with_timestamp(position, button, modifiers, None)
    }

    /// Build a captured pointer-drop input with an optional native input timestamp.
    pub(crate) fn pointer_drop_with_timestamp(
        position: Point,
        button: PointerButton,
        modifiers: PointerModifiers,
        timestamp: Option<InputTimestamp>,
    ) -> Self {
        Self::PointerDrop {
            position,
            button,
            modifiers,
            timestamp,
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
        Self::wheel_with_metadata(position, delta, modifiers, None, None)
    }

    /// Build a wheel or trackpad-scroll input with native sample metadata.
    pub(crate) fn wheel_with_metadata(
        position: Point,
        delta: Vector2,
        modifiers: PointerModifiers,
        timestamp: Option<InputTimestamp>,
        sequence_range: Option<InputSequenceRange>,
    ) -> Self {
        Self::Wheel {
            position,
            delta,
            modifiers,
            timestamp,
            sequence_range,
        }
    }

    /// Build a pointer-modifier state change input with an optional native input timestamp.
    pub(crate) fn pointer_modifiers_changed_with_timestamp(
        modifiers: PointerModifiers,
        timestamp: Option<InputTimestamp>,
    ) -> Self {
        Self::PointerModifiersChanged {
            modifiers,
            timestamp,
        }
    }

    /// Build a key-press input with an optional native input timestamp.
    pub(crate) fn key_press_with_timestamp(
        key: WidgetKey,
        timestamp: Option<InputTimestamp>,
    ) -> Self {
        Self::key_press_with_metadata(key, KeyboardModifiers::default(), false, timestamp)
    }

    /// Build a key-press input with native modifier and repeat metadata.
    pub(crate) fn key_press_with_metadata(
        key: WidgetKey,
        modifiers: KeyboardModifiers,
        repeat: bool,
        timestamp: Option<InputTimestamp>,
    ) -> Self {
        Self::KeyPress {
            key,
            modifiers,
            repeat,
            timestamp,
        }
    }

    /// Build a character input with an optional native input timestamp.
    pub(crate) fn character_with_timestamp(
        character: char,
        timestamp: Option<InputTimestamp>,
    ) -> Self {
        Self::Character {
            character,
            timestamp,
        }
    }

    /// Build a text-edit input with an optional native input timestamp.
    pub(crate) fn text_edit_with_timestamp(
        command: TextEditCommand,
        timestamp: Option<InputTimestamp>,
    ) -> Self {
        Self::TextEdit { command, timestamp }
    }

    /// Build a synthetic key-press input without sample metadata.
    pub fn key_press(key: WidgetKey) -> Self {
        Self::key_press_with_timestamp(key, None)
    }

    /// Build a synthetic character input without sample metadata.
    pub fn character(character: char) -> Self {
        Self::character_with_timestamp(character, None)
    }

    /// Build a synthetic text-edit input without sample metadata.
    pub fn text_edit(command: TextEditCommand) -> Self {
        Self::text_edit_with_timestamp(command, None)
    }

    /// Build a wheel or trackpad-scroll input with no keyboard modifiers.
    pub fn plain_wheel(position: Point, delta: Vector2) -> Self {
        Self::wheel(position, delta, PointerModifiers::default())
    }

    /// Return the pointer position carried by this input, when it has one.
    pub fn pointer_position(&self) -> Option<Point> {
        match self {
            Self::PointerMove { position, .. }
            | Self::PointerPress { position, .. }
            | Self::PointerDoubleClick { position, .. }
            | Self::PointerRelease { position, .. }
            | Self::PointerDrop { position, .. }
            | Self::Wheel { position, .. } => Some(*position),
            Self::PointerModifiersChanged { .. }
            | Self::FocusChanged(_)
            | Self::KeyPress { .. }
            | Self::Character { .. }
            | Self::TextEdit { .. } => None,
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
            WidgetInput::PointerMove {
                position: point,
                modifiers: PointerModifiers::default(),
                timestamp: None,
                sequence_range: None,
            }
        );
        assert_eq!(
            WidgetInput::pointer_press(point, PointerButton::Secondary, modifiers),
            WidgetInput::PointerPress {
                position: point,
                button: PointerButton::Secondary,
                modifiers,
                timestamp: None,
            }
        );
        assert_eq!(
            WidgetInput::primary_release(point),
            WidgetInput::PointerRelease {
                position: point,
                button: PointerButton::Primary,
                modifiers: PointerModifiers::default(),
                timestamp: None,
            }
        );
        assert_eq!(
            WidgetInput::primary_drop(point),
            WidgetInput::PointerDrop {
                position: point,
                button: PointerButton::Primary,
                modifiers: PointerModifiers::default(),
                timestamp: None,
            }
        );
        assert_eq!(
            WidgetInput::primary_double_click(point),
            WidgetInput::PointerDoubleClick {
                position: point,
                button: PointerButton::Primary,
                modifiers: PointerModifiers::default(),
                timestamp: None,
            }
        );
        let timestamp = Some(InputTimestamp::capture());
        assert_eq!(
            WidgetInput::pointer_double_click_with_timestamp(
                point,
                PointerButton::Primary,
                PointerModifiers::default(),
                timestamp,
            ),
            WidgetInput::PointerDoubleClick {
                position: point,
                button: PointerButton::Primary,
                modifiers: PointerModifiers::default(),
                timestamp,
            }
        );
        assert_eq!(
            WidgetInput::plain_wheel(point, Vector2::new(0.0, -120.0)),
            WidgetInput::Wheel {
                position: point,
                delta: Vector2::new(0.0, -120.0),
                modifiers: PointerModifiers::default(),
                timestamp: None,
                sequence_range: None,
            }
        );
        assert_eq!(
            WidgetInput::wheel(point, Vector2::new(0.0, -120.0), modifiers),
            WidgetInput::Wheel {
                position: point,
                delta: Vector2::new(0.0, -120.0),
                modifiers,
                timestamp: None,
                sequence_range: None,
            }
        );
    }

    #[test]
    fn public_keyboard_constructors_omit_sample_metadata() {
        assert_eq!(
            WidgetInput::key_press(WidgetKey::Enter),
            WidgetInput::KeyPress {
                key: WidgetKey::Enter,
                modifiers: KeyboardModifiers::default(),
                repeat: false,
                timestamp: None,
            }
        );
        assert_eq!(
            WidgetInput::character('x'),
            WidgetInput::Character {
                character: 'x',
                timestamp: None,
            }
        );
        assert_eq!(
            WidgetInput::text_edit(TextEditCommand::SelectAll),
            WidgetInput::TextEdit {
                command: TextEditCommand::SelectAll,
                timestamp: None,
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
            WidgetInput::key_press(WidgetKey::Enter).pointer_position(),
            None
        );
    }
}
