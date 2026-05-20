use super::*;

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
