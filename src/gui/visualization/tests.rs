use super::{
    CanvasInvalidation, CanvasLayer, CanvasLayerOrder, CanvasLayerParts, ChannelViewMode,
    DragHandle, DragHandleRole, PointRenderMode, SignalChromeParts, SignalChromeState,
    SignalRasterPreview, SignalRasterPreviewParts, SignalToolFlags, SignalToolState, SpatialPanel,
    SpatialPoint, TimelineCoordinateMapper, TimelineEditPreview, TimelineEditPreviewParts,
    TimelineFeedbackEvents, TimelineMarkerPreview, TimelineMotionState, TimelinePresentationState,
    TimelineSurfaceParts, TimelineSurfaceState, TimelineTransportState, TimelineViewport,
    canvas_layer_at_point, drag_handle_at_point, normalized_milli_point_in_rect,
};
use crate::gui::{
    range::{NormalizedPixelSnap, NormalizedRange, NormalizedViewport},
    types::{ImageRgba, Point, Rect},
};
use std::sync::Arc;

#[test]
fn point_render_mode_defaults_to_points() {
    assert_eq!(PointRenderMode::default(), PointRenderMode::Points);
}

#[test]
fn channel_view_mode_distinguishes_combined_and_split_views() {
    assert_ne!(ChannelViewMode::Mono, ChannelViewMode::Stereo);
}

#[test]
fn spatial_point_preserves_normalized_coordinates_and_id() {
    let point = SpatialPoint {
        id: Arc::<str>::from("item-1"),
        x_milli: 250,
        y_milli: 750,
        cluster_id: Some(3),
    };

    assert_eq!(point.id.as_ref(), "item-1");
    assert_eq!(point.x_milli, 250);
    assert_eq!(point.y_milli, 750);
    assert_eq!(point.cluster_id, Some(3));
}

#[test]
fn spatial_panel_defaults_to_inactive_empty_points() {
    let panel = SpatialPanel::default();

    assert!(!panel.active);
    assert_eq!(panel.render_mode, PointRenderMode::Points);
    assert!(panel.points.is_empty());
    assert_eq!(panel.selected_item_id, None);
    assert_eq!(panel.focused_item_id, None);
}

#[test]
fn normalized_milli_point_projects_and_clamps_into_rect() {
    let rect = Rect::from_min_max(Point::new(100.0, 200.0), Point::new(300.0, 500.0));

    assert_eq!(
        normalized_milli_point_in_rect(rect, 250, 500),
        Point::new(150.0, 350.0)
    );
    assert_eq!(normalized_milli_point_in_rect(rect, 1400, 1300), rect.max);
}

#[test]
fn canvas_layer_hit_testing_prefers_topmost_interactive_layer() {
    let bounds = Rect::from_min_max(Point::new(0.0, 0.0), Point::new(100.0, 100.0));
    let layers = [
        CanvasLayer::from_parts(CanvasLayerParts {
            id: String::from("base"),
            order: CanvasLayerOrder::Background,
            bounds,
            interactive: true,
        }),
        CanvasLayer::from_parts(CanvasLayerParts {
            id: String::from("paint"),
            order: CanvasLayerOrder::Content,
            bounds,
            interactive: false,
        }),
        CanvasLayer::from_parts(CanvasLayerParts {
            id: String::from("handle"),
            order: CanvasLayerOrder::Interaction,
            bounds: Rect::from_min_max(Point::new(40.0, 40.0), Point::new(60.0, 60.0)),
            interactive: true,
        }),
        CanvasLayer::from_parts(CanvasLayerParts {
            id: String::from("focus"),
            order: CanvasLayerOrder::Focus,
            bounds: Rect::from_min_max(Point::new(45.0, 45.0), Point::new(55.0, 55.0)),
            interactive: true,
        }),
    ];

    assert_eq!(
        canvas_layer_at_point(&layers, Point::new(50.0, 50.0)),
        Some("focus")
    );
    assert_eq!(
        canvas_layer_at_point(&layers, Point::new(20.0, 20.0)),
        Some("base")
    );
    assert_eq!(
        canvas_layer_at_point(&layers, Point::new(120.0, 20.0)),
        None
    );
}

#[test]
fn drag_handle_hit_testing_uses_reverse_paint_order_and_enabled_state() {
    let handles = [
        DragHandle::new(
            DragHandleRole::Body,
            Rect::from_min_max(Point::new(10.0, 10.0), Point::new(50.0, 30.0)),
            1,
        ),
        DragHandle::new(
            DragHandleRole::Start,
            Rect::from_min_max(Point::new(10.0, 10.0), Point::new(20.0, 30.0)),
            2,
        )
        .with_enabled(false),
        DragHandle::new(
            DragHandleRole::End,
            Rect::from_min_max(Point::new(40.0, 10.0), Point::new(50.0, 30.0)),
            3,
        ),
    ];

    assert_eq!(
        drag_handle_at_point(&handles, Point::new(45.0, 20.0)).map(|handle| handle.role),
        Some(DragHandleRole::End)
    );
    assert_eq!(
        drag_handle_at_point(&handles, Point::new(15.0, 20.0)).map(|handle| handle.role),
        Some(DragHandleRole::Body)
    );
    assert_eq!(drag_handle_at_point(&handles, Point::new(5.0, 20.0)), None);
}

