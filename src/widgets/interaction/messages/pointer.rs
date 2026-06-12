use crate::{
    gui::types::Point,
    layout::Vector2,
    widgets::interaction::input::{PointerButton, PointerModifiers},
};

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
