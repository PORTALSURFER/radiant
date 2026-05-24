use super::{
    AppMessage, DATA_SOURCE_NOTE, NoteSelectionMode, PIANO_ROLL_WIDGET_ID, PITCH_ROWS,
    PianoRollMessage, PianoRollState, PianoRollTool, STATUS_WIDGET_ID, TOTAL_BEATS,
    geometry::{row_height_for, x_for_beat_view, y_for_pitch_view},
    model::STRESS_NOTE_COUNT,
    paint, project_surface, update,
    widget::PianoRollWidget,
};
use radiant::prelude::*;
use radiant::runtime::{RuntimeBridge, SurfaceRuntime};
use radiant::widgets::PointerModifiers;

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
    let widget = PianoRollWidget::new(
        state.notes,
        state.selected_note,
        state.selected_notes.clone(),
        state.selected_pitch,
        state.playhead_beat,
        state.viewport,
        state.tool,
    );
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
fn piano_roll_viewport_zoom_and_pan_updates_visible_range() {
    let mut state = PianoRollState::default();

    state.apply_roll_message(PianoRollMessage::ZoomTime { factor: 0.5 });
    state.apply_roll_message(PianoRollMessage::PanViewport {
        beat_delta: 3.0,
        pitch_delta: 0,
    });
    state.apply_roll_message(PianoRollMessage::ZoomPitch { rows_delta: -8 });
    state.apply_roll_message(PianoRollMessage::PanViewport {
        beat_delta: 0.0,
        pitch_delta: 4,
    });

    assert_eq!(state.viewport.visible_beats, 8.0);
    assert_eq!(state.viewport.beat_start, 7.0);
    assert_eq!(state.viewport.visible_pitches, 16);
    assert_eq!(state.viewport.pitch_start, 56);
    assert!(state.status().contains("beats 7.0-15.0"));
}

#[test]
fn piano_roll_viewport_scales_note_geometry() {
    let mut state = PianoRollState::default();
    state.apply_roll_message(PianoRollMessage::ZoomTime { factor: 0.5 });
    state.apply_roll_message(PianoRollMessage::ZoomPitch { rows_delta: -8 });
    let widget = PianoRollWidget::new(
        state.notes.clone(),
        state.selected_note,
        state.selected_notes.clone(),
        state.selected_pitch,
        state.playhead_beat,
        state.viewport,
        state.tool,
    );
    let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(960.0, 390.0));
    let grid = widget.editor_rect(bounds);
    let note = widget.note_by_id(2).expect("default note should exist");
    let zoomed = widget.note_rect(grid, note);
    let default_widget = PianoRollWidget::new(
        state.notes.clone(),
        state.selected_note,
        state.selected_notes.clone(),
        state.selected_pitch,
        state.playhead_beat,
        Default::default(),
        state.tool,
    );
    let unzoomed = default_widget.note_rect(grid, note);

    assert!(
        zoomed.width() > unzoomed.width(),
        "horizontal zoom should increase note width in screen space"
    );
    assert!(
        zoomed.height() > unzoomed.height(),
        "vertical zoom should increase row height in screen space"
    );
}

#[test]
fn piano_roll_note_geometry_can_move_past_vertical_viewport_edges_for_clipping() {
    let mut state = PianoRollState::default();
    state.viewport.pitch_start = 60;
    state.viewport.visible_pitches = 8;
    state.notes = vec![super::model::PianoNote {
        id: 101,
        pitch: 72,
        start_beat: 1.0,
        length_beats: 1.0,
        velocity: 0.7,
    }];
    let widget = PianoRollWidget::new(
        state.notes.clone(),
        state.selected_note,
        state.selected_notes,
        state.selected_pitch,
        state.playhead_beat,
        state.viewport,
        state.tool,
    );
    let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(960.0, 390.0));
    let grid = widget.editor_rect(bounds);
    let rect = widget.note_rect(grid, state.notes[0]);

    assert!(
        rect.max.y < grid.min.y,
        "notes above the visible pitch range should project past the editor edge and be clipped by the paint clip"
    );
}

#[test]
fn piano_roll_clips_notes_to_editor_grid_with_radiant_clip() {
    let mut state = PianoRollState::default();
    state.notes = vec![
        super::model::PianoNote {
            id: 101,
            pitch: 55,
            start_beat: 2.0,
            length_beats: 2.0,
            velocity: 1.0,
        },
        super::model::PianoNote {
            id: 102,
            pitch: 57,
            start_beat: 6.0,
            length_beats: 2.0,
            velocity: 1.0,
        },
    ];
    state.selected_note = None;
    state.selected_notes.clear();
    state.viewport.beat_start = 3.0;
    state.viewport.visible_beats = 4.0;
    let widget = PianoRollWidget::new(
        state.notes,
        state.selected_note,
        state.selected_notes,
        state.selected_pitch,
        state.playhead_beat,
        state.viewport,
        state.tool,
    );
    let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(960.0, 390.0));
    let grid = widget.editor_rect(bounds);
    let mut primitives = Vec::new();

    widget.append_paint(
        &mut primitives,
        bounds,
        &LayoutOutput::default(),
        &ThemeTokens::default(),
    );

    let clip_start = primitives
        .iter()
        .position(
            |primitive| matches!(primitive, PaintPrimitive::ClipStart(clip) if clip.rect == grid),
        )
        .expect("piano-roll notes should enter a Radiant clip for the editor grid");
    let clip_end = primitives
        .iter()
        .position(
            |primitive| matches!(primitive, PaintPrimitive::ClipEnd(clip) if clip.node_id == widget.common.id),
        )
        .expect("piano-roll notes should leave the editor-grid clip");
    let note_rects = widget
        .notes
        .iter()
        .map(|note| widget.note_rect(grid, *note))
        .collect::<Vec<_>>();
    let note_fill_positions = note_rects
        .iter()
        .map(|rect| {
            primitives
                .iter()
                .position(
                    |primitive| matches!(primitive, PaintPrimitive::FillRect(fill) if fill.rect == *rect),
                )
                .expect("raw note fill should be emitted inside the clip")
        })
        .collect::<Vec<_>>();

    assert!(clip_start < clip_end);
    assert!(
        note_fill_positions
            .iter()
            .all(|position| clip_start < *position && *position < clip_end),
        "note geometry should be clipped by Radiant clip primitives rather than per-rect clamping"
    );
    assert!(
        note_rects
            .iter()
            .any(|rect| rect.min.x < grid.min.x && rect.max.x > grid.min.x),
        "test should include a note that overhangs the left edge before renderer clipping"
    );
    assert!(
        note_rects
            .iter()
            .any(|rect| rect.min.x < grid.max.x && rect.max.x > grid.max.x),
        "test should include a note that overhangs the right edge before renderer clipping"
    );
}

