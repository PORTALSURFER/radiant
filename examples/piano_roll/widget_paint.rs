use radiant::prelude::*;

use super::{model::PianoNote, widget::PianoRollWidget};

#[path = "widget_paint/grid.rs"]
mod grid;
#[path = "widget_paint/keyboard.rs"]
mod keyboard;
#[path = "widget_paint/note.rs"]
mod note;
#[path = "widget_paint/overlay.rs"]
mod overlay;

pub(super) fn append_keyboard(
    widget: &PianoRollWidget,
    primitives: &mut Vec<PaintPrimitive>,
    bounds: Rect,
    theme: &ThemeTokens,
) {
    keyboard::append_keyboard(widget, primitives, bounds, theme);
}

pub(super) fn append_grid(
    widget: &PianoRollWidget,
    primitives: &mut Vec<PaintPrimitive>,
    grid: Rect,
    theme: &ThemeTokens,
) {
    grid::append_grid(widget, primitives, grid, theme);
}

pub(super) fn append_note(
    widget: &PianoRollWidget,
    primitives: &mut Vec<PaintPrimitive>,
    grid: Rect,
    note: PianoNote,
    theme: &ThemeTokens,
) {
    note::append_note(widget, primitives, grid, note, theme);
}

pub(super) fn append_hover_guides(
    widget: &PianoRollWidget,
    primitives: &mut Vec<PaintPrimitive>,
    grid: Rect,
    theme: &ThemeTokens,
) {
    overlay::append_hover_guides(widget, primitives, grid, theme);
}
