use crate::gui::types::ImageRgba;
use std::sync::Arc;

/// Immutable public properties for a reusable image widget.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ImageProps {
    /// Shared RGBA image payload.
    pub image: Arc<ImageRgba>,
}