#[test]
fn piano_roll_drag_paints_new_note_length_before_commit() {
    let state = PianoRollState::default();
    let mut widget = PianoRollWidget::new(
        state.notes,
        state.selected_note,
        state.selected_notes,
        state.selected_pitch,
        state.playhead_beat,
        state.viewport,
        state.tool,
    );
    let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(960.0, 390.0));
    let grid = widget.editor_rect(bounds);
    let start = Point::new(
        x_for_beat_view(grid, state.viewport, 6.10),
        y_for_pitch_view(grid, state.viewport, 58) + 4.0,
    );
    let end = Point::new(x_for_beat_view(grid, state.viewport, 7.60), start.y);

    let press_output = widget.handle_input(
        bounds,
        WidgetInput::PointerPress {
            position: start,
            button: PointerButton::Primary,
            modifiers: Default::default(),
        },
    );
    let move_output = widget.handle_input(bounds, WidgetInput::PointerMove { position: end });

    assert!(press_output.is_none());
    assert!(move_output.is_none());
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
            .any(|primitive| matches!(primitive, PaintPrimitive::FillRect(fill) if fill.color == paint::translucent(ThemeTokens::default().highlight_blue, 120))),
        "new-note paint drag should show a local note preview"
    );

    let output = widget.handle_input(
        bounds,
        WidgetInput::PointerRelease {
            position: end,
            button: PointerButton::Primary,
            modifiers: Default::default(),
        },
    );

    assert_eq!(
        output.and_then(|output| output.typed_ref::<PianoRollMessage>().cloned()),
        Some(PianoRollMessage::CreateNote {
            pitch: 58,
            start_beat: 6.0,
            length_beats: 1.5,
        })
    );
}

#[test]
fn piano_roll_drag_routes_move_message() {
    let state = PianoRollState::default();
    let mut widget = PianoRollWidget::new(
        state.notes,
        state.selected_note,
        state.selected_notes,
        state.selected_pitch,
        state.playhead_beat,
        state.viewport,
        state.tool,
    );
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
                start.y - row_height_for(grid, state.viewport),
            ),
        },
    );

    assert!(output.is_none());
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
            .any(|primitive| matches!(primitive, PaintPrimitive::FillRect(fill) if fill.color == paint::translucent(ThemeTokens::default().highlight_blue, 120))),
        "moving a held note should paint a local drag preview"
    );
    let release = widget.handle_input(
        bounds,
        WidgetInput::PointerRelease {
            position: Point::new(
                start.x + grid.width() / TOTAL_BEATS,
                start.y - row_height_for(grid, state.viewport),
            ),
            button: PointerButton::Primary,
            modifiers: Default::default(),
        },
    );
    assert!(matches!(
        release.and_then(|output| output.typed_ref::<PianoRollMessage>().cloned()),
        Some(PianoRollMessage::MoveNote {
            id: 2,
            pitch: 56,
            ..
        })
    ));
}

#[test]
fn piano_roll_dragging_selected_note_moves_the_selected_group() {
    let mut state = PianoRollState::default();
    state.apply_roll_message(PianoRollMessage::SelectNotes {
        ids: vec![2, 3],
        mode: NoteSelectionMode::Replace,
    });
    let mut widget = PianoRollWidget::new(
        state.notes.clone(),
        state.selected_note,
        state.selected_notes.clone(),
        state.selected_pitch,
        state.playhead_beat,
        state.viewport,
        state.tool,
    );
    let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(960.0, 390.0));
    let grid = widget.editor_rect(bounds);
    let note = widget.note_by_id(2).expect("selected note should exist");
    let start = widget.note_rect(grid, note).center();
    let end = Point::new(
        start.x + grid.width() / TOTAL_BEATS,
        start.y - row_height_for(grid, state.viewport),
    );

    let press = widget.handle_input(
        bounds,
        WidgetInput::PointerPress {
            position: start,
            button: PointerButton::Primary,
            modifiers: PointerModifiers::default(),
        },
    );
    assert!(
        press.is_none(),
        "pressing an already selected note should keep the group selection"
    );
    widget.handle_input(bounds, WidgetInput::PointerMove { position: end });
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
            .filter(|primitive| matches!(primitive, PaintPrimitive::FillRect(fill) if fill.color == paint::translucent(ThemeTokens::default().highlight_blue, 120)))
            .count()
            >= 2,
        "drag preview should show every selected note moving together"
    );

    let release = widget
        .handle_input(
            bounds,
            WidgetInput::PointerRelease {
                position: end,
                button: PointerButton::Primary,
                modifiers: PointerModifiers::default(),
            },
        )
        .and_then(|output| output.typed_ref::<PianoRollMessage>().cloned())
        .expect("group drag release should commit a move");

    match release {
        PianoRollMessage::MoveNotes {
            ids,
            pitch_delta,
            beat_delta,
        } => {
            assert_eq!(ids, vec![2, 3]);
            assert_eq!(pitch_delta, 1);
            assert!((beat_delta - 1.0).abs() < f32::EPSILON);
            state.apply_roll_message(PianoRollMessage::MoveNotes {
                ids,
                pitch_delta,
                beat_delta,
            });
        }
        other => panic!("expected group move message, got {other:?}"),
    }

    assert_eq!(
        state.notes.iter().find(|note| note.id == 2).unwrap().pitch,
        56
    );
    assert_eq!(
        state.notes.iter().find(|note| note.id == 3).unwrap().pitch,
        61
    );
    assert_eq!(state.selected_notes, vec![2, 3]);
}

