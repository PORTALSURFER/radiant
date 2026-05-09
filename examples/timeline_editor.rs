//! Timeline-editor-style sandbox for generic visualization state.

use radiant::gui::{
    range::NormalizedRange,
    visualization::{
        ChannelViewMode, SignalChromeState, SignalRasterPreview, SignalToolState,
        TimelineEditPreview, TimelineFeedbackEvents, TimelineMarkerPreview, TimelineMotionState,
        TimelinePresentationState, TimelineSurfaceState, TimelineTransportState, TimelineViewport,
    },
};
use radiant::prelude::*;

#[derive(Clone, Debug)]
struct TimelineEditorState {
    playing: bool,
    repeat_enabled: bool,
    playhead_micros: u32,
    selected_marker: &'static str,
    revision: u64,
    feedback_nonce: u64,
}

impl Default for TimelineEditorState {
    fn default() -> Self {
        Self {
            playing: false,
            repeat_enabled: true,
            playhead_micros: 320_000,
            selected_marker: "intro",
            revision: 1,
            feedback_nonce: 0,
        }
    }
}

fn main() -> radiant::Result {
    radiant::app(TimelineEditorState::default())
        .title("Radiant Timeline Editor")
        .size(760, 420)
        .min_size(560, 320)
        .view(project_surface)
        .run()
}

