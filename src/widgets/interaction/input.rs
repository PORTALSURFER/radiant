use crate::gui::{input::KeyCode, types::Point};

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

/// Backend-neutral single-line text editing commands.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum TextEditCommand {
    /// Move the caret one logical character left.
    MoveLeft {
        /// Extend the current selection instead of collapsing it.
        extend_selection: bool,
    },
    /// Move the caret one logical character right.
    MoveRight {
        /// Extend the current selection instead of collapsing it.
        extend_selection: bool,
    },
    /// Move the caret to the start of the value.
    MoveHome {
        /// Extend the current selection instead of collapsing it.
        extend_selection: bool,
    },
    /// Move the caret to the end of the value.
    MoveEnd {
        /// Extend the current selection instead of collapsing it.
        extend_selection: bool,
    },
    /// Select the full text value.
    SelectAll,
    /// Insert or paste a text payload at the current selection.
    InsertText(String),
    /// Delete the selected range or previous character.
    Backspace,
    /// Delete the selected range or next character.
    Delete,
    /// Delete the selected range for a cut operation.
    CutSelection,
}

/// Backend-neutral interaction routed into a reusable widget primitive.
#[derive(Clone, Debug, PartialEq)]
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
    /// Pointer wheel or trackpad scroll occurred over the widget.
    Wheel {
        /// Pointer position in the widget host's logical coordinate space.
        position: Point,
        /// Logical scroll delta. Positive values move content right/down.
        delta: crate::gui::types::Vector2,
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
