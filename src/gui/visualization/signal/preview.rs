use std::sync::Arc;

use crate::gui::types::ImageRgba;

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
