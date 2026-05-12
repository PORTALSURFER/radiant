//! Arrangement-style timeline sandbox for generic visualization state.

use radiant::gui::{
    range::NormalizedRange,
    types::Rgba8,
    visualization::{
        ChannelViewMode, SignalChromeState, SignalRasterPreview, SignalToolState,
        TimelineEditPreview, TimelineFeedbackEvents, TimelineMarkerPreview, TimelineMotionState,
        TimelinePresentationState, TimelineSurfaceState, TimelineTransportState, TimelineViewport,
    },
};
use radiant::layout::{LayoutOutput, Point, Rect, Vector2};
use radiant::prelude::*;
use radiant::runtime::{
    PaintFillRect, PaintPrimitive, PaintStrokeRect, PaintTextAlign, PaintTextRun,
};
use radiant::theme::ThemeTokens;
use radiant::widgets::{
    FocusBehavior, PointerButton, TextWrap, WidgetCommon, WidgetInput, WidgetKey, WidgetOutput,
    WidgetSizing,
};

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

#[derive(Clone, Debug, PartialEq, Eq)]
struct TimelineEditorState {
    playing: bool,
    repeat_enabled: bool,
    playhead_beat: u32,
    hover_beat: Option<u32>,
    selected_clip: Option<u32>,
    selection: Option<BeatRange>,
    next_clip_id: u32,
    revision: u64,
    feedback_nonce: u64,
    status: String,
    clips: Vec<TimelineClip>,
}

