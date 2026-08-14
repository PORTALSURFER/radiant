use super::*;

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
                modifiers: Default::default(),
                timestamp: None,
            },
        )
        .expect("empty track press seeks");
    assert_surface_message(&press, |message| {
        matches!(message, TimelineSurfaceMessage::Seek { beat: 48 })
    });

    let moved = widget.handle_input(
        bounds,
        WidgetInput::pointer_move(Point::new(
            geometry.x_for_beat(56),
            geometry.lane_rect(0).center().y,
        )),
    );
    assert!(moved.is_none(), "selection preview stays widget-local");

    let selected = widget
        .handle_input(
            bounds,
            WidgetInput::PointerRelease {
                position: Point::new(geometry.x_for_beat(56), geometry.lane_rect(0).center().y),
                button: PointerButton::Primary,
                modifiers: Default::default(),
                timestamp: None,
            },
        )
        .expect("selection release commits the preview");
    assert_surface_message(&selected, |message| {
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
                modifiers: Default::default(),
                timestamp: None,
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

    let moved_clip = widget.handle_input(
        bounds,
        WidgetInput::pointer_move(Point::new(
            geometry.x_for_beat(20),
            geometry.lane_rect(2).center().y,
        )),
    );
    assert!(moved_clip.is_none(), "move preview stays widget-local");
    let moved_clip = widget
        .handle_input(
            bounds,
            WidgetInput::PointerRelease {
                position: Point::new(geometry.x_for_beat(20), geometry.lane_rect(2).center().y),
                button: PointerButton::Primary,
                modifiers: Default::default(),
                timestamp: None,
            },
        )
        .expect("dragged clip commits on release");
    assert_surface_message(&moved_clip, |message| {
        matches!(
            message,
            TimelineSurfaceMessage::MoveClip {
                clip_id: 1,
                lane: 2,
                start: 16
            }
        )
    });

    let _ = widget.handle_input(bounds, WidgetInput::FocusChanged(true));
    let deleted = widget
        .handle_input(bounds, WidgetInput::key_press(WidgetKey::Delete))
        .expect("focused timeline delete key emits deletion");
    assert_surface_message(&deleted, |message| {
        matches!(message, TimelineSurfaceMessage::DeleteSelected)
    });
}

#[test]
fn timeline_widget_capture_cancellation_discards_drag_preview_without_committing() {
    let mut widget = ArrangementTimelineWidget::new(&TimelineEditorState::default());
    let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(860.0, 252.0));
    let geometry = widget.geometry(bounds);
    let clip_rect = geometry.clip_rect(&widget.clips[0]);

    let _ = widget.handle_input(
        bounds,
        WidgetInput::PointerPress {
            position: clip_rect.center(),
            button: PointerButton::Primary,
            modifiers: Default::default(),
            timestamp: None,
        },
    );
    let _ = widget.handle_input(
        bounds,
        WidgetInput::pointer_move(Point::new(
            geometry.x_for_beat(20),
            geometry.lane_rect(2).center().y,
        )),
    );

    assert!(widget.drag.is_some());
    assert!(Widget::handle_pointer_capture_cancelled(&mut widget, bounds).is_none());
    assert!(widget.drag.is_none());
    assert!(!widget.common.state.pressed);
    assert_eq!(widget.selection, Some(widget.clips[0].range));
}

#[test]
fn timeline_widget_focus_cancellation_discards_selection_preview_without_committing() {
    let mut widget = ArrangementTimelineWidget::new(&TimelineEditorState::default());
    let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(860.0, 252.0));
    let geometry = widget.geometry(bounds);
    let original_selection = widget.selection;
    let anchor = Point::new(geometry.x_for_beat(48), geometry.lane_rect(0).center().y);

    let _ = widget.handle_input(
        bounds,
        WidgetInput::PointerPress {
            position: anchor,
            button: PointerButton::Primary,
            modifiers: Default::default(),
            timestamp: None,
        },
    );
    let _ = widget.handle_input(
        bounds,
        WidgetInput::pointer_move(Point::new(
            geometry.x_for_beat(56),
            geometry.lane_rect(0).center().y,
        )),
    );

    assert!(
        widget
            .handle_input(bounds, WidgetInput::FocusChanged(false))
            .is_none()
    );
    assert!(widget.drag.is_none());
    assert!(!widget.common.state.pressed);
    assert!(!widget.common.state.focused);
    assert_eq!(widget.selection, original_selection);
    assert!(
        widget
            .handle_input(
                bounds,
                WidgetInput::PointerRelease {
                    position: Point::new(geometry.x_for_beat(56), geometry.lane_rect(0).center().y),
                    button: PointerButton::Primary,
                    modifiers: Default::default(),
                    timestamp: None,
                },
            )
            .is_none()
    );
}

