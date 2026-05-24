//! Arrangement shell sandbox for DAW-style multi-pane GUI composition.

use radiant::prelude::*;
use radiant::{
    runtime::{PaintFillRect, PaintStrokeRect},
    widgets::PaintBounds,
};

const ARRANGEMENT_WIDGET_ID: u64 = 96;
const STATUS_WIDGET_ID: u64 = 97;
const TRACK_COUNT: usize = 5;
const TOTAL_BEATS: f32 = 32.0;
const DATA_SOURCE_NOTE: &str = "without_audio_or_dsp";

#[derive(Clone, Debug)]
struct ArrangementShellState {
    running: bool,
    frame: u64,
    playhead_beat: f32,
    selected_track: usize,
    selected_clip: Option<u32>,
    browser_open: bool,
    inspector_open: bool,
    clips: Vec<ArrangementClip>,
    mixer: [TrackMeter; TRACK_COUNT],
}

impl Default for ArrangementShellState {
    fn default() -> Self {
        let mut state = Self {
            running: true,
            frame: 0,
            playhead_beat: 0.0,
            selected_track: 1,
            selected_clip: Some(2),
            browser_open: true,
            inspector_open: true,
            clips: vec![
                ArrangementClip::new(1, 0, 0.0, 4.0, "Intro"),
                ArrangementClip::new(2, 1, 2.0, 6.0, "Bass A"),
                ArrangementClip::new(3, 2, 4.0, 4.0, "Keys A"),
                ArrangementClip::new(4, 3, 8.0, 8.0, "Pad Rise"),
                ArrangementClip::new(5, 4, 10.0, 4.0, "Lead"),
                ArrangementClip::new(6, 1, 14.0, 8.0, "Bass B"),
                ArrangementClip::new(7, 2, 16.0, 6.0, "Keys B"),
                ArrangementClip::new(8, 0, 24.0, 4.0, "Outro"),
            ],
            mixer: std::array::from_fn(TrackMeter::new),
        };
        state.tick();
        state
    }
}

impl ArrangementShellState {
    fn tick(&mut self) {
        if !self.running {
            return;
        }
        self.frame = self.frame.saturating_add(1);
        self.playhead_beat = (self.playhead_beat + 0.045) % TOTAL_BEATS;
        for meter in &mut self.mixer {
            meter.tick(self.frame);
        }
    }

    fn reset(&mut self) {
        *self = Self::default();
    }

    fn status(&self) -> String {
        let selected = self
            .selected_clip
            .and_then(|id| self.clips.iter().find(|clip| clip.id == id))
            .map(|clip| {
                format!(
                    "{} on {} beat {:.1}",
                    clip.label, TRACKS[clip.track], clip.start_beat
                )
            })
            .unwrap_or_else(|| format!("track {}", TRACKS[self.selected_track]));
        let transport = if self.running { "running" } else { "paused" };
        format!(
            "{transport} | frame {} | playhead {:.2} | {selected} | synthetic GUI data",
            self.frame, self.playhead_beat
        )
    }

    fn selected_clip(&self) -> Option<ArrangementClip> {
        self.selected_clip
            .and_then(|id| self.clips.iter().copied().find(|clip| clip.id == id))
    }