fn project_surface(state: &mut TimelineEditorState) -> StateView<TimelineEditorState> {
    let timeline = timeline_surface(state);

    column([
        row([
            text("Timeline Editor").height(30.0).fill_width(),
            toggle("Repeat", timeline.surface.presentation.repeat_enabled)
                .on_change(|state: &mut TimelineEditorState, enabled| {
                    state.repeat_enabled = enabled;
                    state.revision += 1;
                })
                .size(102.0, 30.0),
            button(if state.playing { "Pause" } else { "Play" })
                .primary()
                .on_click(|state: &mut TimelineEditorState| {
                    state.playing = !state.playing;
                    state.feedback_nonce += 1;
                })
                .size(84.0, 32.0),
        ])
        .fill_width()
        .spacing(10.0),
        stack([
            retained_canvas(1_400)
                .revision(timeline.surface.raster_preview.image_signature.unwrap_or(0))
                .dirty_mask(3)
                .view()
                .id(20)
                .fill(),
            overlay_panel("selection", 168.0, 58.0, 205.0, 94.0).id(21),
            drop_marker(playhead_x(timeline.surface.transport), 34.0, 3.0, 156.0).id(22),
            column([
                text(timeline_label(&timeline)).height(24.0).fill_width(),
                row([
                    marker_chip(100, "intro", "Intro", state.selected_marker == "intro"),
                    marker_chip(110, "build", "Build", state.selected_marker == "build"),
                    marker_chip(120, "outro", "Outro", state.selected_marker == "outro"),
                ])
                .spacing(10.0)
                .fill_width(),
            ])
            .padding(18.0)
            .spacing(82.0)
            .fill(),
        ])
        .style(WidgetStyle::default())
        .height(212.0)
        .fill_width(),
        row([
            button("Step -")
                .subtle()
                .on_click(|state: &mut TimelineEditorState| step_playhead(state, -40_000))
                .id(30)
                .size(84.0, 30.0),
            button("Step +")
                .subtle()
                .on_click(|state: &mut TimelineEditorState| step_playhead(state, 40_000))
                .id(31)
                .size(84.0, 30.0),
            text(format!(
                "marker={} playhead={}us feedback={}",
                state.selected_marker, state.playhead_micros, state.feedback_nonce
            ))
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

fn timeline_surface(state: &TimelineEditorState) -> TimelineMotionState {
    let selection = NormalizedRange::from_micros(180_000, 420_000);
    let surface = TimelineSurfaceState::new(
        TimelineViewport::new(0, 850, 0, 850_000, 0, 850_000_000),
        TimelineTransportState::new(
            Some((state.playhead_micros / 1_000) as u16),
            Some((state.playhead_micros / 1_000) as u16),
            Some(state.playhead_micros),
            Some(selection),
        ),
        TimelineEditPreview::new(
            Some(selection),
            Some(220),
            Some(220_000),
            Some(185),
            Some(185_000),
            Some(420),
            Some(390),
            Some(390_000),
            Some(415),
            Some(415_000),
            Some(620),
        ),
        TimelineFeedbackEvents::new(state.feedback_nonce, 0, state.revision),
        TimelinePresentationState::new(
            Some(100_000),
            0,
            state.repeat_enabled,
            Some("Arrangement".to_string()),
            Some("0% - 85%".to_string()),
        ),
        SignalRasterPreview::new(
            Some("cached timeline atlas".to_string()),
            false,
            false,
            Some(state.revision),
            None,
        ),
        vec![
            marker(80, 165, state.selected_marker == "intro"),
            marker(420, 540, state.selected_marker == "build"),
            marker(720, 810, state.selected_marker == "outro"),
        ],
    );

    TimelineMotionState::new(
        state.playing,
        surface,
        SignalChromeState::new(
            if state.playing { "playing" } else { "idle" },
            true,
            Some("bar 1".to_string()),
            ChannelViewMode::Stereo,
        ),
        SignalToolState::new(false, false, true, true, false, true, true, true),
    )
}

fn marker(start_milli: u16, end_milli: u16, selected: bool) -> TimelineMarkerPreview {
    TimelineMarkerPreview {
        range: NormalizedRange::new(start_milli, end_milli),
        selected,
        focused: selected,
    }
}

fn marker_chip(
    widget_id: u64,
    marker_id: &'static str,
    label: &'static str,
    selected: bool,
) -> StateView<TimelineEditorState> {
    selectable(label, selected)
        .on_change(move |state: &mut TimelineEditorState, selected| {
            if selected {
                state.selected_marker = marker_id;
                state.revision += 1;
            }
        })
        .id(widget_id)
        .size(104.0, 32.0)
}

fn playhead_x(transport: TimelineTransportState) -> f32 {
    let micros = transport.resolved_playhead_micros().unwrap_or(0);
    40.0 + (micros as f32 / 1_000_000.0) * 620.0
}

fn step_playhead(state: &mut TimelineEditorState, delta: i32) {
    state.playhead_micros = state
        .playhead_micros
        .saturating_add_signed(delta)
        .clamp(0, 850_000);
    state.revision += 1;
}

fn timeline_label(timeline: &TimelineMotionState) -> String {
    let label = timeline
        .surface
        .presentation
        .viewport_label
        .as_deref()
        .unwrap_or("viewport");
    format!(
        "{} / {} / markers {}",
        timeline.chrome.status_hint,
        label,
        timeline.surface.markers.len()
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use radiant::{
        layout::{Point, Vector2},
        runtime::SurfaceRuntime,
        widgets::{ButtonMessage, PointerButton, WidgetInput, WidgetOutput},
    };

    #[test]
    fn timeline_editor_projects_generic_timeline_state() {
        let state = TimelineEditorState::default();
        let timeline = timeline_surface(&state);

        assert_eq!(timeline.surface.markers.len(), 3);
        assert_eq!(
            timeline.surface.transport.resolved_playhead_micros(),
            Some(320_000)
        );
        assert_eq!(timeline.chrome.channel_view, ChannelViewMode::Stereo);
    }

    #[test]
    fn timeline_editor_routes_controls_through_runtime() {
        let bridge = radiant::app(TimelineEditorState::default())
            .view(project_surface)
            .into_bridge();
        let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(760.0, 420.0));

        assert!(runtime.surface().find_widget(20).is_some());
        assert!(runtime.surface().keyboard_focus_order().contains(&100));

        let stepped = runtime
            .surface()
            .dispatch_widget_output(31, WidgetOutput::typed(ButtonMessage::Activate))
            .expect("step button should emit a state action");
        let command = runtime.dispatch_message(stepped);
        let selected = runtime.dispatch_input(
            110,
            WidgetInput::PointerPress {
                position: Point::new(48.0, 48.0),
                button: PointerButton::Primary,
            },
        );

        assert!(command.repaint_requested);
        assert!(selected);
    }
}