#[test]
fn piano_roll_modifier_click_adds_and_toggles_note_selection() {
    let state = PianoRollState::default();
    let mut widget = PianoRollWidget::new(
        state.notes,
        state.selected_note,
        state.selected_notes,
        state.selected_pitch,
        state.playhead_beat,
        state.viewport,
        state.tool,
    );
    let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(960.0, 390.0));
    let grid = widget.editor_rect(bounds);
    let note = widget.note_by_id(3).expect("note should exist");
    let position = widget.note_rect(grid, note).center();

    let shift_output = widget
        .handle_input(
            bounds,
            WidgetInput::PointerPress {
                position,
                button: PointerButton::Primary,
                modifiers: PointerModifiers {
                    shift: true,
                    ..PointerModifiers::default()
                },
            },
        )
        .and_then(|output| output.typed_ref::<PianoRollMessage>().cloned());
    assert_eq!(
        shift_output,
        Some(PianoRollMessage::SelectNotes {
            ids: vec![3],
            mode: NoteSelectionMode::Add,
        })
    );

    let command_output = widget
        .handle_input(
            bounds,
            WidgetInput::PointerPress {
                position,
                button: PointerButton::Primary,
                modifiers: PointerModifiers {
                    command: true,
                    ..PointerModifiers::default()
                },
            },
        )
        .and_then(|output| output.typed_ref::<PianoRollMessage>().cloned());
    assert_eq!(
        command_output,
        Some(PianoRollMessage::SelectNotes {
            ids: vec![3],
            mode: NoteSelectionMode::Toggle,
        })
    );
    assert!(widget.drag.is_none());
}

#[test]
fn piano_roll_selected_notes_paint_persistent_orange_borders() {
    let mut state = PianoRollState::default();
    state.apply_roll_message(PianoRollMessage::SelectNotes {
        ids: vec![2, 3],
        mode: NoteSelectionMode::Replace,
    });
    let widget = PianoRollWidget::new(
        state.notes,
        state.selected_note,
        state.selected_notes,
        state.selected_pitch,
        state.playhead_beat,
        state.viewport,
        state.tool,
    );
    let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(960.0, 390.0));
    let grid = widget.editor_rect(bounds);
    let mut primitives = Vec::new();

    widget.append_paint(
        &mut primitives,
        bounds,
        &LayoutOutput::default(),
        &ThemeTokens::default(),
    );

    for id in [2, 3] {
        let note = widget.note_by_id(id).expect("selected note should exist");
        let rect = widget.note_rect(grid, note);
        assert!(
            primitives.iter().any(|primitive| {
                matches!(
                    primitive,
                    PaintPrimitive::StrokeRect(stroke)
                        if stroke.color == ThemeTokens::default().highlight_orange
                            && stroke.width == 2.0
                            && stroke.rect == rect
                )
            }),
            "selected notes should keep a persistent orange border"
        );
    }
}

#[test]
fn piano_roll_plain_vertical_wheel_zooms_pitch_only() {
    let state = PianoRollState::default();
    let mut widget = PianoRollWidget::new(
        state.notes,
        state.selected_note,
        state.selected_notes,
        state.selected_pitch,
        state.playhead_beat,
        state.viewport,
        state.tool,
    );
    let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(960.0, 390.0));
    let grid = widget.editor_rect(bounds);

    let output = widget.handle_input(
        bounds,
        WidgetInput::Wheel {
            position: grid.center(),
            delta: Vector2::new(0.0, -20.0),
            modifiers: PointerModifiers::default(),
        },
    );

    assert_eq!(
        output.and_then(|output| output.typed_ref::<PianoRollMessage>().cloned()),
        Some(PianoRollMessage::ZoomViewport {
            time_factor: None,
            rows_delta: -2,
        })
    );

    let mut zoomed = PianoRollState::default();
    zoomed.apply_roll_message(PianoRollMessage::ZoomViewport {
        time_factor: None,
        rows_delta: -2,
    });

    assert_eq!(zoomed.viewport.visible_beats, TOTAL_BEATS);
    assert_eq!(zoomed.viewport.visible_pitches, 22);
}

#[test]
fn piano_roll_alt_vertical_wheel_zooms_time_only() {
    let state = PianoRollState::default();
    let mut widget = PianoRollWidget::new(
        state.notes.clone(),
        state.selected_note,
        state.selected_notes.clone(),
        state.selected_pitch,
        state.playhead_beat,
        state.viewport,
        state.tool,
    );
    let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(960.0, 390.0));
    let grid = widget.editor_rect(bounds);

    let output = widget.handle_input(
        bounds,
        WidgetInput::Wheel {
            position: grid.center(),
            delta: Vector2::new(0.0, -20.0),
            modifiers: PointerModifiers {
                alt: true,
                ..PointerModifiers::default()
            },
        },
    );

    assert!(matches!(
        output.and_then(|output| output.typed_ref::<PianoRollMessage>().cloned()),
        Some(PianoRollMessage::ZoomViewport {
            time_factor: Some(factor),
            rows_delta: 0
        }) if (factor - 0.8).abs() < f32::EPSILON
    ));

    let mut zoomed = PianoRollState::default();
    zoomed.apply_roll_message(PianoRollMessage::ZoomViewport {
        time_factor: Some(0.8),
        rows_delta: 0,
    });

    assert!((zoomed.viewport.visible_beats - 12.8).abs() < f32::EPSILON);
    assert_eq!(zoomed.viewport.visible_pitches, PITCH_ROWS);
}

