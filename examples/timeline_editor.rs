//! Arrangement-style timeline sandbox for generic visualization state.

#[path = "timeline_editor/widget.rs"]
mod timeline_widget;

#[path = "timeline_editor/model.rs"]
mod model;

use radiant::prelude::*;

use model::*;
use timeline_widget::ArrangementTimelineWidget;

const TIMELINE_WIDGET_ID: u64 = 20;
const STATUS_WIDGET_ID: u64 = 500;
const TOTAL_BEATS: u32 = 64;
const LANE_COUNT: usize = 4;
const MIN_CLIP_BEATS: u32 = 2;
const CLIP_HEIGHT: f32 = 30.0;
const HEADER_WIDTH: f32 = 112.0;
const RULER_HEIGHT: f32 = 30.0;
const LANE_HEIGHT: f32 = 48.0;
const TRACK_PAD: f32 = 12.0;
const RESIZE_HANDLE_WIDTH: f32 = 7.0;

fn main() -> radiant::Result {
    radiant::app(TimelineEditorState::default())
        .title("Radiant Timeline Editor")
        .size(860, 460)
        .min_size(620, 360)
        .view(project_surface)
        .update(update)
        .run()
}

fn project_surface(state: &mut TimelineEditorState) -> View<TimelineMessage> {
    let timeline = timeline_surface(state);

    column([
        row([
            text("Arrangement").height(30.0).fill_width(),
            toggle("Repeat", timeline.surface.presentation.repeat_enabled)
                .message(TimelineMessage::ToggleRepeat)
                .size(102.0, 30.0),
            button(if state.playback.playing {
                "Pause"
            } else {
                "Play"
            })
            .primary()
            .message(TimelineMessage::TogglePlay)
            .size(84.0, 32.0),
        ])
        .fill_width()
        .spacing(10.0),
        stack([
            retained_canvas(1_400)
                .revision(timeline.surface.raster_preview.image_signature.unwrap_or(0))
                .dirty_mask(3)
                .view()
                .id(18)
                .fill(),
            custom_widget_mapped(
                ArrangementTimelineWidget::new(state),
                TimelineMessage::Surface,
            )
            .id(TIMELINE_WIDGET_ID)
            .fill(),
        ])
        .style(WidgetStyle::default())
        .height(252.0)
        .fill_width(),
        row([
            button("Rewind")
                .subtle()
                .message(TimelineMessage::Rewind)
                .id(30)
                .size(84.0, 30.0),
            button("Duplicate")
                .subtle()
                .message(TimelineMessage::DuplicateSelection)
                .id(31)
                .size(108.0, 30.0),
            button("Delete")
                .danger()
                .message(TimelineMessage::DeleteSelection)
                .id(32)
                .size(84.0, 30.0),
            text(timeline_label(state, &timeline))
                .id(STATUS_WIDGET_ID)
                .height(30.0)
                .fill_width(),
        ])
        .fill_width()
        .spacing(10.0),
    ])
    .style(WidgetStyle::default())
    .padding(16.0)
    .spacing(12.0)
    .fill()
}

#[cfg(test)]
#[path = "timeline_editor/tests.rs"]
mod tests;
