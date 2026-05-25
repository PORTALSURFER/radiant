use radiant::prelude::*;

use super::widget::PianoRollWidget;

#[path = "widget_paint/grid.rs"]
mod grid;
#[path = "widget_paint/keyboard.rs"]
mod keyboard;
#[path = "widget_paint/note.rs"]
mod note;
#[path = "widget_paint/overlay.rs"]
mod overlay;
#[path = "widget_paint/velocity.rs"]
mod velocity;

pub(super) use grid::append_grid;
pub(super) use keyboard::{
    append_keyboard, append_keyboard_interaction, append_selected_pitch_lane,
};
pub(super) use note::append_note;
pub(super) use overlay::{append_drag_preview, append_hover_guides};
pub(super) use velocity::append_velocity_lane;

pub(super) fn append_editor_clip_start(
    widget: &PianoRollWidget,
    primitives: &mut Vec<PaintPrimitive>,
    grid: Rect,
) {
    primitives.push(PaintPrimitive::ClipStart(PaintClipStart {
        node_id: widget.common.id,
        rect: grid,
    }));
}

pub(super) fn append_editor_clip_end(
    widget: &PianoRollWidget,
    primitives: &mut Vec<PaintPrimitive>,
) {
    primitives.push(PaintPrimitive::ClipEnd(PaintClipEnd {
        node_id: widget.common.id,
    }));
}
