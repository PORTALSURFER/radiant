use super::super::pointer::CanvasPointer;
use crate::widgets::interaction::{PointerButton, PointerModifiers};

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct ActiveCanvasPress {
    pub(super) origin: CanvasPointer,
    pub(super) button: PointerButton,
    pub(super) modifiers: PointerModifiers,
}

impl ActiveCanvasPress {
    pub(super) const fn new(
        origin: CanvasPointer,
        button: PointerButton,
        modifiers: PointerModifiers,
    ) -> Self {
        Self {
            origin,
            button,
            modifiers,
        }
    }
}
