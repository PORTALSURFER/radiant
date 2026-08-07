//! Platform selection for native external drag launching.

use super::ExternalDragLaunchDisposition;
use crate::gui_runtime::native_vello::RuntimeUserEvent;
use crate::runtime::{ExternalDragIdentity, ExternalDragRequest};
use winit::{event_loop::EventLoopProxy, window::WindowId};

pub(super) struct ExternalDragLaunchContext {
    #[cfg_attr(
        not(target_os = "macos"),
        expect(
            dead_code,
            reason = "The launch context is consumed only by the macOS adapter."
        )
    )]
    pub(super) window_id: Option<WindowId>,
    #[cfg_attr(
        not(target_os = "macos"),
        expect(
            dead_code,
            reason = "The launch context is consumed only by the macOS adapter."
        )
    )]
    pub(super) event_proxy: Option<EventLoopProxy<RuntimeUserEvent>>,
    #[cfg_attr(
        not(target_os = "macos"),
        expect(
            dead_code,
            reason = "The launch context is consumed only by the macOS adapter."
        )
    )]
    pub(super) identity: ExternalDragIdentity,
}

impl ExternalDragLaunchContext {
    pub(super) const fn new(
        window_id: Option<WindowId>,
        event_proxy: Option<EventLoopProxy<RuntimeUserEvent>>,
        identity: ExternalDragIdentity,
    ) -> Self {
        Self {
            window_id,
            event_proxy,
            identity,
        }
    }
}

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
    _context: ExternalDragLaunchContext,
) -> Result<ExternalDragLaunchDisposition, String> {
    windows::start_external_drag(request).map(ExternalDragLaunchDisposition::Completed)
}

#[cfg(target_os = "macos")]
pub(super) fn start_external_drag(
    request: &ExternalDragRequest,
    context: ExternalDragLaunchContext,
) -> Result<ExternalDragLaunchDisposition, String> {
    macos::start_external_drag(request, context)
}

#[cfg(not(any(target_os = "windows", target_os = "macos")))]
pub(super) fn start_external_drag(
    _request: &ExternalDragRequest,
    _context: ExternalDragLaunchContext,
) -> Result<ExternalDragLaunchDisposition, String> {
    Err(String::from(
        "External drag-out is only supported on Windows and macOS in this backend",
    ))
}
