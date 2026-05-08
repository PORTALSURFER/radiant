//! Backend-neutral image buffer types.

use std::sync::Arc;

/// Owned RGBA image buffer used by the GUI layer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ImageRgba {
    /// Image width in pixels.
    pub width: usize,
    /// Image height in pixels.
    pub height: usize,
    /// Shared packed RGBA8 pixels in row-major order.
    ///
    /// Cloning `ImageRgba` reuses this backing storage to avoid deep payload copies.
    pub pixels: Arc<[u8]>,
}

impl ImageRgba {
    /// Create an image buffer from width/height and RGBA8 bytes.
    pub fn new(width: usize, height: usize, pixels: Vec<u8>) -> Option<Self> {
        if pixels.len() != width.saturating_mul(height).saturating_mul(4) {
            return None;
        }
        Some(Self {
            width,
            height,
            pixels: pixels.into(),
        })
    }
}
