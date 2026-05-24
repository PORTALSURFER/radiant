use super::*;

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

#[test]
fn creating_clip_cuts_existing_clips_on_same_lane() {
    let mut state = TimelineEditorState::default();

    update_surface(
        &mut state,
        TimelineSurfaceMessage::CreateClip {
            lane: 0,
            range: BeatRange { start: 4, end: 12 },
        },
    );

    assert_clip(&state, 5, 0, BeatRange { start: 4, end: 12 });
    assert_clip(&state, 1, 0, BeatRange { start: 0, end: 4 });
    assert_clip(&state, 6, 0, BeatRange { start: 12, end: 16 });
    assert_lane_has_no_overlaps(&state, 0);
}

#[test]
fn moving_clip_cuts_existing_clips_on_target_lane() {
    let mut state = TimelineEditorState::default();

    update_surface(
        &mut state,
        TimelineSurfaceMessage::MoveClip {
            clip_id: 1,
            lane: 1,
            start: 16,
        },
    );

    assert_clip(&state, 1, 1, BeatRange { start: 16, end: 32 });
    assert_clip(&state, 2, 1, BeatRange { start: 12, end: 16 });
    assert_lane_has_no_overlaps(&state, 1);
}

#[test]
fn resizing_clip_cuts_existing_clips_on_same_lane() {
    let mut state = TimelineEditorState::default();
    state.clips.push(TimelineClip::new(TimelineClipParts {
        id: 99,
        name: "Pad tail",
        lane: 1,
        range: BeatRange { start: 30, end: 44 },
    }));

    update_surface(
        &mut state,
        TimelineSurfaceMessage::ResizeClip {
            clip_id: 2,
            range: BeatRange { start: 8, end: 36 },
        },
    );

    assert_clip(&state, 2, 1, BeatRange { start: 8, end: 36 });
    assert_clip(&state, 99, 1, BeatRange { start: 36, end: 44 });
    assert_lane_has_no_overlaps(&state, 1);
}
