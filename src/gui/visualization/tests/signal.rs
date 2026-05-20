use super::super::{
    ChannelViewMode, SignalChromeParts, SignalChromeState, SignalRasterPreview,
    SignalRasterPreviewParts, SignalToolFlags, SignalToolState,
};
use crate::gui::types::ImageRgba;
use std::sync::Arc;

#[test]
fn channel_view_mode_distinguishes_combined_and_split_views() {
    assert_ne!(ChannelViewMode::Mono, ChannelViewMode::Stereo);
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
