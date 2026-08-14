use super::super::{ArrangementTimelineWidget, ResizeEdge, TimelineDrag, TimelineGeometry};
use super::clip_handles::clip_handle_at;
use crate::model::{BeatRange, TimelineSurfaceMessage};
use crate::{MIN_CLIP_BEATS, TOTAL_BEATS};
use radiant::layout::{Point, Rect};
use radiant::widgets::WidgetOutput;

pub(super) fn handle_pointer_move(
    widget: &mut ArrangementTimelineWidget,
    bounds: Rect,
    geometry: TimelineGeometry,
    position: Point,
) -> Option<WidgetOutput> {
    widget.common.state.hovered = bounds.contains(position);
    let beat = if geometry.cursor_x_at(position).is_some() {
        widget.cursor.set_hover(geometry, position)
    } else {
        widget.cursor.clear_hover();
        None
    };
    widget.hover_clip_id = clip_handle_at(widget, geometry, position).map(|handle| handle.clip_id);
    match (widget.drag, beat) {
        (
            Some(TimelineDrag::Selecting {
                lane,
                anchor_beat,
                previous_selection,
                previous_selected_clip,
                ..
            }),
            Some(current),
        ) => {
            widget.hover_clip_id = None;
            let range = BeatRange::normalized(anchor_beat, current);
            widget.selection = Some(range);
            widget.drag = Some(TimelineDrag::Selecting {
                lane,
                anchor_beat,
                current_range: range,
                previous_selection,
                previous_selected_clip,
            });
            None
        }
        (
            Some(TimelineDrag::MovingClip {
                clip_id,
                clip_name,
                source_lane,
                pointer_offset,
                duration,
                initial_start,
                ..
            }),
            Some(current),
        ) => {
            let lane = geometry.lane_at(position).unwrap_or(0);
            let max_start = TOTAL_BEATS.saturating_sub(duration);
            let start = current.saturating_sub(pointer_offset).min(max_start);
            widget.hover_clip_id = Some(clip_id);
            widget.selection = Some(BeatRange {
                start,
                end: start + duration,
            });
            widget.drag = Some(TimelineDrag::MovingClip {
                clip_id,
                clip_name,
                source_lane,
                pointer_offset,
                duration,
                initial_start,
                current_lane: lane,
                current_start: start,
            });
            None
        }
        (
            Some(TimelineDrag::ResizingClip {
                clip_id,
                clip_name,
                source_lane,
                edge,
                fixed_beat,
                initial_range,
                ..
            }),
            Some(current),
        ) => {
            let range = resized_range(edge, fixed_beat, current);
            widget.hover_clip_id = Some(clip_id);
            widget.selection = Some(range);
            widget.drag = Some(TimelineDrag::ResizingClip {
                clip_id,
                clip_name,
                source_lane,
                edge,
                fixed_beat,
                initial_range,
                current_range: range,
            });
            None
        }
        _ => None,
    }
}

pub(super) fn handle_primary_press(
    widget: &mut ArrangementTimelineWidget,
    geometry: TimelineGeometry,
    position: Point,
) -> Option<WidgetOutput> {
    let beat = geometry.beat_at(position)?;
    widget.common.state.pressed = true;
    widget.hover_clip_id = clip_handle_at(widget, geometry, position).map(|handle| handle.clip_id);
    if let Some(handle) = clip_handle_at(widget, geometry, position) {
        widget.drag = if let Some(edge) = handle.resize_edge() {
            Some(TimelineDrag::ResizingClip {
                clip_id: handle.clip_id,
                clip_name: handle.clip_name,
                source_lane: handle.clip_lane,
                edge,
                fixed_beat: match edge {
                    ResizeEdge::Start => handle.clip_end,
                    ResizeEdge::End => handle.clip_start,
                },
                initial_range: BeatRange {
                    start: handle.clip_start,
                    end: handle.clip_end,
                },
                current_range: BeatRange {
                    start: handle.clip_start,
                    end: handle.clip_end,
                },
            })
        } else {
            Some(TimelineDrag::MovingClip {
                clip_id: handle.clip_id,
                clip_name: handle.clip_name,
                source_lane: handle.clip_lane,
                pointer_offset: beat.saturating_sub(handle.clip_start),
                duration: handle.duration,
                initial_start: handle.clip_start,
                current_lane: handle.clip_lane,
                current_start: handle.clip_start,
            })
        };
        widget.selected_clip = Some(handle.clip_id);
        Some(WidgetOutput::typed(TimelineSurfaceMessage::SelectClip {
            clip_id: handle.clip_id,
            beat,
        }))
    } else {
        let lane = geometry.lane_at(position).unwrap_or(0);
        widget.drag = Some(TimelineDrag::Selecting {
            lane,
            anchor_beat: beat,
            current_range: BeatRange {
                start: beat,
                end: beat,
            },
            previous_selection: widget.selection,
            previous_selected_clip: widget.selected_clip,
        });
        widget.selection = Some(BeatRange {
            start: beat,
            end: beat,
        });
        Some(WidgetOutput::typed(TimelineSurfaceMessage::Seek { beat }))
    }
}

pub(super) fn handle_primary_release(
    widget: &mut ArrangementTimelineWidget,
    geometry: TimelineGeometry,
    position: Point,
) -> Option<WidgetOutput> {
    widget.common.state.pressed = false;
    let release_beat = geometry.beat_at(position);
    let drag = widget.drag.take();
    match (drag, release_beat) {
        (
            Some(TimelineDrag::Selecting {
                anchor_beat, lane, ..
            }),
            Some(end),
        ) => {
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
        (
            Some(TimelineDrag::MovingClip {
                clip_id,
                source_lane,
                initial_start,
                current_lane,
                current_start,
                ..
            }),
            _,
        ) if source_lane != current_lane || initial_start != current_start => {
            Some(WidgetOutput::typed(TimelineSurfaceMessage::MoveClip {
                clip_id,
                lane: current_lane,
                start: current_start,
            }))
        }
        (
            Some(TimelineDrag::ResizingClip {
                clip_id,
                initial_range,
                current_range,
                ..
            }),
            _,
        ) if initial_range != current_range => {
            Some(WidgetOutput::typed(TimelineSurfaceMessage::ResizeClip {
                clip_id,
                range: current_range,
            }))
        }
        _ => None,
    }
}

pub(super) fn discard_drag_preview(widget: &mut ArrangementTimelineWidget) {
    let drag = widget.drag.take();
    widget.common.state.pressed = false;
    widget.hover_clip_id = None;
    match drag {
        Some(TimelineDrag::Selecting {
            previous_selection,
            previous_selected_clip,
            ..
        }) => {
            widget.selection = previous_selection;
            widget.selected_clip = previous_selected_clip;
        }
        Some(TimelineDrag::MovingClip {
            clip_id,
            initial_start,
            duration,
            ..
        }) => {
            widget.selected_clip = Some(clip_id);
            widget.selection = Some(BeatRange {
                start: initial_start,
                end: initial_start + duration,
            });
        }
        Some(TimelineDrag::ResizingClip { initial_range, .. }) => {
            widget.selection = Some(initial_range);
        }
        None => {}
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
