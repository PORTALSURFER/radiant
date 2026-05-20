use super::super::{
    ChannelViewMode, SignalChromeParts, SignalChromeState, SignalRasterPreview, SignalToolFlags,
    SignalToolState, TimelineCoordinateMapper, TimelineEditPreview, TimelineEditPreviewParts,
    TimelineFeedbackEvents, TimelineFeedbackParts, TimelineMarkerPreview, TimelineMotionState,
    TimelinePresentationParts, TimelinePresentationState, TimelineSurfaceParts,
    TimelineSurfaceState, TimelineTransportParts, TimelineTransportState, TimelineViewport,
    TimelineViewportParts,
};
use crate::gui::{
    range::{NormalizedPixelSnap, NormalizedRange, NormalizedViewport},
    types::{Point, Rect},
};

fn timeline_viewport_parts(
    start_milli: u16,
    end_milli: u16,
    start_micros: u32,
    end_micros: u32,
    start_nanos: u32,
    end_nanos: u32,
) -> TimelineViewportParts {
    TimelineViewportParts {
        start_milli,
        end_milli,
        start_micros,
        end_micros,
        start_nanos,
        end_nanos,
    }
}

#[test]
fn timeline_coordinate_mapper_projects_and_back_projects_micros() {
    let viewport = TimelineViewport::from_parts(timeline_viewport_parts(
        250,
        750,
        250_000,
        750_000,
        250_000_000,
        750_000_000,
    ));
    let rect = Rect::from_min_max(Point::new(10.0, 0.0), Point::new(210.0, 40.0));
    let mapper = TimelineCoordinateMapper::new(viewport, rect, NormalizedPixelSnap::Nearest);

    assert_eq!(
        viewport.normalized_viewport(),
        NormalizedViewport::from_micros(250_000, 750_000)
    );
    assert_eq!(mapper.x_for_micros(250_000), 10.0);
    assert_eq!(mapper.x_for_micros(500_000), 110.0);
    assert_eq!(
        mapper.x_range_for(NormalizedRange::from_micros(300_000, 700_000)),
        (30.0, 190.0)
    );
    assert_eq!(mapper.micros_for_x(110.0), 500_000);
}

#[test]
fn timeline_coordinate_mapper_sanitizes_invalid_back_projection_inputs() {
    let viewport = TimelineViewport::from_parts(timeline_viewport_parts(
        250,
        750,
        250_000,
        750_000,
        250_000_000,
        750_000_000,
    ));
    let rect = Rect::from_min_max(Point::new(10.0, 0.0), Point::new(210.0, 40.0));
    let mapper = TimelineCoordinateMapper::new(viewport, rect, NormalizedPixelSnap::Nearest);
    let invalid_rect_mapper = TimelineCoordinateMapper::new(
        viewport,
        Rect::from_min_max(Point::new(f32::NAN, 0.0), Point::new(210.0, 40.0)),
        NormalizedPixelSnap::Nearest,
    );

    assert_eq!(mapper.micros_for_x(f32::NAN), 250_000);
    assert_eq!(mapper.micros_for_x(f32::INFINITY), 250_000);
    assert_eq!(invalid_rect_mapper.micros_for_x(110.0), 250_000);
}

#[test]
fn timeline_transport_state_preserves_positions_and_resolves_micro_playhead() {
    let selection = NormalizedRange::new(100, 400);
    let transport = TimelineTransportState::from_parts(TimelineTransportParts {
        cursor_milli: Some(120),
        playhead_milli: Some(250),
        playhead_micros: None,
        selection: Some(selection),
    });

    assert_eq!(transport.cursor_milli, Some(120));
    assert_eq!(transport.playhead_milli, Some(250));
    assert_eq!(transport.resolved_playhead_micros(), Some(250_000));
    assert_eq!(transport.selection, Some(selection));

    let precise = TimelineTransportState::from_parts(TimelineTransportParts {
        cursor_milli: None,
        playhead_milli: Some(250),
        playhead_micros: Some(250_125),
        selection: None,
    });
    assert_eq!(precise.resolved_playhead_micros(), Some(250_125));
}

#[test]
fn timeline_feedback_events_preserve_operation_tokens() {
    let events = TimelineFeedbackEvents::from_parts(TimelineFeedbackParts {
        primary_success_nonce: 10,
        primary_failure_nonce: 20,
        secondary_success_nonce: 30,
    });

    assert_eq!(events.primary_success_nonce, 10);
    assert_eq!(events.primary_failure_nonce, 20);
    assert_eq!(events.secondary_success_nonce, 30);
}

#[test]
fn timeline_presentation_state_preserves_guides_repeat_and_labels() {
    let presentation = TimelinePresentationState::from_parts(TimelinePresentationParts {
        guide_step_micros: Some(125_000),
        guide_origin_micros: 10_000,
        repeat_enabled: true,
        primary_label: Some(String::from("Guide 1")),
        viewport_label: Some(String::from("2x")),
    });

    assert_eq!(presentation.guide_step_micros, Some(125_000));
    assert_eq!(presentation.guide_origin_micros, 10_000);
    assert!(presentation.repeat_enabled);
    assert_eq!(presentation.primary_label.as_deref(), Some("Guide 1"));
    assert_eq!(presentation.viewport_label.as_deref(), Some("2x"));
}

#[test]
fn timeline_viewport_defaults_to_full_normalized_range() {
    let viewport = TimelineViewport::default();

    assert_eq!(viewport.start_milli, 0);
    assert_eq!(viewport.end_milli, 1000);
    assert_eq!(viewport.start_micros, 0);
    assert_eq!(viewport.end_micros, 1_000_000);
    assert_eq!(viewport.start_nanos, 0);
    assert_eq!(viewport.end_nanos, 1_000_000_000);
}

#[test]
fn timeline_edit_preview_preserves_selection_and_handle_positions() {
    let selection = NormalizedRange {
        start_milli: 200,
        end_milli: 800,
        start_micros: 200_000,
        end_micros: 800_000,
        start_nanos: 200_000_000,
        end_nanos: 800_000_000,
    };
    let preview = TimelineEditPreview::from_parts(TimelineEditPreviewParts {
        selection: Some(selection),
        leading_end_milli: Some(300),
        leading_end_micros: Some(300_000),
        leading_inner_start_milli: Some(240),
        leading_inner_start_micros: Some(240_000),
        leading_curve_milli: Some(420),
        trailing_start_milli: Some(700),
        trailing_start_micros: Some(700_000),
        trailing_inner_end_milli: Some(760),
        trailing_inner_end_micros: Some(760_000),
        trailing_curve_milli: Some(580),
    });

    assert_eq!(preview.selection, Some(selection));
    assert_eq!(preview.leading_end_micros, Some(300_000));
    assert_eq!(preview.leading_inner_start_milli, Some(240));
    assert_eq!(preview.leading_curve_milli, Some(420));
    assert_eq!(preview.trailing_start_milli, Some(700));
    assert_eq!(preview.trailing_inner_end_micros, Some(760_000));
    assert_eq!(preview.trailing_curve_milli, Some(580));
}

#[test]
fn timeline_marker_preview_preserves_range_and_focus_state() {
    let marker = TimelineMarkerPreview {
        range: NormalizedRange {
            start_milli: 100,
            end_milli: 200,
            start_micros: 100_000,
            end_micros: 200_000,
            start_nanos: 100_000_000,
            end_nanos: 200_000_000,
        },
        selected: true,
        focused: false,
    };

    assert_eq!(marker.range.start_micros, 100_000);
    assert!(marker.selected);
    assert!(!marker.focused);
}

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
