use super::WindowSpec;
use std::fmt;

/// Error returned when one window descriptor contains invalid geometry.
#[derive(Clone, Debug, PartialEq)]
pub enum WindowSpecError {
    /// Initial or minimum logical size is non-finite or non-positive.
    InvalidSize {
        /// Stable host-owned key for the invalid window.
        key: String,
        /// Name of the invalid size field.
        field: &'static str,
        /// Invalid logical width.
        width: f32,
        /// Invalid logical height.
        height: f32,
    },
    /// Popup position contains a non-finite coordinate.
    InvalidPopupPosition {
        /// Stable host-owned key for the invalid window.
        key: String,
        /// Invalid logical x coordinate.
        x: f32,
        /// Invalid logical y coordinate.
        y: f32,
    },
}

pub(super) fn validate_window_spec(spec: &WindowSpec) -> Result<(), WindowSpecError> {
    validate_size(spec.key.as_str(), "inner_size", spec.options.inner_size)?;
    validate_size(
        spec.key.as_str(),
        "min_inner_size",
        spec.options.min_inner_size,
    )?;
    if let Some(popup) = spec.options.popup_options() {
        validate_position(spec.key.as_str(), popup.position)?;
    }
    Ok(())
}

impl fmt::Display for WindowSpecError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidSize {
                key,
                field,
                width,
                height,
            } => write!(
                formatter,
                "window '{key}' has invalid {field} [{width}, {height}]; logical sizes must be finite and positive"
            ),
            Self::InvalidPopupPosition { key, x, y } => write!(
                formatter,
                "window '{key}' has invalid popup position [{x}, {y}]; popup positions must be finite"
            ),
        }
    }
}

impl std::error::Error for WindowSpecError {}

fn validate_size(
    key: &str,
    field: &'static str,
    size: Option<[f32; 2]>,
) -> Result<(), WindowSpecError> {
    let Some([width, height]) = size else {
        return Ok(());
    };
    if width.is_finite() && height.is_finite() && width > 0.0 && height > 0.0 {
        return Ok(());
    }
    Err(WindowSpecError::InvalidSize {
        key: key.to_string(),
        field,
        width,
        height,
    })
}

fn validate_position(key: &str, position: Option<[f32; 2]>) -> Result<(), WindowSpecError> {
    let Some([x, y]) = position else {
        return Ok(());
    };
    if x.is_finite() && y.is_finite() {
        return Ok(());
    }
    Err(WindowSpecError::InvalidPopupPosition {
        key: key.to_string(),
        x,
        y,
    })
}