impl Default for TimelineEditorState {
    fn default() -> Self {
        Self {
            playing: false,
            repeat_enabled: true,
            playhead_beat: 18,
            hover_beat: None,
            selected_clip: Some(2),
            selection: Some(BeatRange { start: 16, end: 28 }),
            next_clip_id: 5,
            revision: 1,
            feedback_nonce: 0,
            status: "ready".to_string(),
            clips: vec![
                TimelineClip::new(1, "Kick loop", 0, 0, 16),
                TimelineClip::new(2, "Bass phrase", 1, 12, 28),
                TimelineClip::new(3, "Chord stab", 2, 28, 44),
                TimelineClip::new(4, "Vocal chop", 3, 42, 58),
            ],
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct TimelineClip {
    id: u32,
    name: &'static str,
    lane: usize,
    range: BeatRange,
}

impl TimelineClip {
    fn new(id: u32, name: &'static str, lane: usize, start: u32, end: u32) -> Self {
        Self {
            id,
            name,
            lane,
            range: BeatRange { start, end },
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct BeatRange {
    start: u32,
    end: u32,
}

impl BeatRange {
    fn normalized(start: u32, end: u32) -> Self {
        Self {
            start: start.min(end).min(TOTAL_BEATS),
            end: start.max(end).min(TOTAL_BEATS),
        }
    }

    fn duration(self) -> u32 {
        self.end.saturating_sub(self.start)
    }
}

#[derive(Clone, Debug, PartialEq)]
enum TimelineMessage {
    TogglePlay,
    ToggleRepeat(bool),
    Rewind,
    DuplicateSelection,
    DeleteSelection,
    Surface(TimelineSurfaceMessage),
}

#[derive(Clone, Debug, PartialEq)]
enum TimelineSurfaceMessage {
    Hover {
        beat: Option<u32>,
    },
    Seek {
        beat: u32,
    },
    SelectClip {
        clip_id: u32,
        beat: u32,
    },
    MoveClip {
        clip_id: u32,
        lane: usize,
        start: u32,
    },
    ResizeClip {
        clip_id: u32,
        range: BeatRange,
    },
    SelectRange {
        range: BeatRange,
    },
    CreateClip {
        lane: usize,
        range: BeatRange,
    },
    DeleteSelected,
}

#[derive(Clone, Debug)]
struct ArrangementTimelineWidget {
    common: WidgetCommon,
    clips: Vec<TimelineClip>,
    selected_clip: Option<u32>,
    selection: Option<BeatRange>,
    playhead_beat: u32,
    hover_beat: Option<u32>,
    hover_clip_id: Option<u32>,
    drag: Option<TimelineDrag>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TimelineDrag {
    Selecting {
        lane: usize,
        anchor_beat: u32,
    },
    MovingClip {
        clip_id: u32,
        pointer_offset: u32,
        duration: u32,
    },
    ResizingClip {
        clip_id: u32,
        edge: ResizeEdge,
        fixed_beat: u32,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ResizeEdge {
    Start,
    End,
}

impl ArrangementTimelineWidget {
    fn new(state: &TimelineEditorState) -> Self {
        let mut common = WidgetCommon::new(
            0,
            WidgetSizing::new(Vector2::new(520.0, 224.0), Vector2::new(760.0, 252.0)),
        );
        common.focus = FocusBehavior::Keyboard;
        Self {
            common,
            clips: state.clips.clone(),
            selected_clip: state.selected_clip,
            selection: state.selection,
            playhead_beat: state.playhead_beat,
            hover_beat: state.hover_beat,
            hover_clip_id: None,
            drag: None,
        }
    }

    fn geometry(&self, bounds: Rect) -> TimelineGeometry {
        TimelineGeometry::new(bounds)
    }

    fn clip_at(&self, geometry: TimelineGeometry, position: Point) -> Option<&TimelineClip> {
        self.clips.iter().rev().find(|clip| {
            geometry
                .clip_rect(clip)
                .inset_vertical(-4.0, -4.0)
                .contains(position)
        })
    }

    fn resize_edge_at(
        &self,
        geometry: TimelineGeometry,
        clip: &TimelineClip,
        position: Point,
    ) -> Option<ResizeEdge> {
        let rect = geometry.clip_rect(clip);
        if !rect.inset_vertical(-4.0, -4.0).contains(position) {
            return None;
        }
        if position.x <= rect.min.x + RESIZE_HANDLE_WIDTH {
            Some(ResizeEdge::Start)
        } else if position.x >= rect.max.x - RESIZE_HANDLE_WIDTH {
            Some(ResizeEdge::End)
        } else {
            None
        }
    }
}

impl Widget for ArrangementTimelineWidget {
    fn common(&self) -> &WidgetCommon {
        &self.common
    }

    fn common_mut(&mut self) -> &mut WidgetCommon {
        &mut self.common
    }

    fn handle_input(&mut self, bounds: Rect, input: WidgetInput) -> Option<WidgetOutput> {
        let geometry = self.geometry(bounds);
        match input {
            WidgetInput::PointerMove { position } => {
                self.common.state.hovered = bounds.contains(position);
                let beat = geometry.beat_at(position);
                self.hover_clip_id = self.clip_at(geometry, position).map(|clip| clip.id);
                self.hover_beat = beat;
                match (self.drag, beat) {
                    (
                        Some(TimelineDrag::Selecting {
                            lane: _,
                            anchor_beat,
                        }),
                        Some(current),
                    ) => {
                        self.hover_clip_id = None;
                        let range = BeatRange::normalized(anchor_beat, current);
                        self.selection = Some(range);
                        Some(WidgetOutput::typed(TimelineSurfaceMessage::SelectRange {
                            range,
                        }))
                    }
                    (
                        Some(TimelineDrag::MovingClip {
                            clip_id,
                            pointer_offset,
                            duration,
                        }),
                        Some(current),
                    ) => {
                        self.hover_clip_id = Some(clip_id);
                        let lane = geometry.lane_at(position).unwrap_or(0);
                        let max_start = TOTAL_BEATS.saturating_sub(duration);
                        let start = current.saturating_sub(pointer_offset).min(max_start);
                        Some(WidgetOutput::typed(TimelineSurfaceMessage::MoveClip {
                            clip_id,
                            lane,
                            start,
                        }))
                    }
                    (
                        Some(TimelineDrag::ResizingClip {
                            clip_id,
                            edge,
                            fixed_beat,
                        }),
                        Some(current),
                    ) => {
                        self.hover_clip_id = Some(clip_id);
                        let range = resized_range(edge, fixed_beat, current);
                        Some(WidgetOutput::typed(TimelineSurfaceMessage::ResizeClip {
                            clip_id,
                            range,
                        }))
                    }
                    _ => Some(WidgetOutput::typed(TimelineSurfaceMessage::Hover { beat })),
                }
            }
            WidgetInput::PointerPress {
                position,
                button: PointerButton::Primary,
            } if bounds.contains(position) => {
                let beat = geometry.beat_at(position)?;
                self.common.state.pressed = true;
                self.hover_clip_id = self.clip_at(geometry, position).map(|clip| clip.id);
                if let Some((clip_id, clip_start, clip_end, duration, edge)) =
                    self.clip_at(geometry, position).map(|clip| {
                        (
                            clip.id,
                            clip.range.start,
                            clip.range.end,
                            clip.range.duration(),
                            self.resize_edge_at(geometry, clip, position),
                        )
                    })
                {
                    self.drag = if let Some(edge) = edge {
                        Some(TimelineDrag::ResizingClip {
                            clip_id,
                            edge,
                            fixed_beat: match edge {
                                ResizeEdge::Start => clip_end,
                                ResizeEdge::End => clip_start,
                            },
                        })
                    } else {
                        Some(TimelineDrag::MovingClip {
                            clip_id,
                            pointer_offset: beat.saturating_sub(clip_start),
                            duration,
                        })
                    };
                    self.selected_clip = Some(clip_id);
                    Some(WidgetOutput::typed(TimelineSurfaceMessage::SelectClip {
                        clip_id,
                        beat,
                    }))
                } else {
                    let lane = geometry.lane_at(position).unwrap_or(0);
                    self.drag = Some(TimelineDrag::Selecting {
                        lane,
                        anchor_beat: beat,
                    });
                    self.selection = Some(BeatRange {
                        start: beat,
                        end: beat,
                    });
                    Some(WidgetOutput::typed(TimelineSurfaceMessage::Seek { beat }))
                }
            }
            WidgetInput::PointerRelease {
                position,
                button: PointerButton::Primary,
            } => {
                self.common.state.pressed = false;
                let drag = self.drag.take();
                match (drag, geometry.beat_at(position)) {
                    (Some(TimelineDrag::Selecting { lane, anchor_beat }), Some(end)) => {
                        let range = BeatRange::normalized(anchor_beat, end);
                        if range.duration() >= MIN_CLIP_BEATS {
                            Some(WidgetOutput::typed(TimelineSurfaceMessage::CreateClip {
                                lane,
                                range,
                            }))
                        } else {
                            Some(WidgetOutput::typed(TimelineSurfaceMessage::Seek {
                                beat: end,
                            }))
                        }
                    }
                    _ => None,
                }
            }
            WidgetInput::FocusChanged(focused) => {
                self.common.state.focused = focused;
                None
            }
            WidgetInput::KeyPress(WidgetKey::Space) if self.common.state.focused => {
                Some(WidgetOutput::typed(TimelineSurfaceMessage::Seek {
                    beat: self.hover_beat.unwrap_or(self.playhead_beat),
                }))
            }
            WidgetInput::KeyPress(WidgetKey::Delete | WidgetKey::Backspace)
                if self.common.state.focused && self.selected_clip.is_some() =>
            {
                Some(WidgetOutput::typed(TimelineSurfaceMessage::DeleteSelected))
            }
            _ => None,
        }
    }

    fn synchronize_from_previous(&mut self, previous: &dyn Widget) {
        if let Some(previous) = previous.as_any().downcast_ref::<Self>() {
            self.common.state = previous.common.state;
            self.drag = previous.drag;
            self.hover_beat = previous.hover_beat;
            self.hover_clip_id = previous.hover_clip_id;
        }
    }

    fn append_paint(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        bounds: Rect,
        _layout: &LayoutOutput,
        theme: &ThemeTokens,
    ) {
        let geometry = self.geometry(bounds);
        push_rect(primitives, self.common.id, bounds, theme.bg_secondary);
        push_rect(
            primitives,
            self.common.id,
            geometry.ruler,
            theme.surface_raised,
        );
        push_rect(
            primitives,
            self.common.id,
            geometry.header,
            theme.surface_base,
        );
        push_stroke(
            primitives,
            self.common.id,
            bounds,
            theme.border_emphasis,
            1.0,
        );

        for lane in 0..LANE_COUNT {
            let rect = geometry.lane_rect(lane);
            let fill = if lane % 2 == 0 {
                theme.surface_base
            } else {
                theme.bg_tertiary
            };
            push_rect(primitives, self.common.id, rect, fill);
            push_text(
                primitives,
                self.common.id,
                format!("Track {}", lane + 1),
                Rect::from_min_max(
                    Point::new(bounds.min.x + 14.0, rect.min.y + 11.0),
                    Point::new(bounds.min.x + HEADER_WIDTH - 10.0, rect.max.y),
                ),
                theme.text_muted,
                PaintTextAlign::Left,
            );
        }

        for beat in (0..=TOTAL_BEATS).step_by(4) {
            let x = geometry.x_for_beat(beat);
            let strong = beat % 16 == 0;
            push_rect(
                primitives,
                self.common.id,
                Rect::from_min_max(
                    Point::new(x, geometry.ruler.min.y),
                    Point::new(x + 1.0, bounds.max.y),
                ),
                if strong {
                    theme.grid_strong
                } else {
                    theme.grid_soft
                },
            );
            if strong {
                push_text(
                    primitives,
                    self.common.id,
                    format!("{}", beat / 4 + 1),
                    Rect::from_min_max(
                        Point::new(x + 5.0, geometry.ruler.min.y + 6.0),
                        Point::new(x + 52.0, geometry.ruler.max.y),
                    ),
                    theme.text_muted,
                    PaintTextAlign::Left,
                );
            }
        }

        if let Some(selection) = self.selection.filter(|range| range.duration() > 0) {
            push_rect(
                primitives,
                self.common.id,
                Rect::from_min_max(
                    Point::new(geometry.x_for_beat(selection.start), geometry.lanes.min.y),
                    Point::new(geometry.x_for_beat(selection.end), geometry.lanes.max.y),
                ),
                translucent(theme.highlight_blue, 64),
            );
        }

        for clip in &self.clips {
            let selected = self.selected_clip == Some(clip.id);
            let hovered = self.hover_clip_id == Some(clip.id);
            let rect = geometry.clip_rect(clip);
            let fill = match clip.lane {
                0 => theme.accent_mint,
                1 => theme.highlight_cyan,
                2 => theme.accent_copper,
                _ => theme.highlight_blue,
            };
            push_rect(
                primitives,
                self.common.id,
                rect,
                if selected || hovered {
                    fill
                } else {
                    muted(fill)
                },
            );
            push_stroke(
                primitives,
                self.common.id,
                rect,
                if selected || hovered {
                    theme.text_primary
                } else {
                    theme.border_emphasis
                },
                if selected || hovered { 2.0 } else { 1.0 },
            );
            if hovered && !selected {
                push_rect(
                    primitives,
                    self.common.id,
                    rect.top_edge_strip(4.0),
                    theme.highlight_orange_soft,
                );
            }
            push_rect(
                primitives,
                self.common.id,
                Rect::from_min_max(rect.min, Point::new(rect.min.x + 5.0, rect.max.y)),
                theme.surface_overlay,
            );
            push_text(
                primitives,
                self.common.id,
                clip.name,
                Rect::from_min_max(
                    Point::new(rect.min.x + 12.0, rect.min.y + 6.0),
                    Point::new(rect.max.x - 8.0, rect.max.y),
                ),
                theme.text_primary,
                PaintTextAlign::Left,
            );
            if hovered || selected {
                push_resize_handles(
                    primitives,
                    self.common.id,
                    rect,
                    if hovered {
                        theme.highlight_orange
                    } else {
                        theme.text_primary
                    },
                );
            }
        }

        let indicator_beat = self.hover_beat.unwrap_or(self.playhead_beat);
        let indicator_x = geometry.x_for_beat(indicator_beat);
        let indicator_color = if self.hover_beat.is_some() {
            theme.highlight_orange_soft
        } else {
            theme.highlight_orange
        };
        push_rect(
            primitives,
            self.common.id,
            Rect::from_min_max(
                Point::new(indicator_x - 1.5, geometry.ruler.min.y),
                Point::new(indicator_x + 1.5, bounds.max.y),
            ),
            indicator_color,
        );
        if self.hover_beat.is_some() {
            push_text(
                primitives,
                self.common.id,
                format!("beat {indicator_beat}"),
                Rect::from_min_max(
                    Point::new(
                        (indicator_x + 8.0).min(bounds.max.x - 82.0),
                        geometry.ruler.min.y + 6.0,
                    ),
                    Point::new(
                        (indicator_x + 82.0).min(bounds.max.x - 4.0),
                        geometry.ruler.max.y,
                    ),
                ),
                theme.text_primary,
                PaintTextAlign::Left,
            );
        }
    }
}

#[derive(Clone, Copy)]
struct TimelineGeometry {
    header: Rect,
    ruler: Rect,
    lanes: Rect,
}

impl TimelineGeometry {
    fn new(bounds: Rect) -> Self {
        let header = Rect::from_min_max(
            bounds.min,
            Point::new(bounds.min.x + HEADER_WIDTH, bounds.max.y),
        );
        let ruler = Rect::from_min_max(
            Point::new(bounds.min.x + HEADER_WIDTH, bounds.min.y),
            Point::new(bounds.max.x, bounds.min.y + RULER_HEIGHT),
        );
        let lanes = Rect::from_min_max(
            Point::new(bounds.min.x + HEADER_WIDTH, bounds.min.y + RULER_HEIGHT),
            bounds.max,
        );
        Self {
            header,
            ruler,
            lanes,
        }
    }

    fn lane_rect(self, lane: usize) -> Rect {
        let y = self.lanes.min.y + lane as f32 * LANE_HEIGHT;
        Rect::from_min_max(
            Point::new(self.lanes.min.x, y),
            Point::new(self.lanes.max.x, (y + LANE_HEIGHT).min(self.lanes.max.y)),
        )
    }

    fn clip_rect(self, clip: &TimelineClip) -> Rect {
        let lane = self.lane_rect(clip.lane);
        let y = lane.min.y + (lane.height() - CLIP_HEIGHT) * 0.5;
        Rect::from_min_max(
            Point::new(self.x_for_beat(clip.range.start) + 2.0, y),
            Point::new(self.x_for_beat(clip.range.end) - 2.0, y + CLIP_HEIGHT),
        )
    }

    fn x_for_beat(self, beat: u32) -> f32 {
        self.lanes.min.x + self.beat_width() * beat.min(TOTAL_BEATS) as f32
    }

    fn beat_at(self, position: Point) -> Option<u32> {
        if position.x < self.lanes.min.x || position.x > self.lanes.max.x {
            return None;
        }
        Some(((position.x - self.lanes.min.x) / self.beat_width()).round() as u32)
    }

    fn lane_at(self, position: Point) -> Option<usize> {
        if !self.lanes.contains(position) {
            return None;
        }
        Some(((position.y - self.lanes.min.y) / LANE_HEIGHT).floor() as usize)
            .map(|lane| lane.min(LANE_COUNT - 1))
    }

    fn beat_width(self) -> f32 {
        ((self.lanes.width() - TRACK_PAD).max(1.0)) / TOTAL_BEATS as f32
    }
}

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
            button(if state.playing { "Pause" } else { "Play" })
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

fn update(state: &mut TimelineEditorState, message: TimelineMessage) {
    match message {
        TimelineMessage::TogglePlay => {
            state.playing = !state.playing;
            state.feedback_nonce += 1;
            state.status = if state.playing { "playing" } else { "paused" }.to_string();
        }
        TimelineMessage::ToggleRepeat(enabled) => {
            state.repeat_enabled = enabled;
            state.status = if enabled {
                "loop enabled"
            } else {
                "loop disabled"
            }
            .to_string();
            state.revision += 1;
        }
        TimelineMessage::Rewind => {
            state.playhead_beat = 0;
            state.status = "rewound to bar 1".to_string();
            state.revision += 1;
        }
        TimelineMessage::DuplicateSelection => duplicate_selected_clip(state),
        TimelineMessage::DeleteSelection => delete_selected_clip(state),
        TimelineMessage::Surface(message) => update_surface(state, message),
    }
}

fn update_surface(state: &mut TimelineEditorState, message: TimelineSurfaceMessage) {
    match message {
        TimelineSurfaceMessage::Hover { beat } => {
            state.hover_beat = beat;
        }
        TimelineSurfaceMessage::Seek { beat } => {
            state.playhead_beat = beat.min(TOTAL_BEATS);
            state.selection = None;
            state.status = format!("playhead at beat {}", state.playhead_beat);
            state.revision += 1;
        }
        TimelineSurfaceMessage::SelectClip { clip_id, beat } => {
            state.selected_clip = Some(clip_id);
            state.playhead_beat = beat.min(TOTAL_BEATS);
            state.selection = clip_range(state, clip_id);
            state.status = format!("clip {} selected", clip_id);
            state.revision += 1;
        }
        TimelineSurfaceMessage::MoveClip {
            clip_id,
            lane,
            start,
        } => {
            if let Some(clip) = state.clips.iter_mut().find(|clip| clip.id == clip_id) {
                let duration = clip.range.duration();
                let start = start.min(TOTAL_BEATS.saturating_sub(duration));
                clip.lane = lane.min(LANE_COUNT - 1);
                clip.range = BeatRange {
                    start,
                    end: start + duration,
                };
                state.selected_clip = Some(clip_id);
                state.selection = Some(clip.range);
                state.status = format!("{} moved to track {}", clip.name, clip.lane + 1);
                state.revision += 1;
            }
        }
        TimelineSurfaceMessage::ResizeClip { clip_id, range } => {
            if let Some(clip) = state.clips.iter_mut().find(|clip| clip.id == clip_id) {
                clip.range = range;
                state.selected_clip = Some(clip_id);
                state.selection = Some(range);
                state.status = format!(
                    "{} resized to beats {}-{}",
                    clip.name, clip.range.start, clip.range.end
                );
                state.revision += 1;
            }
        }
        TimelineSurfaceMessage::SelectRange { range } => {
            state.selection = Some(range);
            state.selected_clip = None;
            state.status = format!("selected beats {}-{}", range.start, range.end);
            state.revision += 1;
        }
        TimelineSurfaceMessage::CreateClip { lane, range } => {
            create_clip(state, lane, range);
        }
        TimelineSurfaceMessage::DeleteSelected => delete_selected_clip(state),
    }
}

fn create_clip(state: &mut TimelineEditorState, lane: usize, range: BeatRange) {
    if range.duration() < MIN_CLIP_BEATS {
        return;
    }
    let id = state.next_clip_id;
    state.next_clip_id += 1;
    state.clips.push(TimelineClip {
        id,
        name: "New clip",
        lane: lane.min(LANE_COUNT - 1),
        range,
    });
    state.selected_clip = Some(id);
    state.selection = Some(range);
    state.playhead_beat = range.start;
    state.status = format!("created clip {} on track {}", id, lane + 1);
    state.feedback_nonce += 1;
    state.revision += 1;
}

fn duplicate_selected_clip(state: &mut TimelineEditorState) {
    let Some(source_id) = state.selected_clip else {
        state.status = "select a clip first".to_string();
        return;
    };
    let Some(source) = state
        .clips
        .iter()
        .find(|clip| clip.id == source_id)
        .cloned()
    else {
        return;
    };
    let duration = source.range.duration();
    let start = (source.range.end + 2).min(TOTAL_BEATS.saturating_sub(duration));
    let id = state.next_clip_id;
    state.next_clip_id += 1;
    state.clips.push(TimelineClip {
        id,
        name: "Copy",
        lane: source.lane,
        range: BeatRange {
            start,
            end: start + duration,
        },
    });
    state.selected_clip = Some(id);
    state.selection = Some(BeatRange {
        start,
        end: start + duration,
    });
    state.status = format!("duplicated clip {}", source_id);
    state.revision += 1;
}

fn delete_selected_clip(state: &mut TimelineEditorState) {
    let Some(clip_id) = state.selected_clip else {
        state.status = "select a clip first".to_string();
        return;
    };
    let before = state.clips.len();
    state.clips.retain(|clip| clip.id != clip_id);
    if state.clips.len() == before {
        state.status = format!("clip {} was already gone", clip_id);
        state.selected_clip = None;
        state.selection = None;
        state.revision += 1;
        return;
    }
    state.selected_clip = None;
    state.selection = None;
    state.status = format!("deleted clip {}", clip_id);
    state.feedback_nonce += 1;
    state.revision += 1;
}

fn timeline_surface(state: &TimelineEditorState) -> TimelineMotionState {
    let selection = state.selection.map(|range| {
        NormalizedRange::from_micros(beat_to_micros(range.start), beat_to_micros(range.end))
    });
    let surface = TimelineSurfaceState::new(
        TimelineViewport::new(0, 1_000, 0, 1_000_000, 0, 1_000_000_000),
        TimelineTransportState::new(
            Some(beat_to_normalized(state.playhead_beat)),
            state.hover_beat.map(beat_to_normalized),
            Some(beat_to_micros(state.playhead_beat)),
            selection,
        ),
        TimelineEditPreview::new(
            selection,
            selection.map(|range| range.start_milli),
            selection.map(|range| range.start_micros),
            selection.map(|range| range.start_milli.saturating_add(1)),
            selection.map(|range| range.start_micros.saturating_add(1_000)),
            selection.map(|range| range.end_milli),
            selection.map(|range| range.end_milli.saturating_sub(1)),
            selection.map(|range| range.end_micros.saturating_sub(1_000)),
            selection.map(|range| range.end_milli),
            selection.map(|range| range.end_micros),
            state.hover_beat.map(beat_to_normalized),
        ),
        TimelineFeedbackEvents::new(state.feedback_nonce, 0, state.revision),
        TimelinePresentationState::new(
            Some(beat_to_micros(4)),
            0,
            state.repeat_enabled,
            Some("Arrangement".to_string()),
            Some(format!("{} beats", TOTAL_BEATS)),
        ),
        SignalRasterPreview::new(
            Some("arrangement timeline atlas".to_string()),
            false,
            false,
            Some(state.revision),
            None,
        ),
        state
            .clips
            .iter()
            .map(|clip| {
                marker(
                    beat_to_normalized(clip.range.start),
                    beat_to_normalized(clip.range.end),
                    state.selected_clip == Some(clip.id),
                )
            })
            .collect(),
    );

    TimelineMotionState::new(
        state.playing,
        surface,
        SignalChromeState::new(
            if state.playing { "playing" } else { "idle" },
            true,
            Some(format!("beat {}", state.playhead_beat)),
            ChannelViewMode::Stereo,
        ),
        SignalToolState::new(false, true, true, true, true, true, true, true),
    )
}

fn marker(start: u16, end: u16, selected: bool) -> TimelineMarkerPreview {
    TimelineMarkerPreview {
        range: NormalizedRange::new(start, end),
        selected,
        focused: selected,
    }
}

fn clip_range(state: &TimelineEditorState, clip_id: u32) -> Option<BeatRange> {
    state
        .clips
        .iter()
        .find(|clip| clip.id == clip_id)
        .map(|clip| clip.range)
}

fn resized_range(edge: ResizeEdge, fixed_beat: u32, pointer_beat: u32) -> BeatRange {
    match edge {
        ResizeEdge::Start => {
            let start = pointer_beat.min(fixed_beat.saturating_sub(MIN_CLIP_BEATS));
            BeatRange {
                start,
                end: fixed_beat,
            }
        }
        ResizeEdge::End => {
            let end = pointer_beat
                .max(fixed_beat.saturating_add(MIN_CLIP_BEATS))
                .min(TOTAL_BEATS);
            BeatRange {
                start: fixed_beat,
                end,
            }
        }
    }
}

fn beat_to_normalized(beat: u32) -> u16 {
    ((beat.min(TOTAL_BEATS) as f32 / TOTAL_BEATS as f32) * 1_000.0).round() as u16
}

fn beat_to_micros(beat: u32) -> u32 {
    beat.min(TOTAL_BEATS) * 125_000
}

fn timeline_label(state: &TimelineEditorState, timeline: &TimelineMotionState) -> String {
    format!(
        "{} / clips {} / cursor {} / {}",
        timeline.chrome.status_hint,
        timeline.surface.markers.len(),
        state
            .hover_beat
            .map(|beat| format!("beat {beat}"))
            .unwrap_or_else(|| "off timeline".to_string()),
        state.status
    )
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
        font_size: 13.0,
        baseline: Some(18.0),
        color,
        align,
        wrap: TextWrap::None,
    }));
}

fn push_resize_handles(
    primitives: &mut Vec<PaintPrimitive>,
    widget_id: u64,
    rect: Rect,
    color: Rgba8,
) {
    let width = RESIZE_HANDLE_WIDTH.min((rect.width() * 0.5).max(0.0));
    if width <= 0.0 {
        return;
    }
    push_rect(
        primitives,
        widget_id,
        Rect::from_min_max(rect.min, Point::new(rect.min.x + width, rect.max.y)),
        color,
    );
    push_rect(
        primitives,
        widget_id,
        Rect::from_min_max(Point::new(rect.max.x - width, rect.min.y), rect.max),
        color,
    );
}

fn translucent(mut color: Rgba8, alpha: u8) -> Rgba8 {
    color.a = alpha;
    color
}

fn muted(color: Rgba8) -> Rgba8 {
    Rgba8 {
        r: ((color.r as u16 + 28) / 2) as u8,
        g: ((color.g as u16 + 28) / 2) as u8,
        b: ((color.b as u16 + 36) / 2) as u8,
        a: 230,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use radiant::{
        runtime::{RuntimeBridge, SurfaceRuntime},
        widgets::{TextWidget, WidgetOutput},
    };

    #[test]
    fn timeline_editor_projects_arrangement_state() {
        let state = TimelineEditorState::default();
        let timeline = timeline_surface(&state);

        assert_eq!(timeline.surface.markers.len(), 4);
        assert_eq!(
            timeline.surface.transport.resolved_playhead_micros(),
            Some(2_250_000)
        );
        assert_eq!(timeline.chrome.channel_view, ChannelViewMode::Stereo);
        assert_eq!(
            timeline.surface.transport.cursor_milli,
            Some(beat_to_normalized(18))
        );
    }

    #[test]
    fn timeline_widget_creates_and_moves_clips_from_pointer_input() {
        let mut widget = ArrangementTimelineWidget::new(&TimelineEditorState::default());
        let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(860.0, 252.0));
        let geometry = widget.geometry(bounds);

        let press = widget
            .handle_input(
                bounds,
                WidgetInput::PointerPress {
                    position: Point::new(geometry.x_for_beat(48), geometry.lane_rect(0).center().y),
                    button: PointerButton::Primary,
                },
            )
            .expect("empty track press seeks");
        assert_surface_message(&press, |message| {
            matches!(message, TimelineSurfaceMessage::Seek { beat: 48 })
        });

        let moved = widget
            .handle_input(
                bounds,
                WidgetInput::PointerMove {
                    position: Point::new(geometry.x_for_beat(56), geometry.lane_rect(0).center().y),
                },
            )
            .expect("selection drag updates range");
        assert_surface_message(&moved, |message| {
            matches!(
                message,
                TimelineSurfaceMessage::SelectRange { range }
                    if *range == BeatRange { start: 48, end: 56 }
            )
        });

        let created = widget
            .handle_input(
                bounds,
                WidgetInput::PointerRelease {
                    position: Point::new(geometry.x_for_beat(56), geometry.lane_rect(0).center().y),
                    button: PointerButton::Primary,
                },
            )
            .expect("selection release creates a clip");
        assert_surface_message(&created, |message| {
            matches!(
                message,
                TimelineSurfaceMessage::CreateClip { lane: 0, range }
                    if *range == BeatRange { start: 48, end: 56 }
            )
        });

        let press_clip = widget
            .handle_input(
                bounds,
                WidgetInput::PointerPress {
                    position: Point::new(geometry.x_for_beat(4), geometry.lane_rect(0).center().y),
                    button: PointerButton::Primary,
                },
            )
            .expect("clip press selects before moving");
        assert_surface_message(&press_clip, |message| {
            matches!(
                message,
                TimelineSurfaceMessage::SelectClip {
                    clip_id: 1,
                    beat: 4
                }
            )
        });

        let moved_clip = widget
            .handle_input(
                bounds,
                WidgetInput::PointerMove {
                    position: Point::new(geometry.x_for_beat(20), geometry.lane_rect(2).center().y),
                },
            )
            .expect("dragged clip emits a move");
        assert_surface_message(&moved_clip, |message| {
            matches!(
                message,
                TimelineSurfaceMessage::MoveClip {
                    clip_id: 1,
                    lane: 2,
                    start: 16,
                }
            )
        });

        let _ = widget.handle_input(bounds, WidgetInput::FocusChanged(true));
        let deleted = widget
            .handle_input(bounds, WidgetInput::KeyPress(WidgetKey::Delete))
            .expect("focused timeline delete key emits deletion");
        assert_surface_message(&deleted, |message| {
            matches!(message, TimelineSurfaceMessage::DeleteSelected)
        });
    }

    #[test]
    fn timeline_widget_resizes_clips_from_edge_drag() {
        let mut widget = ArrangementTimelineWidget::new(&TimelineEditorState::default());
        let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(860.0, 252.0));
        let geometry = widget.geometry(bounds);
        let clip_rect = geometry.clip_rect(&widget.clips[0]);

        let press_edge = widget
            .handle_input(
                bounds,
                WidgetInput::PointerPress {
                    position: Point::new(clip_rect.max.x - 2.0, clip_rect.center().y),
                    button: PointerButton::Primary,
                },
            )
            .expect("clip edge press selects before resizing");
        assert_surface_message(&press_edge, |message| {
            matches!(
                message,
                TimelineSurfaceMessage::SelectClip {
                    clip_id: 1,
                    beat: 16
                }
            )
        });

        let resized = widget
            .handle_input(
                bounds,
                WidgetInput::PointerMove {
                    position: Point::new(geometry.x_for_beat(22), clip_rect.center().y),
                },
            )
            .expect("edge drag emits resize");
        assert_surface_message(&resized, |message| {
            matches!(
                message,
                TimelineSurfaceMessage::ResizeClip { clip_id: 1, range }
                    if *range == BeatRange { start: 0, end: 22 }
            )
        });
    }

    #[test]
    fn timeline_widget_paints_one_vertical_cursor_indicator() {
        let mut state = TimelineEditorState::default();
        state.hover_beat = Some(24);
        let widget = ArrangementTimelineWidget::new(&state);
        let theme = ThemeTokens::default();
        let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(860.0, 252.0));
        let mut primitives = Vec::new();

        widget.append_paint(&mut primitives, bounds, &LayoutOutput::default(), &theme);

        let indicator_lines = primitives
            .iter()
            .filter(|primitive| {
                matches!(
                    primitive,
                    PaintPrimitive::FillRect(PaintFillRect { rect, color, .. })
                        if rect.width() <= 3.0
                            && rect.height() >= bounds.height() - RULER_HEIGHT
                            && (*color == theme.highlight_orange
                                || *color == theme.highlight_orange_soft)
                )
            })
            .count();
        assert_eq!(indicator_lines, 1);
    }

    #[test]
    fn timeline_widget_highlights_hovered_clip() {
        let mut widget = ArrangementTimelineWidget::new(&TimelineEditorState::default());
        let theme = ThemeTokens::default();
        let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(860.0, 252.0));
        let geometry = widget.geometry(bounds);

        let handled = widget.handle_input(
            bounds,
            WidgetInput::PointerMove {
                position: Point::new(geometry.x_for_beat(4), geometry.lane_rect(0).center().y),
            },
        );
        assert!(handled.is_some());

        let mut primitives = Vec::new();
        widget.append_paint(&mut primitives, bounds, &LayoutOutput::default(), &theme);

        let hover_rect = geometry.clip_rect(&widget.clips[0]);
        let hover_border = primitives.iter().any(|primitive| {
            matches!(
                primitive,
                PaintPrimitive::StrokeRect(PaintStrokeRect {
                    rect,
                    color,
                    width,
                    ..
                }) if *rect == hover_rect && *color == theme.text_primary && *width == 2.0
            )
        });
        let hover_strip = primitives.iter().any(|primitive| {
            matches!(
                primitive,
                PaintPrimitive::FillRect(PaintFillRect { rect, color, .. })
                    if *rect == hover_rect.top_edge_strip(4.0)
                        && *color == theme.highlight_orange_soft
            )
        });

        assert!(hover_border);
        assert!(hover_strip);
    }

    #[test]
    fn timeline_editor_routes_surface_messages_through_runtime() {
        let bridge = radiant::app(TimelineEditorState::default())
            .view(project_surface)
            .update(update)
            .into_bridge();
        let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(860.0, 460.0));

        assert!(runtime.surface().find_widget(TIMELINE_WIDGET_ID).is_some());
        assert!(runtime.surface().find_widget(18).is_some());
        assert!(
            runtime
                .surface()
                .keyboard_focus_order()
                .contains(&TIMELINE_WIDGET_ID)
        );

        let geometry = TimelineGeometry::new(Rect::from_min_size(
            Point::new(16.0, 58.0),
            Vector2::new(828.0, 252.0),
        ));
        let target = Point::new(geometry.x_for_beat(48), geometry.lane_rect(0).center().y);
        assert!(runtime.dispatch_input(
            TIMELINE_WIDGET_ID,
            WidgetInput::PointerPress {
                position: target,
                button: PointerButton::Primary,
            },
        ));
        assert!(runtime.dispatch_input(
            TIMELINE_WIDGET_ID,
            WidgetInput::PointerRelease {
                position: Point::new(geometry.x_for_beat(56), target.y),
                button: PointerButton::Primary,
            },
        ));

        let status = status_text(&runtime);
        assert!(status.contains("created clip"));
    }

    #[test]
    fn timeline_editor_deletes_selected_clip_from_toolbar() {
        let bridge = radiant::app(TimelineEditorState::default())
            .view(project_surface)
            .update(update)
            .into_bridge();
        let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(860.0, 460.0));

        assert!(runtime.focus_widget(32));
        assert!(runtime.dispatch_input(32, WidgetInput::KeyPress(WidgetKey::Enter)));

        let status = status_text(&runtime);
        assert!(status.contains("clips 3"));
        assert!(status.contains("deleted clip 2"));
    }

    #[test]
    fn delete_selected_clip_clears_selection_without_touching_other_clips() {
        let mut state = TimelineEditorState::default();

        delete_selected_clip(&mut state);

        assert_eq!(state.clips.len(), 3);
        assert!(state.clips.iter().all(|clip| clip.id != 2));
        assert_eq!(state.selected_clip, None);
        assert_eq!(state.selection, None);
        assert_eq!(state.status, "deleted clip 2");
    }

    #[test]
    fn resize_clip_updates_range_and_selection() {
        let mut state = TimelineEditorState::default();

        update_surface(
            &mut state,
            TimelineSurfaceMessage::ResizeClip {
                clip_id: 2,
                range: BeatRange { start: 8, end: 30 },
            },
        );

        let resized = state
            .clips
            .iter()
            .find(|clip| clip.id == 2)
            .expect("clip remains after resize");
        assert_eq!(resized.range, BeatRange { start: 8, end: 30 });
        assert_eq!(state.selected_clip, Some(2));
        assert_eq!(state.selection, Some(BeatRange { start: 8, end: 30 }));
        assert!(state.status.contains("resized to beats 8-30"));
    }

    fn assert_surface_message(
        output: &WidgetOutput,
        matches: impl FnOnce(&TimelineSurfaceMessage) -> bool,
    ) {
        let message = output
            .typed_ref::<TimelineSurfaceMessage>()
            .expect("timeline widget emits timeline messages");
        assert!(matches(message), "unexpected message: {message:?}");
    }

    fn status_text<Bridge>(runtime: &SurfaceRuntime<Bridge, TimelineMessage>) -> String
    where
        Bridge: RuntimeBridge<TimelineMessage>,
    {
        runtime
            .surface()
            .find_widget(STATUS_WIDGET_ID)
            .expect("status widget exists")
            .widget_object()
            .as_any()
            .downcast_ref::<TextWidget>()
            .expect("status widget is text")
            .text
            .to_string()
    }
}
