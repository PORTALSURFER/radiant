//! Backend-neutral widget interaction events and emitted messages.

use crate::gui::input::KeyCode;
use crate::gui::types::Point;
use std::{any::Any, sync::Arc};

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

/// Type-erased widget output payload.
#[derive(Clone)]
pub struct CustomWidgetOutput {
    payload: Arc<dyn Any + Send + Sync>,
}

impl CustomWidgetOutput {
    /// Build a custom widget output from any cloneable, thread-safe payload.
    pub fn new<T>(payload: T) -> Self
    where
        T: Send + Sync + 'static,
    {
        Self {
            payload: Arc::new(payload),
        }
    }

    /// Downcast this output payload to the requested type.
    pub fn downcast_ref<T: 'static>(&self) -> Option<&T> {
        self.payload.downcast_ref::<T>()
    }
}

impl std::fmt::Debug for CustomWidgetOutput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CustomWidgetOutput").finish_non_exhaustive()
    }
}

impl PartialEq for CustomWidgetOutput {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.payload, &other.payload)
    }
}

/// Typed widget output payload.
///
/// Outputs are intentionally open: a widget emits its own message type with
/// [`WidgetOutput::typed`], and message mappers downcast only the payload types
/// they understand. Adding a widget does not require adding a central enum
/// variant.
#[derive(Clone, PartialEq)]
pub struct WidgetOutput {
    payload: CustomWidgetOutput,
}

impl WidgetOutput {
    /// Build a typed widget output payload.
    pub fn typed<T>(payload: T) -> Self
    where
        T: Send + Sync + 'static,
    {
        Self {
            payload: CustomWidgetOutput::new(payload),
        }
    }

    /// Downcast this widget output to the requested payload type.
    pub fn typed_ref<T: 'static>(&self) -> Option<&T> {
        self.payload.downcast_ref()
    }

    /// Build a user-defined widget output payload.
    pub fn custom<T>(payload: T) -> Self
    where
        T: Send + Sync + 'static,
    {
        Self::typed(payload)
    }

    /// Downcast this widget output to the requested custom payload type.
    pub fn custom_ref<T: 'static>(&self) -> Option<&T> {
        self.typed_ref()
    }
}

impl std::fmt::Debug for WidgetOutput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WidgetOutput").finish_non_exhaustive()
    }
}
