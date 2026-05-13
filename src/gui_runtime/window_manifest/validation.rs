use super::WindowSpec;
use crate::gui_runtime::NativeRunOptionsError;
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
    spec.options
        .validate()
        .map_err(|err| WindowSpecError::from_native_options_error(spec.key.as_str(), err))
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

impl WindowSpecError {
    fn from_native_options_error(key: &str, error: NativeRunOptionsError) -> Self {
        match error {
            NativeRunOptionsError::InvalidSize {
                field,
                width,
                height,
            } => Self::InvalidSize {
                key: key.to_string(),
                field,
                width,
                height,
            },
            NativeRunOptionsError::InvalidPopupPosition { x, y, .. } => {
                Self::InvalidPopupPosition {
                    key: key.to_string(),
                    x,
                    y,
                }
            }
        }
    }
}