#[test]
fn timeline_coordinate_mapper_projects_and_back_projects_micros() {
    let viewport = TimelineViewport::new(250, 750, 250_000, 750_000, 250_000_000, 750_000_000);
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
    let viewport = TimelineViewport::new(250, 750, 250_000, 750_000, 250_000_000, 750_000_000);
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
fn canvas_invalidation_splits_scene_and_interaction_rebuilds() {
    let interaction = CanvasInvalidation {
        interaction_changed: true,
        ..CanvasInvalidation::default()
    };
    let projection = CanvasInvalidation {
        projection_changed: true,
        ..CanvasInvalidation::default()
    };

    assert!(!interaction.requires_scene_rebuild());
    assert!(interaction.requires_interaction_overlay_rebuild());
    assert!(projection.requires_scene_rebuild());
    assert!(projection.requires_interaction_overlay_rebuild());
}

#[test]
fn signal_raster_preview_preserves_label_flags_signature_and_image() {
    let image = Arc::new(ImageRgba::new(1, 1, vec![255, 0, 0, 255]).unwrap());
    let preview = SignalRasterPreview::from_parts(SignalRasterPreviewParts {
        loaded_label: Some(String::from("preview")),
        loading: true,
        image_rendering: false,
        image_signature: Some(42),
        image: Some(Arc::clone(&image)),
    });

    assert_eq!(preview.loaded_label.as_deref(), Some("preview"));
    assert!(preview.loading);
    assert!(!preview.image_rendering);
    assert_eq!(preview.image_signature, Some(42));
    assert_eq!(preview.image.as_deref(), Some(image.as_ref()));
}

#[test]
fn signal_chrome_state_preserves_status_reference_and_channel_view() {
    let chrome = SignalChromeState::from_parts(SignalChromeParts {
        status_hint: String::from("playing"),
        reference_anchor_available: true,
        reference_anchor_label: Some(String::from("A")),
        channel_view: ChannelViewMode::Stereo,
    });

    assert_eq!(chrome.status_hint, "playing");
    assert!(chrome.reference_anchor_available);
    assert_eq!(chrome.reference_anchor_label.as_deref(), Some("A"));
    assert_eq!(chrome.channel_view, ChannelViewMode::Stereo);
}

#[test]
fn signal_tool_state_preserves_generic_interaction_flags() {
    let tools = SignalToolState::from_flags(SignalToolFlags {
        lock_enabled: true,
        alternate_preview_enabled: true,
        primary_snap_enabled: false,
        relative_grid_enabled: true,
        secondary_snap_enabled: false,
        markers_visible: true,
        marker_mode_enabled: true,
        batch_action_available: false,
    });

    assert!(tools.lock_enabled);
    assert!(tools.alternate_preview_enabled);
    assert!(!tools.primary_snap_enabled);
    assert!(tools.relative_grid_enabled);
    assert!(!tools.secondary_snap_enabled);
    assert!(tools.markers_visible);
    assert!(tools.marker_mode_enabled);
    assert!(!tools.batch_action_available);
}

#[test]
fn timeline_transport_state_preserves_positions_and_resolves_micro_playhead() {
    let selection = NormalizedRange::new(100, 400);
    let transport = TimelineTransportState::new(Some(120), Some(250), None, Some(selection));

    assert_eq!(transport.cursor_milli, Some(120));
    assert_eq!(transport.playhead_milli, Some(250));
    assert_eq!(transport.resolved_playhead_micros(), Some(250_000));
    assert_eq!(transport.selection, Some(selection));

    let precise = TimelineTransportState::new(None, Some(250), Some(250_125), None);
    assert_eq!(precise.resolved_playhead_micros(), Some(250_125));
}

#[test]
fn timeline_feedback_events_preserve_operation_tokens() {
    let events = TimelineFeedbackEvents::new(10, 20, 30);

    assert_eq!(events.primary_success_nonce, 10);
    assert_eq!(events.primary_failure_nonce, 20);
    assert_eq!(events.secondary_success_nonce, 30);
}

#[test]
fn timeline_presentation_state_preserves_guides_repeat_and_labels() {
    let presentation = TimelinePresentationState::new(
        Some(125_000),
        10_000,
        true,
        Some(String::from("Guide 1")),
        Some(String::from("2x")),
    );

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
        viewport: TimelineViewport::new(10, 900, 10_000, 900_000, 10_000_000, 900_000_000),
        transport: TimelineTransportState::new(Some(20), Some(30), Some(30_500), None),
        edit_preview: TimelineEditPreview::default(),
        feedback_events: TimelineFeedbackEvents::new(1, 2, 3),
        presentation: TimelinePresentationState::new(
            None,
            0,
            true,
            Some(String::from("tempo")),
            None,
        ),
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
            viewport: TimelineViewport::new(0, 500, 0, 500_000, 0, 500_000_000),
            transport: TimelineTransportState::new(None, Some(10), Some(10_250), None),
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
