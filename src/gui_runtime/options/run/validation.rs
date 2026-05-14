use std::fmt;

/// Error returned when native launch options contain invalid geometry.
#[derive(Clone, Debug, PartialEq)]
pub enum NativeRunOptionsError {
    /// Initial or minimum logical size is non-finite or non-positive.
    InvalidSize {
        /// Name of the invalid size field.
        field: &'static str,
        /// Invalid logical width.
        width: f32,
        /// Invalid logical height.
        height: f32,
    },
    /// Popup position contains a non-finite coordinate.
    InvalidPopupPosition {
        /// Name of the invalid position field.
        field: &'static str,
        /// Invalid logical x coordinate.
        x: f32,
        /// Invalid logical y coordinate.
        y: f32,
    },
    /// Popup drag region height is non-finite or negative.
    InvalidPopupDragRegionHeight {
        /// Invalid logical drag-region height.
        height: f32,
    },
}

impl fmt::Display for NativeRunOptionsError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidSize {
                field,
                width,
                height,
            } => write!(
                formatter,
                "invalid native {field} [{width}, {height}]; logical sizes must be finite and positive"
            ),
            Self::InvalidPopupPosition { field, x, y } => write!(
                formatter,
                "invalid native {field} [{x}, {y}]; popup positions must be finite"
            ),
            Self::InvalidPopupDragRegionHeight { height } => write!(
                formatter,
                "invalid native popup drag region height [{height}]; height must be finite and non-negative"
            ),
        }
    }
}

impl std::error::Error for NativeRunOptionsError {}

pub(super) fn validate_size(
    field: &'static str,
    size: Option<[f32; 2]>,
) -> Result<(), NativeRunOptionsError> {
    let Some([width, height]) = size else {
        return Ok(());
    };
    if width.is_finite() && height.is_finite() && width > 0.0 && height > 0.0 {
        return Ok(());
    }
    Err(NativeRunOptionsError::InvalidSize {
        field,
        width,
        height,
    })
}

pub(super) fn validate_position(
    field: &'static str,
    position: Option<[f32; 2]>,
) -> Result<(), NativeRunOptionsError> {
    let Some([x, y]) = position else {
        return Ok(());
    };
    if x.is_finite() && y.is_finite() {
        return Ok(());
    }
    Err(NativeRunOptionsError::InvalidPopupPosition { field, x, y })
}

pub(super) fn validate_popup_drag_region(height: Option<f32>) -> Result<(), NativeRunOptionsError> {
    let Some(height) = height else {
        return Ok(());
    };
    if height.is_finite() && height >= 0.0 {
        return Ok(());
    }
    Err(NativeRunOptionsError::InvalidPopupDragRegionHeight { height })
}
