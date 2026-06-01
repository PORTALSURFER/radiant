use crate::gui::{
    input::KeyCode,
    types::{Point, Rect},
};

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
    /// Request completion for the focused widget.
    Tab,
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
            KeyCode::Tab => Self::Tab,
            KeyCode::Space => Self::Space,
            KeyCode::Backspace => Self::Backspace,
            KeyCode::ArrowLeft => Self::ArrowLeft,
            KeyCode::ArrowRight => Self::ArrowRight,
            KeyCode::ArrowUp => Self::ArrowUp,
            KeyCode::ArrowDown => Self::ArrowDown,
            KeyCode::Home => Self::Home,
            KeyCode::End => Self::End,
            KeyCode::Delete => Self::Delete,
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
    /// Move the caret one word boundary left.
    MoveWordLeft {
        /// Extend the current selection instead of collapsing it.
        extend_selection: bool,
    },
    /// Move the caret one word boundary right.
    MoveWordRight {
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
    /// Delete the selected range or previous word boundary span.
    DeleteWordLeft,
    /// Delete the selected range or next word boundary span.
    DeleteWordRight,
    /// Delete the selected range for a cut operation.
    CutSelection,
}

/// Modifier state captured with one pointer interaction.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PointerModifiers {
    /// Whether the platform command modifier is held.
    pub command: bool,
    /// Whether Shift is held.
    pub shift: bool,
    /// Whether Alt is held.
    pub alt: bool,
}

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
        delta: crate::gui::types::Vector2,
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
