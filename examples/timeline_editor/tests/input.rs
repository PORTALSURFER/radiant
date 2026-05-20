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
                modifiers: Default::default(),
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
                modifiers: Default::default(),
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
                modifiers: Default::default(),
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
