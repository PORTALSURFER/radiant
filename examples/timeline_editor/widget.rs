use super::{
    CLIP_HEIGHT, HEADER_WIDTH, LANE_COUNT, LANE_HEIGHT, MIN_CLIP_BEATS, RESIZE_HANDLE_WIDTH,
    RULER_HEIGHT, TOTAL_BEATS, TRACK_PAD,
    model::{BeatRange, TimelineClip, TimelineEditorState, TimelineSurfaceMessage},
};
#[path = "widget/paint.rs"]
mod paint;

use radiant::layout::{LayoutOutput, Point, Rect, Vector2};
use radiant::runtime::PaintPrimitive;
use radiant::theme::ThemeTokens;
use radiant::widgets::{
    FocusBehavior, PointerButton, Widget, WidgetCommon, WidgetInput, WidgetKey, WidgetOutput,
    WidgetSizing,
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
        paint::append_timeline_paint(self, primitives, bounds, theme);
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