#[test]
fn piano_roll_runtime_routes_mouse_wheel_to_viewport_zoom() {
    let bridge = piano_roll_test_bridge(PianoRollState::default());
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(1040.0, 620.0));
    let bounds = runtime.layout().rects[&PIANO_ROLL_WIDGET_ID];
    let initial_status = status_text(&runtime);

    assert!(runtime.wheel_or_scroll_at(bounds.center(), Vector2::new(0.0, -40.0)));

    let next_status = status_text(&runtime);
    assert_ne!(next_status, initial_status);
    assert!(
        next_status.contains("beats 0.0-16.0") && next_status.contains("pitches C#3-A#4"),
        "live wheel routing should reach the piano roll widget and zoom pitch only; got {next_status}"
    );
}

#[test]
fn piano_roll_runtime_routes_alt_mouse_wheel_to_time_zoom() {
    let bridge = piano_roll_test_bridge(PianoRollState::default());
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(1040.0, 620.0));
    let bounds = runtime.layout().rects[&PIANO_ROLL_WIDGET_ID];
    let initial_status = status_text(&runtime);

    assert!(runtime.wheel_or_scroll_at_with_modifiers(
        bounds.center(),
        Vector2::new(0.0, -40.0),
        PointerModifiers {
            alt: true,
            ..PointerModifiers::default()
        }
    ));

    let next_status = status_text(&runtime);
    assert_ne!(next_status, initial_status);
    assert!(
        next_status.contains("beats 1.6-14.4") && next_status.contains("pitches C3-B4"),
        "alt wheel routing should reach the piano roll widget and zoom time only; got {next_status}"
    );
}

#[test]
fn piano_roll_horizontal_wheel_still_pans_time_range() {
    let state = PianoRollState::default();
    let mut widget = PianoRollWidget::new(
        state.notes,
        state.selected_note,
        state.selected_notes,
        state.selected_pitch,
        state.playhead_beat,
        state.viewport,
        state.tool,
    );
    let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(960.0, 390.0));
    let grid = widget.editor_rect(bounds);

    let output = widget.handle_input(
        bounds,
        WidgetInput::Wheel {
            position: grid.center(),
            delta: Vector2::new(64.0, 0.0),
            modifiers: PointerModifiers::default(),
        },
    );

    assert!(matches!(
        output.and_then(|output| output.typed_ref::<PianoRollMessage>().cloned()),
        Some(PianoRollMessage::PanViewport {
            pitch_delta: 0,
            beat_delta
        }) if beat_delta > 0.0
    ));
}

#[test]
fn piano_roll_middle_mouse_drag_pans_view() {
    let mut state = PianoRollState::default();
    state.viewport.beat_start = 4.0;
    state.viewport.visible_beats = 8.0;
    state.viewport.pitch_start = 52;
    state.viewport.visible_pitches = 8;
    let mut widget = PianoRollWidget::new(
        state.notes,
        state.selected_note,
        state.selected_notes,
        state.selected_pitch,
        state.playhead_beat,
        state.viewport,
        state.tool,
    );
    let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(960.0, 390.0));
    let grid = widget.editor_rect(bounds);
    let start = grid.center();
    let end = Point::new(
        start.x - grid.width() * 0.125,
        start.y + row_height_for(grid, state.viewport) * 2.0,
    );

    let press = widget.handle_input(
        bounds,
        WidgetInput::PointerPress {
            position: start,
            button: PointerButton::Auxiliary,
            modifiers: PointerModifiers::default(),
        },
    );
    assert!(press.is_none());
    assert!(matches!(
        widget.drag,
        Some(super::drag::PianoDrag::Pan { .. })
    ));

    let output = widget.handle_input(bounds, WidgetInput::PointerMove { position: end });

    assert!(matches!(
        output.and_then(|output| output.typed_ref::<PianoRollMessage>().cloned()),
        Some(PianoRollMessage::PanViewport {
            beat_delta,
            pitch_delta
        }) if beat_delta > 0.0 && pitch_delta == 2
    ));
    let release = widget.handle_input(
        bounds,
        WidgetInput::PointerRelease {
            position: end,
            button: PointerButton::Auxiliary,
            modifiers: PointerModifiers::default(),
        },
    );
    assert!(release.is_none());
    assert!(widget.drag.is_none());
}

#[test]
fn piano_roll_middle_mouse_vertical_pan_accumulates_sub_row_motion() {
    let mut state = PianoRollState::default();
    state.viewport.visible_pitches = 8;
    state.viewport.pitch_start = 52;
    let mut widget = PianoRollWidget::new(
        state.notes,
        state.selected_note,
        state.selected_notes,
        state.selected_pitch,
        state.playhead_beat,
        state.viewport,
        state.tool,
    );
    let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(960.0, 390.0));
    let grid = widget.editor_rect(bounds);
    let row_height = row_height_for(grid, state.viewport);
    let start = grid.center();

    widget.handle_input(
        bounds,
        WidgetInput::PointerPress {
            position: start,
            button: PointerButton::Auxiliary,
            modifiers: PointerModifiers::default(),
        },
    );
    let first = widget.handle_input(
        bounds,
        WidgetInput::PointerMove {
            position: Point::new(start.x, start.y + row_height * 0.4),
        },
    );
    let second = widget.handle_input(
        bounds,
        WidgetInput::PointerMove {
            position: Point::new(start.x, start.y + row_height * 0.8),
        },
    );

    assert!(
        first.is_none(),
        "sub-row pan movement should wait until the accumulated drag reaches a row"
    );
    assert!(matches!(
        second.and_then(|output| output.typed_ref::<PianoRollMessage>().cloned()),
        Some(PianoRollMessage::PanViewport {
            beat_delta,
            pitch_delta: 1
        }) if beat_delta.abs() < f32::EPSILON
    ));
}

#[test]
fn piano_roll_stress_mode_generates_thousands_of_notes_for_marquee_selection() {
    let mut state = PianoRollState::default();

    state.apply_roll_message(PianoRollMessage::ToggleStressNotes);

    assert_eq!(state.notes.len(), STRESS_NOTE_COUNT);
    assert_eq!(state.tool, PianoRollTool::Select);
    assert!(state.selected_notes.is_empty());
    assert!(
        state.status().contains("stress 4096 notes"),
        "status should make the dense GUI stress load visible"
    );
}

