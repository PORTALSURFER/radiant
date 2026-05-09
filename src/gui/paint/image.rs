use crate::gui::types::{ImageRgba, Rect};
use std::sync::Arc;

/// Textured RGBA image draw primitive stretched into one destination rect.
#[derive(Clone, Debug, PartialEq)]
pub struct DrawImage {
    /// Destination rectangle in logical surface coordinates.
    pub rect: Rect,
    /// RGBA image payload.
    pub image: Arc<ImageRgba>,
}
