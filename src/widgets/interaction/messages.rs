use crate::gui::types::Point;

use super::WidgetInput;

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