#[test]
fn piano_roll_marquee_selects_thousands_of_notes_with_paint_only_preview() {
    let mut state = PianoRollState::default();
    state.apply_roll_message(PianoRollMessage::ToggleStressNotes);
    let mut widget = PianoRollWidget::new(
        state.notes.clone(),
        state.selected_note,
        state.selected_notes.clone(),
        state.selected_pitch,
        state.playhead_beat,
        state.viewport,
        state.tool,
    );
    let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(960.0, 390.0));
    let grid = widget.editor_rect(bounds);
    let start = Point::new(grid.min.x + 1.0, grid.min.y + 1.0);
    let end = Point::new(grid.min.x + grid.width() * 0.66, grid.max.y - 1.0);

    let press_output = widget.handle_input(
        bounds,
        WidgetInput::PointerPress {
            position: start,
            button: PointerButton::Primary,
            modifiers: PointerModifiers::default(),
        },
    );
    let move_output = widget.handle_input(bounds, WidgetInput::PointerMove { position: end });

    assert!(press_output.is_none());
    assert!(move_output.is_none());
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
        "marquee drag should paint a local selection rectangle"
    );

    let output = widget.handle_input(
        bounds,
        WidgetInput::PointerRelease {
            position: end,
            button: PointerButton::Primary,
            modifiers: PointerModifiers::default(),
        },
    );
    let message = output
        .and_then(|output| output.typed_ref::<PianoRollMessage>().cloned())
        .expect("marquee release should emit a selection message");

    match message {
        PianoRollMessage::SelectNotes { ids, mode } => {
            assert_eq!(mode, NoteSelectionMode::Replace);
            assert!(
                ids.len() > 2_000,
                "wide marquee should select thousands of dense synthetic notes"
            );
            state.apply_roll_message(PianoRollMessage::SelectNotes { ids, mode });
        }
        other => panic!("expected marquee selection message, got {other:?}"),
    }
    assert!(state.selected_notes.len() > 2_000);
}

#[test]
fn piano_roll_marquee_preview_lights_intersecting_notes_like_hover() {
    let state = PianoRollState::default();
    let mut widget = PianoRollWidget::new(
        state.notes,
        state.selected_note,
        state.selected_notes,
        state.selected_pitch,
        state.playhead_beat,
        state.viewport,
        PianoRollTool::Select,
    );
    let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(960.0, 390.0));
    let grid = widget.editor_rect(bounds);
    let note = widget.note_by_id(2).expect("default note should exist");
    let note_rect = widget.note_rect(grid, note);
    let start = Point::new(note_rect.min.x - 6.0, note_rect.min.y - 6.0);
    let end = Point::new(note_rect.max.x + 6.0, note_rect.max.y + 6.0);

    widget.handle_input(
        bounds,
        WidgetInput::PointerPress {
            position: start,
            button: PointerButton::Primary,
            modifiers: PointerModifiers::default(),
        },
    );
    widget.handle_input(bounds, WidgetInput::PointerMove { position: end });

    let mut overlay = Vec::new();
    widget.append_runtime_overlay_paint(
        &mut overlay,
        bounds,
        &LayoutOutput::default(),
        &ThemeTokens::default(),
    );

    assert!(
        overlay.iter().any(|primitive| {
            matches!(
                primitive,
                PaintPrimitive::StrokeRect(stroke)
                    if stroke.color == ThemeTokens::default().highlight_orange
                        && stroke.width == 2.0
                        && stroke.rect == note_rect
            )
        }),
        "notes intersecting the active marquee should use the orange hover-style highlight"
    );
}

#[test]
fn piano_roll_shift_drag_uses_marquee_selection_in_paint_tool() {
    let state = PianoRollState::default();
    assert_eq!(state.tool, PianoRollTool::Paint);
    let mut widget = PianoRollWidget::new(
        state.notes,
        state.selected_note,
        state.selected_notes,
        state.selected_pitch,
        state.playhead_beat,
        state.viewport,
        state.tool,
    );
    let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(960.0, 390.0));
    let grid = widget.editor_rect(bounds);
    let start = Point::new(grid.min.x + 1.0, grid.min.y + 1.0);
    let end = Point::new(grid.min.x + grid.width() * 0.33, grid.max.y - 1.0);
    let modifiers = PointerModifiers {
        shift: true,
        ..PointerModifiers::default()
    };

    let press = widget.handle_input(
        bounds,
        WidgetInput::PointerPress {
            position: start,
            button: PointerButton::Primary,
            modifiers,
        },
    );
    let move_output = widget.handle_input(bounds, WidgetInput::PointerMove { position: end });

    assert!(press.is_none());
    assert!(move_output.is_none());
    assert!(matches!(
        widget.drag,
        Some(super::drag::PianoDrag::Marquee { .. })
    ));
    let release = widget
        .handle_input(
            bounds,
            WidgetInput::PointerRelease {
                position: end,
                button: PointerButton::Primary,
                modifiers,
            },
        )
        .and_then(|output| output.typed_ref::<PianoRollMessage>().cloned())
        .expect("shift marquee release should emit selection");

    assert!(matches!(
        release,
        PianoRollMessage::SelectNotes {
            mode: NoteSelectionMode::Replace,
            ..
        }
    ));
}

