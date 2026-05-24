use super::{
    AppMessage, DATA_SOURCE_NOTE, PIANO_ROLL_WIDGET_ID, PITCH_ROWS, PianoRollMessage,
    PianoRollState, STATUS_WIDGET_ID, TOTAL_BEATS,
    geometry::{row_height, x_for_beat, y_for_pitch},
    project_surface, update,
    widget::PianoRollWidget,
};
use radiant::prelude::*;
use radiant::runtime::{RuntimeBridge, SurfaceRuntime};

#[test]
fn piano_roll_tick_advances_synthetic_playhead_without_midi_or_dsp() {
    let mut state = PianoRollState::default();
    let initial = state.playhead_beat;

    state.tick();

    assert_eq!(state.frame, 1);
    assert!(state.playhead_beat > initial);
    assert_eq!(DATA_SOURCE_NOTE, "without_midi_or_dsp");
}

#[test]
fn piano_roll_widget_paints_keyboard_grid_notes_and_playhead() {
    let state = PianoRollState::default();
    let widget = PianoRollWidget::new(state.notes, state.selected_note, state.playhead_beat);
    let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(960.0, 390.0));
    let mut primitives = Vec::new();
    let mut overlay = Vec::new();

    widget.append_paint(
        &mut primitives,
        bounds,
        &LayoutOutput::default(),
        &ThemeTokens::default(),
    );
    widget.append_runtime_overlay_paint(
        &mut overlay,
        bounds,
        &LayoutOutput::default(),
        &ThemeTokens::default(),
    );

    assert!(
        primitives
            .iter()
            .filter(|primitive| matches!(primitive, PaintPrimitive::FillRect(_)))
            .count()
            > PITCH_ROWS
    );
    assert!(primitives.iter().any(
        |primitive| matches!(primitive, PaintPrimitive::Text(text) if text.text.as_str() == "C4")
    ));
    assert!(
        overlay
            .iter()
            .any(|primitive| matches!(primitive, PaintPrimitive::FillRect(_))),
        "playhead should paint as a lightweight runtime overlay"
    );
}

#[test]
fn piano_roll_clicking_empty_grid_creates_quantized_note() {
    let state = PianoRollState::default();
    let mut widget = PianoRollWidget::new(state.notes, state.selected_note, state.playhead_beat);
    let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(960.0, 390.0));
    let grid = widget.editor_rect(bounds);

    let output = widget.handle_input(
        bounds,
        WidgetInput::PointerPress {
            position: Point::new(x_for_beat(grid, 6.10), y_for_pitch(grid, 58) + 4.0),
            button: PointerButton::Primary,
            modifiers: Default::default(),
        },
    );

    assert_eq!(
        output.and_then(|output| output.typed_ref::<PianoRollMessage>().copied()),
        Some(PianoRollMessage::CreateNote {
            pitch: 58,
            start_beat: 6.10
        })
    );
}

#[test]
fn piano_roll_drag_routes_move_message() {
    let state = PianoRollState::default();
    let mut widget = PianoRollWidget::new(state.notes, state.selected_note, state.playhead_beat);
    let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(960.0, 390.0));
    let grid = widget.editor_rect(bounds);
    let note = widget.note_by_id(2).expect("default note should exist");
    let start = widget.note_rect(grid, note).center();

    let _ = widget.handle_input(
        bounds,
        WidgetInput::PointerPress {
            position: start,
            button: PointerButton::Primary,
            modifiers: Default::default(),
        },
    );
    let output = widget.handle_input(
        bounds,
        WidgetInput::PointerMove {
            position: Point::new(
                start.x + grid.width() / TOTAL_BEATS,
                start.y - row_height(grid),
            ),
        },
    );

    assert!(matches!(
        output.and_then(|output| output.typed_ref::<PianoRollMessage>().copied()),
        Some(PianoRollMessage::MoveNote {
            id: 2,
            pitch: 56,
            ..
        })
    ));
    assert!(!widget.prefers_pointer_move_paint_only());
}

#[test]
fn piano_roll_hover_uses_paint_only_runtime_overlay() {
    let state = PianoRollState::default();
    let mut widget = PianoRollWidget::new(state.notes, state.selected_note, state.playhead_beat);
    let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(960.0, 390.0));
    let grid = widget.editor_rect(bounds);
    let note = widget.note_by_id(2).expect("default note should exist");

    let output = widget.handle_input(
        bounds,
        WidgetInput::PointerMove {
            position: widget.note_rect(grid, note).center(),
        },
    );

    assert!(output.is_none());
    assert_eq!(widget.hover_note, Some(2));
    assert!(widget.prefers_pointer_move_paint_only());
    let mut overlay = Vec::new();
    widget.append_runtime_overlay_paint(
        &mut overlay,
        bounds,
        &LayoutOutput::default(),
        &ThemeTokens::default(),
    );
    assert!(
        overlay
            .iter()
            .any(|primitive| matches!(primitive, PaintPrimitive::StrokeRect(_))),
        "hovered note should paint as a lightweight runtime overlay"
    );
}

#[test]
fn piano_roll_runtime_hover_does_not_refresh_surface() {
    let bridge = piano_roll_test_bridge(PianoRollState::default());
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(1040.0, 620.0));
    let bounds = runtime.layout().rects[&PIANO_ROLL_WIDGET_ID];
    let first = runtime
        .dispatch_pointer_move_with_outcome(Point::new(bounds.min.x + 160.0, bounds.center().y));
    let second = runtime
        .dispatch_pointer_move_with_outcome(Point::new(bounds.min.x + 260.0, bounds.center().y));

    assert!(first.needs_scene_rebuild());
    assert!(second.paint_only_requested);
    assert!(
        !second.needs_scene_rebuild(),
        "stable piano-roll hover should avoid reprojection and full scene rebuilds"
    );
}

#[test]
fn piano_roll_runtime_frame_messages_advance_status() {
    let bridge = piano_roll_test_bridge(PianoRollState::default());
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(1040.0, 620.0));
    let initial_status = status_text(&runtime);

    assert!(runtime.bridge_mut().needs_animation());
    assert!(runtime.bridge_mut().queue_animation_frame());
    let outcome = runtime.drain_runtime_messages();

    assert_eq!(outcome.messages_dispatched, 1);
    assert_ne!(status_text(&runtime), initial_status);
}

fn piano_roll_test_bridge(state: PianoRollState) -> impl RuntimeBridge<AppMessage> {
    radiant::app(state)
        .view(project_surface)
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