#[test]
fn timeline_widget_short_selection_seeks_on_release() {
    let mut widget = ArrangementTimelineWidget::new(&TimelineEditorState::default());
    let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(860.0, 252.0));
    let geometry = widget.geometry(bounds);

    let _ = widget.handle_input(
        bounds,
        WidgetInput::PointerPress {
            position: Point::new(geometry.x_for_beat(48), geometry.lane_rect(0).center().y),
            button: PointerButton::Primary,
            modifiers: Default::default(),
            timestamp: None,
        },
    );
    let seek = widget
        .handle_input(
            bounds,
            WidgetInput::PointerRelease {
                position: Point::new(geometry.x_for_beat(49), geometry.lane_rect(0).center().y),
                button: PointerButton::Primary,
                modifiers: Default::default(),
                timestamp: None,
            },
        )
        .expect("short selection releases as seek");
    assert_surface_message(&seek, |message| {
        matches!(message, TimelineSurfaceMessage::Seek { beat: 49 })
    });
}

#[test]
fn timeline_widget_long_selection_release_outside_bounds_commits_stored_range_once() {
    let mut widget = ArrangementTimelineWidget::new(&TimelineEditorState::default());
    let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(860.0, 252.0));
    let geometry = widget.geometry(bounds);
    let lane_y = geometry.lane_rect(0).center().y;

    let _ = widget.handle_input(
        bounds,
        WidgetInput::PointerPress {
            position: Point::new(geometry.x_for_beat(48), lane_y),
            button: PointerButton::Primary,
            modifiers: Default::default(),
            timestamp: None,
        },
    );
    let _ = widget.handle_input(
        bounds,
        WidgetInput::pointer_move(Point::new(geometry.x_for_beat(56), lane_y)),
    );

    let release = widget
        .handle_input(
            bounds,
            WidgetInput::PointerRelease {
                position: Point::new(bounds.max.x + 24.0, lane_y),
                button: PointerButton::Primary,
                modifiers: Default::default(),
                timestamp: None,
            },
        )
        .expect("captured selection release commits outside timeline bounds");
    assert_surface_message(&release, |message| {
        matches!(
            message,
            TimelineSurfaceMessage::CreateClip { lane: 0, range }
                if *range == BeatRange { start: 48, end: 56 }
        )
    });
    assert!(widget.drag.is_none());
    assert!(!widget.common.state.pressed);
    assert!(
        widget
            .handle_input(
                bounds,
                WidgetInput::PointerRelease {
                    position: Point::new(bounds.max.x + 24.0, lane_y),
                    button: PointerButton::Primary,
                    modifiers: Default::default(),
                    timestamp: None,
                },
            )
            .is_none()
    );
}

#[test]
fn timeline_widget_short_reverse_selection_release_outside_bounds_seeks_stored_endpoint() {
    let mut widget = ArrangementTimelineWidget::new(&TimelineEditorState::default());
    let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(860.0, 252.0));
    let geometry = widget.geometry(bounds);
    let lane_y = geometry.lane_rect(0).center().y;

    let _ = widget.handle_input(
        bounds,
        WidgetInput::PointerPress {
            position: Point::new(geometry.x_for_beat(49), lane_y),
            button: PointerButton::Primary,
            modifiers: Default::default(),
            timestamp: None,
        },
    );
    let _ = widget.handle_input(
        bounds,
        WidgetInput::pointer_move(Point::new(geometry.x_for_beat(48), lane_y)),
    );

    let release = widget
        .handle_input(
            bounds,
            WidgetInput::PointerRelease {
                position: Point::new(bounds.min.x - 24.0, lane_y),
                button: PointerButton::Primary,
                modifiers: Default::default(),
                timestamp: None,
            },
        )
        .expect("captured short reverse selection release seeks outside bounds");
    assert_surface_message(&release, |message| {
        matches!(message, TimelineSurfaceMessage::Seek { beat: 48 })
    });
    assert!(widget.drag.is_none());
    assert!(!widget.common.state.pressed);
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
                modifiers: Default::default(),
                timestamp: None,
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

    let resized = widget.handle_input(
        bounds,
        WidgetInput::pointer_move(Point::new(geometry.x_for_beat(22), clip_rect.center().y)),
    );
    assert!(resized.is_none(), "resize preview stays widget-local");
    let resized = widget
        .handle_input(
            bounds,
            WidgetInput::PointerRelease {
                position: Point::new(geometry.x_for_beat(22), clip_rect.center().y),
                button: PointerButton::Primary,
                modifiers: Default::default(),
                timestamp: None,
            },
        )
        .expect("edge drag commits on release");
    assert_surface_message(&resized, |message| {
        matches!(
            message,
            TimelineSurfaceMessage::ResizeClip { clip_id: 1, range }
                if *range == BeatRange { start: 0, end: 22 }
        )
    });
}