#[test]
fn piano_roll_shift_command_marquee_adds_to_existing_selection() {
    let mut state = PianoRollState::default();
    state.apply_roll_message(PianoRollMessage::SelectNotes {
        ids: vec![2],
        mode: NoteSelectionMode::Replace,
    });
    let mut widget = PianoRollWidget::new(
        state.notes.clone(),
        state.selected_note,
        state.selected_notes.clone(),
        state.selected_pitch,
        state.playhead_beat,
        state.viewport,
        state.tool,
    );
    let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(960.0, 390.0));
    let grid = widget.editor_rect(bounds);
    let note = widget.note_by_id(3).expect("target note should exist");
    let note_rect = widget.note_rect(grid, note);
    let start = Point::new(note_rect.min.x - 2.0, note_rect.min.y - 2.0);
    let end = Point::new(note_rect.max.x + 2.0, note_rect.max.y + 2.0);
    let modifiers = PointerModifiers {
        shift: true,
        command: true,
        ..PointerModifiers::default()
    };

    widget.handle_input(
        bounds,
        WidgetInput::PointerPress {
            position: start,
            button: PointerButton::Primary,
            modifiers,
        },
    );
    widget.handle_input(bounds, WidgetInput::PointerMove { position: end });
    let release = widget
        .handle_input(
            bounds,
            WidgetInput::PointerRelease {
                position: end,
                button: PointerButton::Primary,
                modifiers,
            },
        )
        .and_then(|output| output.typed_ref::<PianoRollMessage>().cloned())
        .expect("shift+command marquee release should emit selection");

    match release {
        PianoRollMessage::SelectNotes { ids, mode } => {
            assert_eq!(mode, NoteSelectionMode::Add);
            assert_eq!(ids, vec![3]);
            state.apply_roll_message(PianoRollMessage::SelectNotes { ids, mode });
        }
        other => panic!("expected additive marquee selection, got {other:?}"),
    }

    assert_eq!(state.selected_notes, vec![2, 3]);
}

#[test]
fn piano_roll_velocity_lane_paints_dense_pillars_for_stress_notes() {
    let mut state = PianoRollState::default();
    state.apply_roll_message(PianoRollMessage::ToggleStressNotes);
    let widget = PianoRollWidget::new(
        state.notes,
        state.selected_note,
        state.selected_notes,
        state.selected_pitch,
        state.playhead_beat,
        state.viewport,
        state.tool,
    );
    let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(960.0, 390.0));
    let mut primitives = Vec::new();

    widget.append_paint(
        &mut primitives,
        bounds,
        &LayoutOutput::default(),
        &ThemeTokens::default(),
    );

    assert!(primitives.iter().any(
        |primitive| matches!(primitive, PaintPrimitive::Text(text) if text.text.as_str() == "Velocity")
    ));
    assert!(
        primitives
            .iter()
            .filter(|primitive| matches!(primitive, PaintPrimitive::FillRect(_)))
            .count()
            > STRESS_NOTE_COUNT,
        "dense velocity lane should add stem and handle primitives for synthetic notes"
    );
}

#[test]
fn piano_roll_velocity_pillars_align_to_note_start() {
    let state = PianoRollState::default();
    let widget = PianoRollWidget::new(
        state.notes.clone(),
        state.selected_note,
        state.selected_notes,
        state.selected_pitch,
        state.playhead_beat,
        state.viewport,
        state.tool,
    );
    let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(960.0, 390.0));
    let lane = widget.velocity_rect(bounds);
    let note = widget.note_by_id(2).expect("default note should exist");
    let stem = widget.velocity_preview_stem_rect(lane, note);
    let expected_x = x_for_beat_view(lane, state.viewport, note.start_beat);

    assert!(
        (stem.center().x - expected_x).abs() < f32::EPSILON,
        "velocity pillar should line up with the start of the note"
    );
}

#[test]
fn piano_roll_note_fill_alpha_tracks_velocity_with_visible_floor() {
    let mut state = PianoRollState::default();
    state.notes = vec![
        super::model::PianoNote {
            id: 101,
            pitch: 55,
            start_beat: 1.0,
            length_beats: 1.0,
            velocity: 0.0,
        },
        super::model::PianoNote {
            id: 102,
            pitch: 57,
            start_beat: 3.0,
            length_beats: 1.0,
            velocity: 1.0,
        },
    ];
    state.selected_note = None;
    state.selected_notes.clear();
    let widget = PianoRollWidget::new(
        state.notes.clone(),
        state.selected_note,
        state.selected_notes,
        state.selected_pitch,
        state.playhead_beat,
        state.viewport,
        state.tool,
    );
    let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(960.0, 390.0));
    let grid = widget.editor_rect(bounds);
    let quiet_rect = widget.note_rect(grid, state.notes[0]);
    let loud_rect = widget.note_rect(grid, state.notes[1]);
    let mut primitives = Vec::new();

    widget.append_paint(
        &mut primitives,
        bounds,
        &LayoutOutput::default(),
        &ThemeTokens::default(),
    );

    let quiet_alpha = fill_alpha_for_rect(&primitives, quiet_rect);
    let loud_alpha = fill_alpha_for_rect(&primitives, loud_rect);

    assert_eq!(quiet_alpha, 51);
    assert_eq!(loud_alpha, 255);
    assert!(
        loud_alpha > quiet_alpha,
        "higher velocity notes should paint more opaque than low velocity notes"
    );
}

#[test]
fn piano_roll_selected_note_fill_alpha_tracks_velocity_while_border_marks_selection() {
    let mut state = PianoRollState::default();
    state.notes = vec![super::model::PianoNote {
        id: 101,
        pitch: 55,
        start_beat: 1.0,
        length_beats: 1.0,
        velocity: 0.0,
    }];
    state.selected_note = Some(101);
    state.selected_notes = vec![101];
    let widget = PianoRollWidget::new(
        state.notes.clone(),
        state.selected_note,
        state.selected_notes,
        state.selected_pitch,
        state.playhead_beat,
        state.viewport,
        state.tool,
    );
    let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(960.0, 390.0));
    let grid = widget.editor_rect(bounds);
    let rect = widget.note_rect(grid, state.notes[0]);
    let mut primitives = Vec::new();

    widget.append_paint(
        &mut primitives,
        bounds,
        &LayoutOutput::default(),
        &ThemeTokens::default(),
    );

    assert_eq!(fill_alpha_for_rect(&primitives, rect), 51);
    assert!(
        primitives.iter().any(|primitive| {
            matches!(
                primitive,
                PaintPrimitive::StrokeRect(stroke)
                    if stroke.rect == rect
                        && stroke.color == ThemeTokens::default().highlight_orange
                        && stroke.width == 2.0
            )
        }),
        "selection should stay visible through the orange border even when low velocity dims the fill"
    );
}

