use super::{super::super::*, fixtures::timeline_viewport_parts};
use crate::gui::range::{IndexViewport, NormalizedRange};

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
fn timeline_viewport_from_parts_preserves_multiresolution_bounds() {
    let viewport = TimelineViewport::from_parts(timeline_viewport_parts(
        10,
        900,
        10_000,
        900_000,
        10_000_000,
        900_000_000,
    ));

    assert_eq!(viewport.start_milli, 10);
    assert_eq!(viewport.end_milli, 900);
    assert_eq!(viewport.start_micros, 10_000);
    assert_eq!(viewport.end_nanos, 900_000_000);
}

#[test]
fn timeline_viewport_from_index_viewport_projects_integer_bounds() {
    let viewport = TimelineViewport::from_index_viewport(IndexViewport { start: 25, end: 75 }, 200);

    assert_eq!(viewport.start_milli, 125);
    assert_eq!(viewport.end_milli, 375);
    assert_eq!(viewport.start_micros, 125_000);
    assert_eq!(viewport.end_micros, 375_000);
    assert_eq!(viewport.start_nanos, 125_000_000);
    assert_eq!(viewport.end_nanos, 375_000_000);
}
