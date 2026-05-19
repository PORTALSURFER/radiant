//! Generic signal visualization state.

use std::sync::Arc;

use crate::gui::types::ImageRgba;

/// Channel layout for visualizing one stream as a combined or split view.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChannelViewMode {
    /// Collapse channels into one combined envelope.
    Mono,
    /// Render channels in a split stereo view.
    Stereo,
}

/// Explicit parts used to build retained raster preview state.
///
/// This keeps cache identity, loading flags, labels, and image payloads readable
/// at app-facing projection call sites.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct SignalRasterPreviewParts {
    /// Display label for the loaded item, when any.
    pub loaded_label: Option<String>,
    /// Whether the preview is waiting for new input content.
    pub loading: bool,
    /// Whether a replacement image is still rendering in the background.
    pub image_rendering: bool,
    /// Stable signature for detecting image updates.
    pub image_signature: Option<u64>,
    /// Optional rasterized image payload.
    pub image: Option<Arc<ImageRgba>>,
}

/// Retained raster preview for a timeline, signal, or visualization surface.
///
/// Hosts may render expensive visualization content into an image, project a
/// stable signature for cache invalidation, and keep lightweight labels/loading
/// state alongside the shared pixel payload.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct SignalRasterPreview {
    /// Display label for the loaded item, when any.
    pub loaded_label: Option<String>,
    /// Whether the preview is waiting for new input content.
    pub loading: bool,
    /// Whether a replacement image is still rendering in the background.
    pub image_rendering: bool,
    /// Stable signature for detecting image updates.
    pub image_signature: Option<u64>,
    /// Optional rasterized image payload.
    pub image: Option<Arc<ImageRgba>>,
}

/// Explicit parts used to build generic signal chrome state.
///
/// This avoids positional status/reference/channel construction as the generic
/// visualization chrome grows.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SignalChromeParts {
    /// Extra status hint shown alongside visualization labels.
    pub status_hint: String,
    /// Whether a host-defined reference anchor is currently available.
    pub reference_anchor_available: bool,
    /// Label for the host-defined reference anchor, when available.
    pub reference_anchor_label: Option<String>,
    /// Channel layout used by the signal visualization.
    pub channel_view: ChannelViewMode,
}

impl Default for SignalChromeParts {
    fn default() -> Self {
        Self {
            status_hint: String::from("idle"),
            reference_anchor_available: false,
            reference_anchor_label: None,
            channel_view: ChannelViewMode::Mono,
        }
    }
}

/// Generic chrome/status state for a signal visualization surface.
///
/// This captures reusable display state such as a transport/status hint,
/// optional reference-anchor metadata, and channel layout. Host-specific tools
/// and edit modes should remain in host state or compatibility adapters.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SignalChromeState {
    /// Extra status hint shown alongside visualization labels.
    pub status_hint: String,
    /// Whether a host-defined reference anchor is currently available.
    pub reference_anchor_available: bool,
    /// Label for the host-defined reference anchor, when available.
    pub reference_anchor_label: Option<String>,
    /// Channel layout used by the signal visualization.
    pub channel_view: ChannelViewMode,
}

/// Generic enabled/visible tool state for a signal visualization surface.
///
/// The fields intentionally describe interaction roles rather than domain
/// operations. Hosts map these booleans to product-specific tools such as snap
/// modes, overlays, review modes, or cleanup availability.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SignalToolState {
    /// Whether the visualization's current mode is locked against host updates.
    pub lock_enabled: bool,
    /// Whether alternate preview behavior is enabled.
    pub alternate_preview_enabled: bool,
    /// Whether the primary snap behavior is enabled.
    pub primary_snap_enabled: bool,
    /// Whether grid/guide alignment uses a relative anchor.
    pub relative_grid_enabled: bool,
    /// Whether the secondary snap behavior is enabled.
    pub secondary_snap_enabled: bool,
    /// Whether marker overlays are visible.
    pub markers_visible: bool,
    /// Whether marker editing mode is active.
    pub marker_mode_enabled: bool,
    /// Whether a host-defined batch action is available.
    pub batch_action_available: bool,
}

