use super::{
    CLIP_HEIGHT, HEADER_WIDTH, LANE_COUNT, LANE_HEIGHT, MIN_CLIP_BEATS, RESIZE_HANDLE_WIDTH,
    RULER_HEIGHT, TOTAL_BEATS, TRACK_PAD,
    model::{BeatRange, TimelineClip, TimelineEditorState, TimelineSurfaceMessage},
};
use radiant::gui::types::Rgba8;
use radiant::layout::{LayoutOutput, Point, Rect, Vector2};
use radiant::runtime::{
    PaintFillRect, PaintPrimitive, PaintStrokeRect, PaintTextAlign, PaintTextRun,
};
use radiant::theme::ThemeTokens;
use radiant::widgets::{
    FocusBehavior, PointerButton, TextWrap, Widget, WidgetCommon, WidgetInput, WidgetKey,
    WidgetOutput, WidgetSizing,
};

#[derive(Clone, Debug)]
pub(super) struct ArrangementTimelineWidget {
    pub(super) common: WidgetCommon,
    pub(super) clips: Vec<TimelineClip>,
    pub(super) selected_clip: Option<u32>,
    pub(super) selection: Option<BeatRange>,
    pub(super) playhead_beat: u32,
    pub(super) hover_beat: Option<u32>,
    pub(super) hover_clip_id: Option<u32>,
    pub(super) drag: Option<TimelineDrag>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum TimelineDrag {
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
pub(super) enum ResizeEdge {
    Start,
    End,
}

impl ArrangementTimelineWidget {
    pub(super) fn new(state: &TimelineEditorState) -> Self {
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
            hover_beat: None,
            hover_clip_id: None,
            drag: None,
        }
    }

    pub(super) fn geometry(&self, bounds: Rect) -> TimelineGeometry {
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
                    _ => None,
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
pub(super) struct TimelineGeometry {
    header: Rect,
    ruler: Rect,
    lanes: Rect,
}

impl TimelineGeometry {
    pub(super) fn new(bounds: Rect) -> Self {
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

    pub(super) fn lane_rect(self, lane: usize) -> Rect {
        let y = self.lanes.min.y + lane as f32 * LANE_HEIGHT;
        Rect::from_min_max(
            Point::new(self.lanes.min.x, y),
            Point::new(self.lanes.max.x, (y + LANE_HEIGHT).min(self.lanes.max.y)),
        )
    }

    pub(super) fn clip_rect(self, clip: &TimelineClip) -> Rect {
        let lane = self.lane_rect(clip.lane);
        let y = lane.min.y + (lane.height() - CLIP_HEIGHT) * 0.5;
        Rect::from_min_max(
            Point::new(self.x_for_beat(clip.range.start) + 2.0, y),
            Point::new(self.x_for_beat(clip.range.end) - 2.0, y + CLIP_HEIGHT),
        )
    }

    pub(super) fn x_for_beat(self, beat: u32) -> f32 {
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