#[test]
fn piano_roll_velocity_drag_edits_selected_notes_together() {
    let mut state = PianoRollState::default();
    state.apply_roll_message(PianoRollMessage::SelectNotes {
        ids: vec![2, 3],
        mode: NoteSelectionMode::Replace,
    });
    let mut widget = PianoRollWidget::new(
        state.notes.clone(),
        state.selected_note,
        state.selected_notes.clone(),
        state.selected_pitch,
        state.playhead_beat,
        state.viewport,
        state.tool,
    );
    let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(960.0, 390.0));
    let lane = widget.velocity_rect(bounds);
    let note = widget.note_by_id(2).expect("selected note should exist");
    let handle = widget.velocity_handle_rect(lane, note);
    let start = Point::new(handle.center().x, lane.min.y + 4.0);
    let end = Point::new(handle.center().x, lane.min.y + lane.height() * 0.72);

    let press = widget.handle_input(
        bounds,
        WidgetInput::PointerPress {
            position: start,
            button: PointerButton::Primary,
            modifiers: PointerModifiers::default(),
        },
    );
    assert!(
        press.is_none(),
        "velocity drag should not commit a state update on press"
    );

    let move_output = widget.handle_input(bounds, WidgetInput::PointerMove { position: end });
    assert!(move_output.is_none());
    let mut overlay = Vec::new();
    widget.append_runtime_overlay_paint(
        &mut overlay,
        bounds,
        &LayoutOutput::default(),
        &ThemeTokens::default(),
    );
    assert!(
        overlay.iter().any(|primitive| {
            matches!(
                primitive,
                PaintPrimitive::FillRect(fill)
                    if fill.color == paint::translucent(ThemeTokens::default().highlight_orange, 240)
            )
        }),
        "velocity drag should paint the edited bars locally before release"
    );
    let release = widget
        .handle_input(
            bounds,
            WidgetInput::PointerRelease {
                position: end,
                button: PointerButton::Primary,
                modifiers: PointerModifiers::default(),
            },
        )
        .and_then(|output| output.typed_ref::<PianoRollMessage>().cloned())
        .expect("velocity drag release should commit the edited value");

    match release {
        PianoRollMessage::SetVelocity { ids, velocity } => {
            assert_eq!(ids, vec![2, 3]);
            assert!(
                (velocity - 0.28).abs() < 0.02,
                "dragging lower in the velocity lane should reduce the linked selected notes"
            );
            state.apply_roll_message(PianoRollMessage::SetVelocity { ids, velocity });
        }
        other => panic!("expected velocity edit message, got {other:?}"),
    }

    assert!(
        state
            .notes
            .iter()
            .filter(|note| [2, 3].contains(&note.id))
            .all(|note| (note.velocity - 0.28).abs() < 0.02)
    );
}

#[test]
fn piano_roll_created_note_cuts_existing_note_into_left_and_right_fragments() {
    let mut state = PianoRollState::default();
    state.notes = vec![super::model::PianoNote {
        id: 1,
        pitch: 60,
        start_beat: 1.0,
        length_beats: 4.0,
        velocity: 0.8,
    }];
    state.selected_note = Some(1);

    state.apply_roll_message(PianoRollMessage::CreateNote {
        pitch: 60,
        start_beat: 2.0,
        length_beats: 1.0,
    });

    assert_eq!(state.notes.len(), 3);
    let mut ids = state.notes.iter().map(|note| note.id).collect::<Vec<_>>();
    ids.sort_unstable();
    ids.dedup();
    assert_eq!(ids.len(), 3);
    assert!(
        state
            .notes
            .iter()
            .any(|note| note.pitch == 60 && note.start_beat == 1.0 && note.length_beats == 1.0)
    );
    assert!(
        state
            .notes
            .iter()
            .any(|note| note.pitch == 60 && note.start_beat == 2.0 && note.length_beats == 1.0)
    );
    assert!(
        state
            .notes
            .iter()
            .any(|note| note.pitch == 60 && note.start_beat == 3.0 && note.length_beats == 2.0)
    );
}

#[test]
fn piano_roll_hover_uses_paint_only_runtime_overlay() {
    let state = PianoRollState::default();
    let mut widget = PianoRollWidget::new(
        state.notes,
        state.selected_note,
        state.selected_notes,
        state.selected_pitch,
        state.playhead_beat,
        state.viewport,
        state.tool,
    );
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
fn piano_roll_hover_lights_entire_note_tail() {
    let state = PianoRollState::default();
    let mut widget = PianoRollWidget::new(
        state.notes,
        state.selected_note,
        state.selected_notes,
        state.selected_pitch,
        state.playhead_beat,
        state.viewport,
        state.tool,
    );
    let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(960.0, 390.0));
    let grid = widget.editor_rect(bounds);
    let note = widget.note_by_id(2).expect("default note should exist");
    let note_rect = widget.note_rect(grid, note);

    widget.handle_input(
        bounds,
        WidgetInput::PointerMove {
            position: note_rect.center(),
        },
    );

    let mut overlay = Vec::new();
    widget.append_runtime_overlay_paint(
        &mut overlay,
        bounds,
        &LayoutOutput::default(),
        &ThemeTokens::default(),
    );

    assert!(
        overlay.iter().any(|primitive| {
            matches!(
                primitive,
                PaintPrimitive::FillRect(fill)
                    if fill.rect.min.x > note_rect.min.x
                        && (fill.rect.max.x - note_rect.max.x).abs() < f32::EPSILON
                        && fill.rect.min.y == note_rect.min.y
                        && fill.rect.max.y == note_rect.max.y
            )
        }),
        "hover should light the whole trailing body of the note"
    );
}

