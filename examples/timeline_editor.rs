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

#[cfg(test)]
mod tests {
    use super::timeline_widget::TimelineGeometry;
    use super::*;
    use radiant::{
        gui::visualization::ChannelViewMode,
        layout::{LayoutOutput, Point, Rect, Vector2},
        runtime::{PaintFillRect, PaintPrimitive, PaintStrokeRect, RuntimeBridge, SurfaceRuntime},
        theme::ThemeTokens,
        widgets::{PointerButton, TextWidget, Widget, WidgetInput, WidgetKey, WidgetOutput},
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
        let state = TimelineEditorState::default();
        let mut widget = ArrangementTimelineWidget::new(&state);
        let theme = ThemeTokens::default();
        let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(860.0, 252.0));
        let geometry = widget.geometry(bounds);
        let handled = widget.handle_input(
            bounds,
            WidgetInput::PointerMove {
                position: Point::new(geometry.x_for_beat(24), bounds.center().y),
            },
        );
        assert!(handled.is_none());
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
    fn timeline_cursor_overlay_tracks_unsnapped_pointer_position() {
        let mut widget = ArrangementTimelineWidget::new(&TimelineEditorState::default());
        let theme = ThemeTokens::default();
        let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(860.0, 252.0));
        let geometry = widget.geometry(bounds);
        let pointer_x = geometry.x_for_beat(24) + 3.25;

        let handled = widget.handle_input(
            bounds,
            WidgetInput::PointerMove {
                position: Point::new(pointer_x, geometry.lane_rect(1).center().y),
            },
        );
        assert!(handled.is_none());

        let mut primitives = Vec::new();
        widget.append_paint(&mut primitives, bounds, &LayoutOutput::default(), &theme);

        let cursor_rect = primitives
            .iter()
            .find_map(|primitive| match primitive {
                PaintPrimitive::FillRect(PaintFillRect { rect, color, .. })
                    if *color == theme.highlight_orange_soft
                        && rect.width() <= 3.0
                        && rect.height() >= bounds.height() - RULER_HEIGHT =>
                {
                    Some(*rect)
                }
                _ => None,
            })
            .expect("hover cursor line should be painted");
        assert!((cursor_rect.center().x - pointer_x).abs() < 0.01);
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
        assert!(handled.is_none());
        assert_eq!(widget.hover_clip_id, Some(1));

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
