//! Platform selection for native external drag launching.

use crate::runtime::{ExternalDragOutcome, ExternalDragRequest};

#[cfg(target_os = "macos")]
pub(super) fn should_launch_before_app_switch(
    armed: bool,
    session_active: bool,
    current_super: bool,
    next_super: bool,
) -> bool {
    armed && session_active && !current_super && next_super
}

#[cfg(not(target_os = "macos"))]
pub(super) const fn should_launch_before_app_switch(
    _armed: bool,
    _session_active: bool,
    _current_super: bool,
    _next_super: bool,
) -> bool {
    false
}

#[cfg(target_os = "windows")]
#[path = "windows.rs"]
mod windows;

#[cfg(target_os = "macos")]
#[path = "macos.rs"]
mod macos;

#[cfg(target_os = "windows")]
pub(super) fn start_external_drag(
    request: &ExternalDragRequest,
) -> Result<ExternalDragOutcome, String> {
    windows::start_external_drag(request)
}

#[cfg(target_os = "macos")]
pub(super) fn start_external_drag(
    request: &ExternalDragRequest,
) -> Result<ExternalDragOutcome, String> {
    macos::start_external_drag(request)
}

#[cfg(not(any(target_os = "windows", target_os = "macos")))]
pub(super) fn start_external_drag(
    _request: &ExternalDragRequest,
) -> Result<ExternalDragOutcome, String> {
    Err(String::from(
        "External drag-out is only supported on Windows and macOS in this backend",
    ))
}