#[test]
fn piano_roll_hover_lights_left_keyboard_note_row() {
    let state = PianoRollState::default();
    let mut widget = PianoRollWidget::new(
        state.notes,
        state.selected_note,
        state.selected_notes,
        state.selected_pitch,
        state.playhead_beat,
        state.viewport,
        state.tool,
    );
    let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(960.0, 390.0));
    let keyboard = widget.keyboard_rect(bounds);
    let pitch = 60;
    let row = widget.keyboard_pitch_rect(keyboard, pitch);

    let output = widget.handle_input(
        bounds,
        WidgetInput::PointerMove {
            position: row.center(),
        },
    );

    assert!(output.is_none());
    assert_eq!(widget.hover_pitch, Some(pitch));
    assert!(widget.prefers_pointer_move_paint_only());
    let mut overlay = Vec::new();
    widget.append_runtime_overlay_paint(
        &mut overlay,
        bounds,
        &LayoutOutput::default(),
        &ThemeTokens::default(),
    );
    assert!(
        overlay.iter().any(|primitive| {
            matches!(
                primitive,
                PaintPrimitive::FillRect(fill)
                    if fill.color == paint::translucent(ThemeTokens::default().highlight_orange, 85)
                        && fill.rect.min.x == keyboard.min.x
                        && fill.rect.max.x == keyboard.max.x
                        && fill.rect.min.y == row.min.y
                        && fill.rect.max.y == row.max.y
            )
        }),
        "hovering the left keyboard should light the current piano key row"
    );
}

#[test]
fn piano_roll_keyboard_press_lights_pitch_lane_and_selects_pitch() {
    let mut state = PianoRollState::default();
    let mut widget = PianoRollWidget::new(
        state.notes.clone(),
        state.selected_note,
        state.selected_notes.clone(),
        state.selected_pitch,
        state.playhead_beat,
        state.viewport,
        state.tool,
    );
    let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(960.0, 390.0));
    let keyboard = widget.keyboard_rect(bounds);
    let grid = widget.editor_rect(bounds);
    let pitch = 60;
    let row = widget.keyboard_pitch_rect(keyboard, pitch);

    let output = widget.handle_input(
        bounds,
        WidgetInput::PointerPress {
            position: row.center(),
            button: PointerButton::Primary,
            modifiers: PointerModifiers::default(),
        },
    );

    assert_eq!(widget.active_pitch, Some(pitch));
    assert_eq!(
        output.and_then(|output| output.typed_ref::<PianoRollMessage>().cloned()),
        Some(PianoRollMessage::SelectPitch(pitch))
    );
    let mut overlay = Vec::new();
    widget.append_runtime_overlay_paint(
        &mut overlay,
        bounds,
        &LayoutOutput::default(),
        &ThemeTokens::default(),
    );
    let lane = widget.keyboard_pitch_rect(grid, pitch);
    assert!(
        overlay.iter().any(|primitive| {
            matches!(
                primitive,
                PaintPrimitive::FillRect(fill)
                    if fill.color == paint::translucent(ThemeTokens::default().highlight_orange, 72)
                        && fill.rect.min.y == lane.min.y
                        && fill.rect.max.y == lane.max.y
            )
        }),
        "pressing a piano key should immediately light the matching editor lane"
    );
    let release = widget.handle_input(
        bounds,
        WidgetInput::PointerRelease {
            position: row.center(),
            button: PointerButton::Primary,
            modifiers: PointerModifiers::default(),
        },
    );
    assert!(release.is_none());
    assert_eq!(widget.active_pitch, None);
    assert_eq!(widget.hover_pitch, Some(pitch));

    state.apply_roll_message(PianoRollMessage::SelectPitch(pitch));
    assert_eq!(state.selected_pitch, Some(pitch));
    let selected_widget = PianoRollWidget::new(
        state.notes,
        state.selected_note,
        state.selected_notes,
        state.selected_pitch,
        state.playhead_beat,
        state.viewport,
        state.tool,
    );
    let mut primitives = Vec::new();
    selected_widget.append_paint(
        &mut primitives,
        bounds,
        &LayoutOutput::default(),
        &ThemeTokens::default(),
    );
    assert!(
        primitives.iter().any(|primitive| {
            matches!(
                primitive,
                PaintPrimitive::FillRect(fill)
                    if fill.color == paint::translucent(ThemeTokens::default().highlight_blue, 30)
                        && fill.rect.min.y == lane.min.y
                        && fill.rect.max.y == lane.max.y
            )
        }),
        "selected piano key should leave a persistent lane accent"
    );
}

#[test]
fn piano_roll_grid_hover_lights_matching_left_keyboard_note_row() {
    let state = PianoRollState::default();
    let mut widget = PianoRollWidget::new(
        state.notes,
        state.selected_note,
        state.selected_notes,
        state.selected_pitch,
        state.playhead_beat,
        state.viewport,
        state.tool,
    );
    let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(960.0, 390.0));
    let grid = widget.editor_rect(bounds);
    let keyboard = widget.keyboard_rect(bounds);
    let pitch = 57;
    let y =
        y_for_pitch_view(grid, state.viewport, pitch) + row_height_for(grid, state.viewport) * 0.5;

    widget.handle_input(
        bounds,
        WidgetInput::PointerMove {
            position: Point::new(grid.center().x, y),
        },
    );

    assert_eq!(widget.hover_pitch, Some(pitch));
    let mut overlay = Vec::new();
    widget.append_runtime_overlay_paint(
        &mut overlay,
        bounds,
        &LayoutOutput::default(),
        &ThemeTokens::default(),
    );
    let row = widget.keyboard_pitch_rect(keyboard, pitch);
    assert!(
        overlay.iter().any(|primitive| {
            matches!(
                primitive,
                PaintPrimitive::FillRect(fill)
                    if fill.color == paint::translucent(ThemeTokens::default().highlight_orange, 85)
                        && fill.rect.min.y == row.min.y
                        && fill.rect.max.y == row.max.y
            )
        }),
        "hovering the grid should light the matching key on the left piano visual"
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

fn fill_alpha_for_rect(primitives: &[PaintPrimitive], rect: Rect) -> u8 {
    primitives
        .iter()
        .find_map(|primitive| match primitive {
            PaintPrimitive::FillRect(fill) if fill.rect == rect => Some(fill.color.a),
            _ => None,
        })
        .expect("fill primitive for note rect should be painted")
}
