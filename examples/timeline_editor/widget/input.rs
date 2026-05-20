#[path = "input/clip_handles.rs"]
mod clip_handles;

use super::super::model::{BeatRange, TimelineSurfaceMessage};
use super::{ArrangementTimelineWidget, MIN_CLIP_BEATS, ResizeEdge, TOTAL_BEATS, TimelineDrag};
use clip_handles::clip_handle_at;
use radiant::layout::Rect;
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
            widget.hover_clip_id =
                clip_handle_at(widget, geometry, position).map(|handle| handle.clip_id);
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
                        clip_name,
                        source_lane,
                        pointer_offset,
                        duration,
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
                        current_lane: lane,
                        current_start: start,
                    });
                    Some(WidgetOutput::typed(TimelineSurfaceMessage::MoveClip {
                        clip_id,
                        lane,
                        start,
                    }))
                }
                (
                    Some(TimelineDrag::ResizingClip {
                        clip_id,
                        clip_name,
                        source_lane,
                        edge,
                        fixed_beat,
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
                        current_range: range,
                    });
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
            ..
        } if bounds.contains(position) => {
            let beat = geometry.beat_at(position)?;
            widget.common.state.pressed = true;
            widget.hover_clip_id =
                clip_handle_at(widget, geometry, position).map(|handle| handle.clip_id);
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
            ..
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