/// Explicit flags used to build signal visualization tool state.
///
/// Prefer this over positional boolean constructors so host projections remain
/// readable as the generic visualization model grows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SignalToolFlags {
    /// Whether the visualization's current mode is locked against host updates.
    pub lock_enabled: bool,
    /// Whether alternate preview behavior is enabled.
    pub alternate_preview_enabled: bool,
    /// Whether the primary snap behavior is enabled.
    pub primary_snap_enabled: bool,
    /// Whether grid/guide alignment uses a relative anchor.
    pub relative_grid_enabled: bool,
    /// Whether the secondary snap behavior is enabled.
    pub secondary_snap_enabled: bool,
    /// Whether marker overlays are visible.
    pub markers_visible: bool,
    /// Whether marker editing mode is active.
    pub marker_mode_enabled: bool,
    /// Whether a host-defined batch action is available.
    pub batch_action_available: bool,
}

impl Default for SignalToolFlags {
    fn default() -> Self {
        Self {
            lock_enabled: false,
            alternate_preview_enabled: false,
            primary_snap_enabled: false,
            relative_grid_enabled: false,
            secondary_snap_enabled: false,
            markers_visible: true,
            marker_mode_enabled: false,
            batch_action_available: false,
        }
    }
}

impl Default for SignalToolState {
    fn default() -> Self {
        Self::from_flags(SignalToolFlags::default())
    }
}

impl SignalToolState {
    /// Build signal tool state from explicitly named generic flags.
    pub fn from_flags(flags: SignalToolFlags) -> Self {
        Self {
            lock_enabled: flags.lock_enabled,
            alternate_preview_enabled: flags.alternate_preview_enabled,
            primary_snap_enabled: flags.primary_snap_enabled,
            relative_grid_enabled: flags.relative_grid_enabled,
            secondary_snap_enabled: flags.secondary_snap_enabled,
            markers_visible: flags.markers_visible,
            marker_mode_enabled: flags.marker_mode_enabled,
            batch_action_available: flags.batch_action_available,
        }
    }
}

impl Default for SignalChromeState {
    fn default() -> Self {
        Self::from_parts(SignalChromeParts::default())
    }
}

impl SignalChromeState {
    /// Build signal chrome state from named generic display parts.
    pub fn from_parts(parts: SignalChromeParts) -> Self {
        Self {
            status_hint: parts.status_hint,
            reference_anchor_available: parts.reference_anchor_available,
            reference_anchor_label: parts.reference_anchor_label,
            channel_view: parts.channel_view,
        }
    }

    /// Build signal chrome state from explicit display values.
    pub fn new(
        status_hint: impl Into<String>,
        reference_anchor_available: bool,
        reference_anchor_label: Option<String>,
        channel_view: ChannelViewMode,
    ) -> Self {
        Self::from_parts(SignalChromeParts {
            status_hint: status_hint.into(),
            reference_anchor_available,
            reference_anchor_label,
            channel_view,
        })
    }
}

impl SignalRasterPreview {
    /// Build a retained raster preview from named generic parts.
    pub fn from_parts(parts: SignalRasterPreviewParts) -> Self {
        Self {
            loaded_label: parts.loaded_label,
            loading: parts.loading,
            image_rendering: parts.image_rendering,
            image_signature: parts.image_signature,
            image: parts.image,
        }
    }

    /// Build a retained raster preview from explicit state.
    pub fn new(
        loaded_label: Option<String>,
        loading: bool,
        image_rendering: bool,
        image_signature: Option<u64>,
        image: Option<Arc<ImageRgba>>,
    ) -> Self {
        Self::from_parts(SignalRasterPreviewParts {
            loaded_label,
            loading,
            image_rendering,
            image_signature,
            image,
        })
    }
}
