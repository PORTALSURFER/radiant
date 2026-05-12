use super::super::model::{BeatRange, TimelineClip, TimelineSurfaceMessage};
use super::{
    ArrangementTimelineWidget, MIN_CLIP_BEATS, RESIZE_HANDLE_WIDTH, ResizeEdge, TOTAL_BEATS,
    TimelineDrag, TimelineGeometry,
};
use radiant::layout::{Point, Rect};
use radiant::widgets::{PointerButton, WidgetInput, WidgetKey, WidgetOutput};

pub(super) fn handle_timeline_input(
    widget: &mut ArrangementTimelineWidget,
    bounds: Rect,
    input: WidgetInput,
) -> Option<WidgetOutput> {
    let geometry = widget.geometry(bounds);
    match input {
        WidgetInput::PointerMove { position } => {
            widget.common.state.hovered = bounds.contains(position);
            let beat = if geometry.cursor_x_at(position).is_some() {
                widget.cursor.set_hover(geometry, position)
            } else {
                widget.cursor.clear_hover();
                None
            };
            widget.hover_clip_id = clip_at(widget, geometry, position).map(|clip| clip.id);
            match (widget.drag, beat) {
                (
                    Some(TimelineDrag::Selecting {
                        lane: _,
                        anchor_beat,
                    }),
                    Some(current),
                ) => {
                    widget.hover_clip_id = None;
                    let range = BeatRange::normalized(anchor_beat, current);
                    widget.selection = Some(range);
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
                    widget.hover_clip_id = Some(clip_id);
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
                    widget.hover_clip_id = Some(clip_id);
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
            widget.common.state.pressed = true;
            widget.hover_clip_id = clip_at(widget, geometry, position).map(|clip| clip.id);
            if let Some((clip_id, clip_start, clip_end, duration, edge)) =
                clip_at(widget, geometry, position).map(|clip| {
                    (
                        clip.id,
                        clip.range.start,
                        clip.range.end,
                        clip.range.duration(),
                        resize_edge_at(geometry, clip, position),
                    )
                })
            {
                widget.drag = if let Some(edge) = edge {
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
                widget.selected_clip = Some(clip_id);
                Some(WidgetOutput::typed(TimelineSurfaceMessage::SelectClip {
                    clip_id,
                    beat,
                }))
            } else {
                let lane = geometry.lane_at(position).unwrap_or(0);
                widget.drag = Some(TimelineDrag::Selecting {
                    lane,
                    anchor_beat: beat,
                });
                widget.selection = Some(BeatRange {
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
            widget.common.state.pressed = false;
            let drag = widget.drag.take();
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
            widget.common.state.focused = focused;
            None
        }
        WidgetInput::KeyPress(WidgetKey::Space) if widget.common.state.focused => {
            Some(WidgetOutput::typed(TimelineSurfaceMessage::Seek {
                beat: widget.cursor.active_beat(widget.playhead_beat),
            }))
        }
        WidgetInput::KeyPress(WidgetKey::Delete | WidgetKey::Backspace)
            if widget.common.state.focused && widget.selected_clip.is_some() =>
        {
            Some(WidgetOutput::typed(TimelineSurfaceMessage::DeleteSelected))
        }
        _ => None,
    }
}

fn clip_at(
    widget: &ArrangementTimelineWidget,
    geometry: TimelineGeometry,
    position: Point,
) -> Option<&TimelineClip> {
    widget.clips.iter().rev().find(|clip| {
        geometry
            .clip_rect(clip)
            .inset_vertical(-4.0, -4.0)
            .contains(position)
    })
}

fn resize_edge_at(
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
