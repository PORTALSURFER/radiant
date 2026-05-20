//! Backend-neutral image buffer types.

use std::fmt;
use std::sync::Arc;

#[cfg(test)]
#[path = "image/tests.rs"]
mod tests;

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

/// Error returned when RGBA image bytes do not match the declared dimensions.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ImageRgbaError {
    /// Declared image width in pixels.
    pub width: usize,
    /// Declared image height in pixels.
    pub height: usize,
    /// Actual number of provided bytes.
    pub actual_len: usize,
    /// Expected byte length, or `None` if the dimensions overflow `usize`.
    pub expected_len: Option<usize>,
}

impl ImageRgba {
    /// Create an image buffer from width/height and RGBA8 bytes.
    pub fn new(width: usize, height: usize, pixels: Vec<u8>) -> Option<Self> {
        Self::try_new(width, height, pixels).ok()
    }

    /// Create an image buffer from width/height and RGBA8 bytes.
    ///
    /// This checked constructor returns a diagnostic when the byte payload does
    /// not match the declared row-major RGBA8 dimensions.
    pub fn try_new(width: usize, height: usize, pixels: Vec<u8>) -> Result<Self, ImageRgbaError> {
        let actual_len = pixels.len();
        let expected_len = rgba_byte_len(width, height);
        if expected_len == Some(actual_len) {
            return Ok(Self {
                width,
                height,
                pixels: pixels.into(),
            });
        }
        Err(ImageRgbaError {
            width,
            height,
            actual_len,
            expected_len,
        })
    }
}

impl fmt::Display for ImageRgbaError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.expected_len {
            Some(expected_len) => write!(
                formatter,
                "invalid RGBA image {}x{}: expected {} bytes, got {}",
                self.width, self.height, expected_len, self.actual_len
            ),
            None => write!(
                formatter,
                "invalid RGBA image {}x{}: byte length overflows usize",
                self.width, self.height
            ),
        }
    }
}

impl std::error::Error for ImageRgbaError {}

fn rgba_byte_len(width: usize, height: usize) -> Option<usize> {
    width.checked_mul(height)?.checked_mul(4)
}