    fn apply_shell_message(&mut self, message: ShellMessage) {
        match message {
            ShellMessage::SelectTrack(track) => {
                self.selected_track = track.min(TRACK_COUNT - 1);
                self.selected_clip = self
                    .clips
                    .iter()
                    .find(|clip| clip.track == self.selected_track)
                    .map(|clip| clip.id);
            }
            ShellMessage::SelectClip(id) => {
                if let Some(clip) = self.clips.iter().find(|clip| clip.id == id) {
                    self.selected_track = clip.track;
                    self.selected_clip = Some(id);
                }
            }
            ShellMessage::Seek { beat } => {
                self.playhead_beat = beat.clamp(0.0, TOTAL_BEATS);
            }
            ShellMessage::ToggleBrowser => {
                self.browser_open = !self.browser_open;
            }
            ShellMessage::ToggleInspector => {
                self.inspector_open = !self.inspector_open;
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct ArrangementClip {
    id: u32,
    track: usize,
    start_beat: f32,
    length_beats: f32,
    label: &'static str,
}

impl ArrangementClip {
    const fn new(
        id: u32,
        track: usize,
        start_beat: f32,
        length_beats: f32,
        label: &'static str,
    ) -> Self {
        Self {
            id,
            track,
            start_beat,
            length_beats,
            label,
        }
    }

    fn end_beat(self) -> f32 {
        self.start_beat + self.length_beats
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct TrackMeter {
    track: usize,
    level: f32,
    peak: f32,
}

impl TrackMeter {
    fn new(track: usize) -> Self {
        Self {
            track,
            level: 0.0,
            peak: 0.0,
        }
    }

    fn tick(&mut self, frame: u64) {
        let phase = frame as f32 * (0.030 + self.track as f32 * 0.006);
        let pulse = (phase.sin() * 0.5 + 0.5).powf(1.8);
        let accent = if (frame + self.track as u64 * 9) % (42 + self.track as u64 * 4) < 5 {
            0.35
        } else {
            0.0
        };
        let target = (0.10 + pulse * 0.62 + accent).min(1.0);
        self.level = self.level * 0.70 + target * 0.30;
        self.peak = if target > self.peak {
            target
        } else {
            (self.peak - 0.012).max(self.level)
        };
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum AppMessage {
    Frame,
    ToggleRun,
    Reset,
    Shell(ShellMessage),
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum ShellMessage {
    SelectTrack(usize),
    SelectClip(u32),
    Seek { beat: f32 },
    ToggleBrowser,
    ToggleInspector,
}

const TRACKS: [&str; TRACK_COUNT] = ["Drums", "Bass", "Keys", "Pads", "Lead"];
const BROWSER_ITEMS: [&str; 6] = [
    "Kick Loop",
    "Snare Fill",
    "Sub Bass",
    "Chord Stab",
    "Pad Bed",
    "Lead Hook",
];

fn main() -> radiant::Result {
    radiant::app(ArrangementShellState::default())
        .title("Radiant Arrangement Shell")
        .size(1180, 700)
        .min_size(900, 560)
        .view(project_surface)
        .animation(|state| state.running)
        .on_frame(|| AppMessage::Frame)
        .update(update)
        .run()
}

fn project_surface(state: &mut ArrangementShellState) -> View<AppMessage> {
    column([
        transport_bar(state),
        row([
            browser_panel(state),
            arrangement_panel(state),
            inspector_panel(state),
        ])
        .fill()
        .spacing(10.0),
        mixer_strip(state),
    ])
    .style(WidgetStyle::default())
    .padding(14.0)
    .spacing(10.0)
    .fill()
}

fn transport_bar(state: &ArrangementShellState) -> View<AppMessage> {
    row([
        text("Arrangement Shell").height(30.0).fill_width(),
        button(if state.browser_open {
            "Hide Browser"
        } else {
            "Show Browser"
        })
        .subtle()
        .message(AppMessage::Shell(ShellMessage::ToggleBrowser))
        .size(126.0, 30.0),
        button(if state.inspector_open {
            "Hide Inspector"
        } else {
            "Show Inspector"
        })
        .subtle()
        .message(AppMessage::Shell(ShellMessage::ToggleInspector))
        .size(136.0, 30.0),
        button(if state.running { "Pause" } else { "Run" })
            .primary()
            .message(AppMessage::ToggleRun)
            .size(86.0, 30.0),
        button("Reset")
            .subtle()
            .message(AppMessage::Reset)
            .size(78.0, 30.0),
    ])
    .fill_width()
    .spacing(10.0)
}

fn browser_panel(state: &ArrangementShellState) -> View<AppMessage> {
    if !state.browser_open {
        return column([text("Browser").height(24.0).fill_width()])
            .style(panel_style())
            .padding(10.0)
            .size(92.0, 400.0);
    }
    let mut rows = Vec::new();
    rows.push(text("Browser").height(24.0).fill_width());
    for item in BROWSER_ITEMS {
        rows.push(text(item).height(26.0).fill_width().style(WidgetStyle {
            tone: WidgetTone::Neutral,
            prominence: WidgetProminence::Subtle,
        }));
    }
    column(rows)
        .style(panel_style())
        .padding(10.0)
        .spacing(6.0)
        .size(168.0, 400.0)
}

fn arrangement_panel(state: &ArrangementShellState) -> View<AppMessage> {
    column([
        row(TRACKS
            .iter()
            .enumerate()
            .map(|(track, label)| {
                button(*label)
                    .style(if track == state.selected_track {
                        WidgetStyle {
                            tone: WidgetTone::Accent,
                            prominence: WidgetProminence::Subtle,
                        }
                    } else {
                        WidgetStyle {
                            tone: WidgetTone::Neutral,
                            prominence: WidgetProminence::Subtle,
                        }
                    })
                    .message(AppMessage::Shell(ShellMessage::SelectTrack(track)))
                    .height(28.0)
                    .fill_width()
            })
            .collect::<Vec<_>>())
        .fill_width()
        .spacing(8.0),
        custom_widget_mapped(
            ArrangementOverviewWidget::new(
                state.clips.clone(),
                state.selected_clip,
                state.playhead_beat,
            ),
            AppMessage::Shell,
        )
        .id(ARRANGEMENT_WIDGET_ID)
        .height(390.0)
        .fill_width(),
    ])
    .style(panel_style())
    .padding(10.0)
    .spacing(10.0)
    .fill()
}

fn inspector_panel(state: &ArrangementShellState) -> View<AppMessage> {
    if !state.inspector_open {
        return column([text("Inspector").height(24.0).fill_width()])
            .style(panel_style())
            .padding(10.0)
            .size(104.0, 400.0);
    }
    let selected = state.selected_clip();
    column([
        text("Inspector").height(24.0).fill_width(),
        stat_tile(
            "Selected",
            selected.map(|clip| clip.label).unwrap_or("Track"),
        ),
        stat_tile(
            "Track",
            selected
                .map(|clip| TRACKS[clip.track])
                .unwrap_or(TRACKS[state.selected_track]),
        ),
        stat_tile(
            "Beat",
            selected
                .map(|clip| format!("{:.1} - {:.1}", clip.start_beat, clip.end_beat()))
                .unwrap_or_else(|| format!("{:.1}", state.playhead_beat)),
        ),
        stat_tile("Source", DATA_SOURCE_NOTE),
    ])
    .style(panel_style())
    .padding(10.0)
    .spacing(8.0)
    .size(196.0, 400.0)
}

fn mixer_strip(state: &ArrangementShellState) -> View<AppMessage> {
    let mut tracks = Vec::new();
    tracks.push(
        text(state.status())
            .id(STATUS_WIDGET_ID)
            .height(64.0)
            .fill_width(),
    );
    for meter in state.mixer {
        tracks.push(meter_tile(TRACKS[meter.track], meter.level, meter.peak));
    }
    row(tracks).fill_width().spacing(10.0)
}

fn meter_tile(label: impl Into<String>, level: f32, peak: f32) -> View<AppMessage> {
    column([
        text(label.into()).height(20.0).fill_width(),
        text(format!(
            "lvl {:>3}% pk {:>3}%",
            (level * 100.0) as u32,
            (peak * 100.0) as u32
        ))
        .height(22.0)
        .fill_width(),
    ])
    .style(panel_style())
    .padding(10.0)
    .height(64.0)
    .fill_width()
}

fn stat_tile(label: impl Into<String>, value: impl Into<String>) -> View<AppMessage> {
    column([
        text(label.into()).height(20.0).fill_width(),
        text(value.into()).height(22.0).fill_width(),
    ])
    .style(WidgetStyle {
        tone: WidgetTone::Neutral,
        prominence: WidgetProminence::Subtle,
    })
    .padding(8.0)
    .height(58.0)
    .fill_width()
}

fn panel_style() -> WidgetStyle {
    WidgetStyle {
        tone: WidgetTone::Neutral,
        prominence: WidgetProminence::Subtle,
    }
}

fn update(state: &mut ArrangementShellState, message: AppMessage) {
    match message {
        AppMessage::Frame => state.tick(),
        AppMessage::ToggleRun => {
            state.running = !state.running;
        }
        AppMessage::Reset => state.reset(),
        AppMessage::Shell(message) => state.apply_shell_message(message),
    }
}

#[derive(Clone, Debug)]
struct ArrangementOverviewWidget {
    common: WidgetCommon,
    clips: Vec<ArrangementClip>,
    selected_clip: Option<u32>,
    playhead_beat: f32,
    hover_clip: Option<u32>,
    hover_position: Option<Point>,
}

impl ArrangementOverviewWidget {
    fn new(clips: Vec<ArrangementClip>, selected_clip: Option<u32>, playhead_beat: f32) -> Self {
        let mut common = WidgetCommon::new(
            0,
            WidgetSizing::new(Vector2::new(620.0, 320.0), Vector2::new(760.0, 390.0)),
        );
        common.focus = FocusBehavior::Pointer;
        common.paint.bounds = PaintBounds::ClipToRect;
        common.paint.paints_focus = false;
        common.paint.paints_state_layers = false;
        Self {
            common,
            clips,
            selected_clip,
            playhead_beat,
            hover_clip: None,
            hover_position: None,
        }
    }

    fn timeline_rect(&self, bounds: Rect) -> Rect {
        Rect::from_min_max(
            Point::new(bounds.min.x + 72.0, bounds.min.y + 34.0),
            Point::new(bounds.max.x - 18.0, bounds.max.y - 16.0),
        )
    }

    fn track_label_rect(&self, timeline: Rect, track: usize) -> Rect {
        let y = timeline.min.y + track as f32 * track_height(timeline);
        Rect::from_min_max(
            Point::new(timeline.min.x - 64.0, y),
            Point::new(timeline.min.x - 8.0, y + track_height(timeline)),
        )
    }

    fn clip_rect(&self, timeline: Rect, clip: ArrangementClip) -> Rect {
        let x0 = x_for_beat(timeline, clip.start_beat);
        let x1 = x_for_beat(timeline, clip.end_beat());
        let y = timeline.min.y + clip.track as f32 * track_height(timeline);
        Rect::from_min_max(
            Point::new(x0, y + 8.0),
            Point::new(x1, y + track_height(timeline) - 8.0),
        )
    }

    fn clip_at_position(&self, timeline: Rect, position: Point) -> Option<u32> {
        self.clips
            .iter()
            .rev()
            .find(|clip| self.clip_rect(timeline, **clip).contains(position))
            .map(|clip| clip.id)
    }
}

impl Widget for ArrangementOverviewWidget {
    fn common(&self) -> &WidgetCommon {
        &self.common
    }

    fn common_mut(&mut self) -> &mut WidgetCommon {
        &mut self.common
    }

    fn handle_input(&mut self, bounds: Rect, input: WidgetInput) -> Option<WidgetOutput> {
        let timeline = self.timeline_rect(bounds);
        match input {
            WidgetInput::PointerMove { position } => {
                self.common.state.hovered = bounds.contains(position);
                self.hover_position = timeline.contains(position).then_some(position);
                self.hover_clip = self.clip_at_position(timeline, position);
                None
            }
            WidgetInput::PointerPress {
                position,
                button: PointerButton::Primary,
                ..
            } if timeline.contains(position) => {
                if let Some(id) = self.clip_at_position(timeline, position) {
                    self.selected_clip = Some(id);
                    Some(WidgetOutput::custom(ShellMessage::SelectClip(id)))
                } else {
                    Some(WidgetOutput::custom(ShellMessage::Seek {
                        beat: beat_for_x(timeline, position.x),
                    }))
                }
            }
            WidgetInput::PointerDrop { .. } => {
                self.hover_clip = None;
                self.hover_position = None;
                None
            }
            WidgetInput::FocusChanged(focused) => {
                self.common.state.focused = focused;
                None
            }
            _ => None,
        }
    }

    fn prefers_pointer_move_paint_only(&self) -> bool {
        true
    }

    fn synchronize_from_previous(&mut self, previous: &dyn Widget) {
        if let Some(previous) = previous.as_any().downcast_ref::<Self>() {
            self.common.state = previous.common.state;
            self.hover_clip = previous.hover_clip;
            self.hover_position = previous.hover_position;
        }
    }

    fn append_paint(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        bounds: Rect,
        _layout: &LayoutOutput,
        theme: &ThemeTokens,
    ) {
        let timeline = self.timeline_rect(bounds);
        push_rect(primitives, self.common.id, bounds, theme.bg_secondary);
        push_rect(primitives, self.common.id, timeline, rgba(8, 12, 18, 255));
        self.append_grid(primitives, timeline, theme);
        for clip in &self.clips {
            self.append_clip(primitives, timeline, *clip, theme);
        }
        push_stroke(
            primitives,
            self.common.id,
            timeline,
            theme.border_emphasis,
            1.0,
        );
    }

    fn append_runtime_overlay_paint(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        bounds: Rect,
        _layout: &LayoutOutput,
        theme: &ThemeTokens,
    ) {
        let timeline = self.timeline_rect(bounds);
        let playhead_x = x_for_beat(timeline, self.playhead_beat);
        push_rect(
            primitives,
            self.common.id,
            Rect::from_min_max(
                Point::new(playhead_x, timeline.min.y),
                Point::new(playhead_x + 2.0, timeline.max.y),
            ),
            translucent(theme.highlight_orange, 210),
        );
        if let Some(position) = self.hover_position {
            push_rect(
                primitives,
                self.common.id,
                Rect::from_min_max(
                    Point::new(position.x, timeline.min.y),
                    Point::new(position.x + 1.0, timeline.max.y),
                ),
                translucent(theme.text_muted, 80),
            );
        }
        if let Some(id) = self.hover_clip
            && let Some(clip) = self.clips.iter().copied().find(|clip| clip.id == id)
        {
            push_stroke(
                primitives,
                self.common.id,
                self.clip_rect(timeline, clip),
                translucent(theme.highlight_cyan, 190),
                2.0,
            );
        }
    }
}

impl ArrangementOverviewWidget {
    fn append_grid(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        timeline: Rect,
        theme: &ThemeTokens,
    ) {
        for (track, label) in TRACKS.iter().enumerate() {
            let y = timeline.min.y + track as f32 * track_height(timeline);
            let row = Rect::from_min_max(
                Point::new(timeline.min.x, y),
                Point::new(timeline.max.x, y + track_height(timeline)),
            );
            push_rect(
                primitives,
                self.common.id,
                row,
                if track % 2 == 0 {
                    rgba(11, 16, 23, 255)
                } else {
                    rgba(14, 19, 27, 255)
                },
            );
            push_text(
                primitives,
                self.common.id,
                *label,
                self.track_label_rect(timeline, track),
                theme.text_muted,
                PaintTextAlign::Right,
            );
        }
        for beat in 0..=TOTAL_BEATS as usize {
            let x = x_for_beat(timeline, beat as f32);
            push_rect(
                primitives,
                self.common.id,
                Rect::from_min_max(
                    Point::new(x, timeline.min.y),
                    Point::new(x + 1.0, timeline.max.y),
                ),
                if beat % 4 == 0 {
                    translucent(theme.grid_strong, 160)
                } else {
                    translucent(theme.grid_soft, 90)
                },
            );
        }
    }

    fn append_clip(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        timeline: Rect,
        clip: ArrangementClip,
        theme: &ThemeTokens,
    ) {
        let rect = self.clip_rect(timeline, clip);
        let selected = self.selected_clip == Some(clip.id);
        push_rect(
            primitives,
            self.common.id,
            rect,
            if selected {
                theme.highlight_blue
            } else {
                theme.highlight_cyan_soft
            },
        );
        push_stroke(
            primitives,
            self.common.id,
            rect,
            if selected {
                theme.border_emphasis
            } else {
                translucent(theme.border_emphasis, 140)
            },
            1.0,
        );
        push_text(
            primitives,
            self.common.id,
            clip.label,
            rect,
            theme.text_primary,
            PaintTextAlign::Center,
        );
    }
}

fn track_height(timeline: Rect) -> f32 {
    timeline.height() / TRACK_COUNT as f32
}

fn x_for_beat(timeline: Rect, beat: f32) -> f32 {
    timeline.min.x + timeline.width() * (beat / TOTAL_BEATS).clamp(0.0, 1.0)
}

fn beat_for_x(timeline: Rect, x: f32) -> f32 {
    ((x - timeline.min.x) / timeline.width().max(1.0) * TOTAL_BEATS).clamp(0.0, TOTAL_BEATS)
}

fn rgba(r: u8, g: u8, b: u8, a: u8) -> Rgba8 {
    Rgba8 { r, g, b, a }
}

fn translucent(mut color: Rgba8, alpha: u8) -> Rgba8 {
    color.a = alpha;
    color
}

fn push_rect(primitives: &mut Vec<PaintPrimitive>, widget_id: u64, rect: Rect, color: Rgba8) {
    primitives.push(PaintPrimitive::FillRect(PaintFillRect {
        widget_id,
        rect,
        color,
    }));
}

fn push_stroke(
    primitives: &mut Vec<PaintPrimitive>,
    widget_id: u64,
    rect: Rect,
    color: Rgba8,
    width: f32,
) {
    primitives.push(PaintPrimitive::StrokeRect(PaintStrokeRect {
        widget_id,
        rect,
        color,
        width,
    }));
}

fn push_text(
    primitives: &mut Vec<PaintPrimitive>,
    widget_id: u64,
    text: impl Into<String>,
    rect: Rect,
    color: Rgba8,
    align: PaintTextAlign,
) {
    primitives.push(PaintPrimitive::Text(PaintTextRun {
        widget_id,
        text: text.into().into(),
        rect,
        font_size: 12.0,
        baseline: Some(16.0),
        color,
        align,
        wrap: TextWrap::None,
    }));
}

#[cfg(test)]
mod tests {
    use super::*;
    use radiant::runtime::{RuntimeBridge, SurfaceRuntime};

    #[test]
    fn arrangement_shell_tick_advances_playhead_and_meters_without_audio_or_dsp() {
        let mut state = ArrangementShellState::default();
        let initial_playhead = state.playhead_beat;
        let initial_level = state.mixer[0].level;

        state.tick();

        assert_eq!(state.frame, 2);
        assert!(state.playhead_beat > initial_playhead);
        assert_ne!(state.mixer[0].level, initial_level);
        assert_eq!(DATA_SOURCE_NOTE, "without_audio_or_dsp");
    }

    #[test]
    fn arrangement_shell_projects_browser_arrangement_inspector_and_mixer() {
        let runtime = SurfaceRuntime::new(
            arrangement_shell_test_bridge(ArrangementShellState::default()),
            Vector2::new(1180.0, 700.0),
        );
        let paint_plan = runtime.paint_plan(&ThemeTokens::default());

        assert!(
            paint_plan
                .primitives
                .iter()
                .any(|primitive| matches!(primitive, PaintPrimitive::Text(text) if text.text.as_str().contains("Browser")))
        );
        assert!(
            runtime
                .surface()
                .find_widget(ARRANGEMENT_WIDGET_ID)
                .is_some()
        );
        assert!(runtime.surface().find_widget(STATUS_WIDGET_ID).is_some());
    }

    #[test]
    fn arrangement_overview_paints_lanes_clips_and_playhead_overlay() {
        let state = ArrangementShellState::default();
        let widget = ArrangementOverviewWidget::new(
            state.clips.clone(),
            state.selected_clip,
            state.playhead_beat,
        );
        let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(760.0, 390.0));
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
                .any(|primitive| matches!(primitive, PaintPrimitive::Text(text) if text.text.as_str() == "Bass A"))
        );
        assert!(
            overlay
                .iter()
                .any(|primitive| matches!(primitive, PaintPrimitive::FillRect(_))),
            "playhead should paint as a lightweight runtime overlay"
        );
    }

    #[test]
    fn arrangement_overview_click_selects_clip_or_seeks_empty_space() {
        let state = ArrangementShellState::default();
        let mut widget = ArrangementOverviewWidget::new(
            state.clips.clone(),
            state.selected_clip,
            state.playhead_beat,
        );
        let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(760.0, 390.0));
        let timeline = widget.timeline_rect(bounds);
        let clip = state.clips[1];

        let select = widget.handle_input(
            bounds,
            WidgetInput::PointerPress {
                position: widget.clip_rect(timeline, clip).center(),
                button: PointerButton::Primary,
                modifiers: Default::default(),
            },
        );
        let seek = widget.handle_input(
            bounds,
            WidgetInput::PointerPress {
                position: Point::new(x_for_beat(timeline, 30.0), timeline.max.y - 8.0),
                button: PointerButton::Primary,
                modifiers: Default::default(),
            },
        );

        assert_eq!(
            select.and_then(|output| output.typed_ref::<ShellMessage>().copied()),
            Some(ShellMessage::SelectClip(2))
        );
        assert!(matches!(
            seek.and_then(|output| output.typed_ref::<ShellMessage>().copied()),
            Some(ShellMessage::Seek { beat }) if beat > 29.0
        ));
    }

    #[test]
    fn arrangement_shell_hover_uses_paint_only_runtime_overlay() {
        let state = ArrangementShellState::default();
        let mut widget = ArrangementOverviewWidget::new(
            state.clips.clone(),
            state.selected_clip,
            state.playhead_beat,
        );
        let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(760.0, 390.0));
        let timeline = widget.timeline_rect(bounds);
        let clip = state.clips[1];

        let output = widget.handle_input(
            bounds,
            WidgetInput::PointerMove {
                position: widget.clip_rect(timeline, clip).center(),
            },
        );

        assert!(output.is_none());
        assert_eq!(widget.hover_clip, Some(2));
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
            "hovered clip should paint as a lightweight runtime overlay"
        );
    }

    #[test]
    fn arrangement_shell_runtime_hover_does_not_refresh_surface() {
        let bridge = arrangement_shell_test_bridge(ArrangementShellState::default());
        let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(1180.0, 700.0));
        let bounds = runtime.layout().rects[&ARRANGEMENT_WIDGET_ID];
        let first = runtime.dispatch_pointer_move_with_outcome(Point::new(
            bounds.min.x + 160.0,
            bounds.center().y,
        ));
        let second = runtime.dispatch_pointer_move_with_outcome(Point::new(
            bounds.min.x + 280.0,
            bounds.center().y,
        ));

        assert!(first.needs_scene_rebuild());
        assert!(second.paint_only_requested);
        assert!(
            !second.needs_scene_rebuild(),
            "stable arrangement-shell hover should avoid reprojection and full scene rebuilds"
        );
    }

    #[test]
    fn arrangement_shell_runtime_frame_messages_advance_status() {
        let bridge = arrangement_shell_test_bridge(ArrangementShellState::default());
        let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(1180.0, 700.0));
        let initial_status = status_text(&runtime);

        assert!(runtime.bridge_mut().needs_animation());
        assert!(runtime.bridge_mut().queue_animation_frame());
        let outcome = runtime.drain_runtime_messages();

        assert_eq!(outcome.messages_dispatched, 1);
        assert_ne!(status_text(&runtime), initial_status);
    }

    fn arrangement_shell_test_bridge(
        state: ArrangementShellState,
    ) -> impl RuntimeBridge<AppMessage> {
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
}
