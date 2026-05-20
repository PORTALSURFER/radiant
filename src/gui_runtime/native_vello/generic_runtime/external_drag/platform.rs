//! Platform selection for native external drag launching.

use crate::runtime::{ExternalDragOutcome, ExternalDragRequest};

#[cfg(target_os = "windows")]
#[path = "windows.rs"]
mod windows;

#[cfg(target_os = "windows")]
pub(super) fn start_external_drag(
    request: &ExternalDragRequest,
) -> Result<ExternalDragOutcome, String> {
    windows::start_external_drag(request)
}

#[cfg(not(target_os = "windows"))]
pub(super) fn start_external_drag(
    _request: &ExternalDragRequest,
) -> Result<ExternalDragOutcome, String> {
    Err(String::from(
        "External drag-out is only supported on Windows in this backend",
    ))
}
