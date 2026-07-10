use radiant::prelude::*;

use super::{
    AppMessage, DATA_SOURCE_NOTE, PIANO_ROLL_WIDGET_ID, PianoRollMessage, PianoRollState,
    PianoRollTool, PianoRollWidget, PianoRollWidgetParts, STATUS_WIDGET_ID,
};

pub(crate) fn project_surface(state: &PianoRollState) -> View<AppMessage> {
    column([
        header_row(state),
        custom_widget_mapped(
            PianoRollWidget::new(PianoRollWidgetParts::from_state(state)),
            AppMessage::Roll,
        )
        .id(PIANO_ROLL_WIDGET_ID)
        .height(390.0)
        .fill_width(),
        status_row(state),
    ])
    .style(WidgetStyle::default())
    .padding(16.0)
    .spacing(12.0)
    .fill()
}

fn header_row(state: &PianoRollState) -> View<AppMessage> {
    row([
        text("Piano Roll").height(30.0).fill_width(),
        button("Paint")
            .subtle()
            .message(AppMessage::Roll(PianoRollMessage::SetTool(
                PianoRollTool::Paint,
            )))
            .size(62.0, 30.0),
        button("Select")
            .subtle()
            .message(AppMessage::Roll(PianoRollMessage::SetTool(
                PianoRollTool::Select,
            )))
            .size(68.0, 30.0),
        button(if state.notes.len() > 1000 {
            "Normal"
        } else {
            "4k Notes"
        })
        .subtle()
        .message(AppMessage::Roll(PianoRollMessage::ToggleStressNotes))
        .size(86.0, 30.0),
        button(if state.snap_enabled {
            "Snap On"
        } else {
            "Snap Off"
        })
        .subtle()
        .message(AppMessage::Roll(PianoRollMessage::ToggleSnap))
        .size(86.0, 30.0),
        button("Undo")
            .subtle()
            .message(AppMessage::Undo)
            .size(62.0, 30.0),
        button("Redo")
            .subtle()
            .message(AppMessage::Redo)
            .size(62.0, 30.0),
        button("H-")
            .subtle()
            .message(AppMessage::Roll(PianoRollMessage::ZoomTime {
                factor: 1.25,
            }))
            .size(42.0, 30.0),
        button("H+")
            .subtle()
            .message(AppMessage::Roll(PianoRollMessage::ZoomTime {
                factor: 0.80,
            }))
            .size(42.0, 30.0),
        button("V-")
            .subtle()
            .message(AppMessage::Roll(PianoRollMessage::ZoomPitch {
                rows_delta: 2,
            }))
            .size(42.0, 30.0),
        button("V+")
            .subtle()
            .message(AppMessage::Roll(PianoRollMessage::ZoomPitch {
                rows_delta: -2,
            }))
            .size(42.0, 30.0),
        button("<")
            .subtle()
            .message(AppMessage::Roll(PianoRollMessage::PanViewport {
                beat_delta: -1.0,
                pitch_delta: 0,
            }))
            .size(34.0, 30.0),
        button(">")
            .subtle()
            .message(AppMessage::Roll(PianoRollMessage::PanViewport {
                beat_delta: 1.0,
                pitch_delta: 0,
            }))
            .size(34.0, 30.0),
        button("^")
            .subtle()
            .message(AppMessage::Roll(PianoRollMessage::PanViewport {
                beat_delta: 0.0,
                pitch_delta: 1,
            }))
            .size(34.0, 30.0),
        button("v")
            .subtle()
            .message(AppMessage::Roll(PianoRollMessage::PanViewport {
                beat_delta: 0.0,
                pitch_delta: -1,
            }))
            .size(34.0, 30.0),
        button(if state.running { "Pause" } else { "Run" })
            .primary()
            .message(AppMessage::ToggleRun)
            .size(88.0, 30.0),
        button("Reset")
            .subtle()
            .message(AppMessage::Reset)
            .size(82.0, 30.0),
    ])
    .fill_width()
    .spacing(10.0)
}

fn status_row(state: &PianoRollState) -> View<AppMessage> {
    row([
        stat_tile("Notes", state.notes.len().to_string()),
        stat_tile("Selected", state.selected_notes.len().to_string()),
        stat_tile(
            "Grid",
            if state.snap_enabled {
                "Snap 1/4"
            } else {
                "Free"
            },
        ),
        stat_tile(
            "History",
            format!(
                "U{} / R{}",
                state.history.undo_len(),
                state.history.redo_len()
            ),
        ),
        stat_tile(
            "Range",
            format!(
                "{:.1}-{:.1} / {} rows",
                state.viewport.beat_start,
                state.viewport.beat_end(),
                state.viewport.visible_pitches
            ),
        ),
        stat_tile("Source", DATA_SOURCE_NOTE),
        text(state.status())
            .id(STATUS_WIDGET_ID)
            .height(68.0)
            .fill_width(),
    ])
    .fill_width()
    .spacing(10.0)
}

fn stat_tile(label: impl Into<String>, value: impl Into<String>) -> View<AppMessage> {
    column([
        text(label.into()).height(22.0).fill_width(),
        text(value.into()).height(24.0).fill_width(),
    ])
    .style(WidgetStyle {
        tone: WidgetTone::Neutral,
        prominence: WidgetProminence::Subtle,
    })
    .padding(10.0)
    .spacing(4.0)
    .height(68.0)
    .fill_width()
}
