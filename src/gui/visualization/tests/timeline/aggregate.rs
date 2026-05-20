use super::{super::super::*, fixtures::timeline_viewport_parts};
use crate::gui::range::NormalizedRange;

#[test]
fn timeline_surface_state_aggregates_generic_timeline_parts() {
    let marker = TimelineMarkerPreview {
        range: NormalizedRange::new(100, 200),
        selected: true,
        focused: false,
    };
    let surface = TimelineSurfaceState::from_parts(TimelineSurfaceParts {
        viewport: TimelineViewport::from_parts(timeline_viewport_parts(
            10,
            900,
            10_000,
            900_000,
            10_000_000,
            900_000_000,
        )),
        transport: TimelineTransportState::from_parts(TimelineTransportParts {
            cursor_milli: Some(20),
            playhead_milli: Some(30),
            playhead_micros: Some(30_500),
            selection: None,
        }),
        edit_preview: TimelineEditPreview::default(),
        feedback_events: TimelineFeedbackEvents::from_parts(TimelineFeedbackParts {
            primary_success_nonce: 1,
            primary_failure_nonce: 2,
            secondary_success_nonce: 3,
        }),
        presentation: TimelinePresentationState::from_parts(TimelinePresentationParts {
            guide_step_micros: None,
            guide_origin_micros: 0,
            repeat_enabled: true,
            primary_label: Some(String::from("tempo")),
            viewport_label: None,
        }),
        raster_preview: SignalRasterPreview::default(),
        markers: vec![marker],
    });

    assert_eq!(surface.viewport.start_micros, 10_000);
    assert_eq!(surface.transport.resolved_playhead_micros(), Some(30_500));
    assert_eq!(surface.feedback_events.primary_failure_nonce, 2);
    assert!(surface.presentation.repeat_enabled);
    assert_eq!(surface.markers.len(), 1);
}

#[test]
fn timeline_motion_state_aggregates_surface_chrome_tools_and_transport() {
    let motion = TimelineMotionState::new(
        true,
        TimelineSurfaceState::from_parts(TimelineSurfaceParts {
            viewport: TimelineViewport::from_parts(timeline_viewport_parts(
                0,
                500,
                0,
                500_000,
                0,
                500_000_000,
            )),
            transport: TimelineTransportState::from_parts(TimelineTransportParts {
                cursor_milli: None,
                playhead_milli: Some(10),
                playhead_micros: Some(10_250),
                selection: None,
            }),
            edit_preview: TimelineEditPreview::default(),
            feedback_events: TimelineFeedbackEvents::default(),
            presentation: TimelinePresentationState::default(),
            raster_preview: SignalRasterPreview::default(),
            markers: Vec::<TimelineMarkerPreview>::new(),
        }),
        SignalChromeState::from_parts(SignalChromeParts {
            status_hint: String::from("moving"),
            reference_anchor_available: true,
            reference_anchor_label: Some(String::from("anchor")),
            channel_view: ChannelViewMode::Mono,
        }),
        SignalToolState::from_flags(SignalToolFlags {
            lock_enabled: false,
            alternate_preview_enabled: true,
            primary_snap_enabled: true,
            relative_grid_enabled: false,
            secondary_snap_enabled: true,
            markers_visible: true,
            marker_mode_enabled: false,
            batch_action_available: true,
        }),
    );

    assert!(motion.transport_running);
    assert_eq!(motion.surface.viewport.end_micros, 500_000);
    assert_eq!(
        motion.surface.transport.resolved_playhead_micros(),
        Some(10_250)
    );
    assert_eq!(motion.chrome.status_hint, "moving");
    assert!(motion.chrome.reference_anchor_available);
    assert!(motion.tools.alternate_preview_enabled);
    assert!(motion.tools.batch_action_available);
}
