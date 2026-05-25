use super::{
    AppMessage, DATA_SOURCE_NOTE, NoteSelectionMode, PIANO_ROLL_WIDGET_ID, PITCH_ROWS,
    PianoRollMessage, PianoRollState, PianoRollTool, STATUS_WIDGET_ID, TOTAL_BEATS,
    drag::PianoDrag,
    geometry::{row_height_for, x_for_beat_view, y_for_pitch_view},
    model::{PianoNote, STRESS_NOTE_COUNT},
    paint, project_surface, update,
    widget::{NoteResizeEdge, PianoRollWidget, PianoRollWidgetParts},
};
use radiant::prelude::*;
use radiant::runtime::{RuntimeBridge, SurfaceRuntime};
use radiant::widgets::PointerModifiers;

#[path = "tests/hover_overlay.rs"]
mod hover_overlay;
#[path = "tests/keyboard_hover.rs"]
mod keyboard_hover;
#[path = "tests/marquee_selection.rs"]
mod marquee_selection;
#[path = "tests/marquee_stress.rs"]
mod marquee_stress;
#[path = "tests/model_behavior.rs"]
mod model_behavior;
#[path = "tests/note_drag.rs"]
mod note_drag;
#[path = "tests/note_move_drag.rs"]
mod note_move_drag;
#[path = "tests/note_static_paint.rs"]
mod note_static_paint;
#[path = "tests/paint_static.rs"]
mod paint_static;
#[path = "tests/pan_navigation.rs"]
mod pan_navigation;
#[path = "tests/runtime.rs"]
mod runtime;
#[path = "tests/selection.rs"]
mod selection;
#[path = "tests/time_selection_drag.rs"]
mod time_selection_drag;
#[path = "tests/time_selection_preview.rs"]
mod time_selection_preview;
#[path = "tests/velocity_drag.rs"]
mod velocity_drag;
#[path = "tests/velocity_drag_alt.rs"]
mod velocity_drag_alt;
#[path = "tests/velocity_drag_group.rs"]
mod velocity_drag_group;
#[path = "tests/velocity_paint.rs"]
mod velocity_paint;
#[path = "tests/wheel_navigation.rs"]
mod wheel_navigation;

fn piano_roll_test_bridge(state: PianoRollState) -> impl RuntimeBridge<AppMessage> {
    radiant::app(state)
        .view(project_surface)
        .shortcuts(
            |_, _, press, _| match UndoRedoIntent::from_key_press(press) {
                Some(UndoRedoIntent::Undo) => ShortcutResolution::action(AppMessage::Undo),
                Some(UndoRedoIntent::Redo) => ShortcutResolution::action(AppMessage::Redo),
                None => ShortcutResolution::unhandled(),
            },
        )
        .animation(|state| state.running)
        .on_frame(|| AppMessage::Frame)
        .update(update)
        .into_bridge()
}

fn status_text<Bridge>(runtime: &SurfaceRuntime<Bridge, AppMessage>) -> String
where
    Bridge: RuntimeBridge<AppMessage>,
{
    runtime
        .paint_plan(&ThemeTokens::default())
        .primitives
        .iter()
        .find_map(|primitive| match primitive {
            PaintPrimitive::Text(text) if text.widget_id == STATUS_WIDGET_ID => {
                Some(text.text.as_str().to_string())
            }
            _ => None,
        })
        .expect("status text should be painted")
}

fn fill_alpha_for_rect(primitives: &[PaintPrimitive], rect: Rect) -> u8 {
    primitives
        .iter()
        .find_map(|primitive| match primitive {
            PaintPrimitive::FillRect(fill) if fill.rect == rect => Some(fill.color.a),
            _ => None,
        })
        .expect("fill primitive for note rect should be painted")
}

fn velocity_for(velocities: &[(u32, f32)], id: u32) -> f32 {
    velocities
        .iter()
        .find_map(|(note_id, velocity)| (*note_id == id).then_some(*velocity))
        .unwrap_or_else(|| panic!("missing velocity for note {id}"))
}
