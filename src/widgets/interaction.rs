//! Backend-neutral widget interaction events and emitted messages.

use crate::gui::input::KeyCode;
use crate::gui::types::Point;

/// Pointer button routed into a widget.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PointerButton {
    /// Primary/left pointer button.
    Primary,
    /// Secondary/right pointer button.
    Secondary,
    /// Auxiliary or middle pointer button.
    Auxiliary,
}

/// Backend-neutral key intents consumed by reusable widget primitives.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum WidgetKey {
    /// Activate or submit the focused widget.
    Enter,
    /// Activate the focused widget.
    Space,
    /// Move one logical position toward the leading edge.
    ArrowLeft,
    /// Move one logical position toward the trailing edge.
    ArrowRight,
    /// Move one logical position upward.
    ArrowUp,
    /// Move one logical position downward.
    ArrowDown,
    /// Move to the start of the value or range.
    Home,
    /// Move to the end of the value or range.
    End,
    /// Delete the codepoint before the caret.
    Backspace,
    /// Delete the codepoint after the caret.
    Delete,
}

impl WidgetKey {
    /// Convert a backend-neutral GUI key code into a widget-edit key when supported.
    pub fn from_key_code(key: KeyCode) -> Option<Self> {
        Some(match key {
            KeyCode::Enter => Self::Enter,
            KeyCode::Space => Self::Space,
            KeyCode::ArrowLeft => Self::ArrowLeft,
            KeyCode::ArrowRight => Self::ArrowRight,
            KeyCode::ArrowUp => Self::ArrowUp,
            KeyCode::ArrowDown => Self::ArrowDown,
            KeyCode::Home => Self::Home,
            KeyCode::End => Self::End,
            _ => return None,
        })
    }
}

/// Backend-neutral interaction routed into a reusable widget primitive.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum WidgetInput {
    /// Pointer hover moved across the widget bounds.
    PointerMove {
        /// Pointer position in the widget host's logical coordinate space.
        position: Point,
    },
    /// Primary or auxiliary pointer press started at the given point.
    PointerPress {
        /// Pointer position in the widget host's logical coordinate space.
        position: Point,
        /// Button that started the press.
        button: PointerButton,
    },
    /// Pointer press ended at the given point.
    PointerRelease {
        /// Pointer position in the widget host's logical coordinate space.
        position: Point,
        /// Button that ended the press.
        button: PointerButton,
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
}

/// Message emitted by a reusable button primitive.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ButtonMessage {
    /// The button was activated by pointer or keyboard input.
    Activate,
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

/// Union over emitted messages from the first reusable widget primitives.
#[derive(Clone, Debug, PartialEq)]
pub enum WidgetOutput {
    /// Button activation output.
    Button(ButtonMessage),
    /// Toggle value-change output.
    Toggle(ToggleMessage),
    /// Text-input editing or submit output.
    TextInput(TextInputMessage),
    /// Scrollbar viewport-request output.
    Scrollbar(ScrollbarMessage),
}
