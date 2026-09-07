//! Platform selection for native external drag launching.

#[cfg(any(target_os = "windows", test))]
#[path = "text_encoding.rs"]
mod text_encoding;

#[cfg(any(target_os = "windows", test))]
#[path = "url_encoding.rs"]
mod url_encoding;

#[cfg(any(target_os = "windows", target_os = "macos", test))]
#[path = "mime_format.rs"]
mod mime_format;

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
    request.validate_for_native_launch()?;
    windows::start_external_drag(request).map(ExternalDragLaunchDisposition::Completed)
}

#[cfg(target_os = "macos")]
pub(super) fn start_external_drag(
    request: &ExternalDragRequest,
    context: ExternalDragLaunchContext,
) -> Result<ExternalDragLaunchDisposition, String> {
    request.validate_for_native_launch()?;
    macos::start_external_drag(request, context)
}

#[cfg(not(any(target_os = "windows", target_os = "macos")))]
pub(super) fn start_external_drag(
    request: &ExternalDragRequest,
    _context: ExternalDragLaunchContext,
) -> Result<ExternalDragLaunchDisposition, String> {
    request.validate_for_native_launch()?;
    Err(String::from(
        "External drag-out is only supported on Windows and macOS in this backend",
    ))
}

#[cfg(all(test, not(any(target_os = "windows", target_os = "macos"))))]
mod tests {
    use super::*;
    use crate::runtime::{ExternalDragIdentity, ExternalDragRequest};

    #[test]
    fn supported_text_still_reports_unsupported_on_other_targets() {
        let error = start_external_drag(
            &ExternalDragRequest::text("text", "text"),
            ExternalDragLaunchContext::new(None, None, ExternalDragIdentity { id: 1, epoch: 1 }),
        )
        .expect_err("non-native platforms should reject a valid text drag explicitly");

        assert!(error.contains("only supported on Windows and macOS"));
    }
}

#[cfg(test)]
mod validation_tests {
    use super::*;

    #[test]
    fn invalid_text_is_rejected_before_any_native_launch() {
        for text in [
            String::from("before\0after"),
            "x".repeat(crate::runtime::MAX_EXTERNAL_OFFER_TEXT_BYTES + 1),
        ] {
            let error = start_external_drag(
                &ExternalDragRequest::text(text, "invalid"),
                ExternalDragLaunchContext::new(
                    None,
                    None,
                    ExternalDragIdentity { id: 1, epoch: 1 },
                ),
            )
            .expect_err("invalid text must fail before native context access");
            assert!(error.contains("External drag text"));
        }
    }

    #[test]
    fn invalid_url_is_rejected_before_any_native_launch() {
        let error = start_external_drag(
            &ExternalDragRequest::url("relative-url", "invalid"),
            ExternalDragLaunchContext::new(None, None, ExternalDragIdentity { id: 1, epoch: 1 }),
        )
        .expect_err("invalid URL must fail before native context access");
        assert!(error.contains("External drag URL"));
    }

    #[test]
    fn invalid_mime_is_rejected_before_any_native_launch() {
        let error = start_external_drag(
            &ExternalDragRequest::mime("not a MIME type", [], "invalid"),
            ExternalDragLaunchContext::new(None, None, ExternalDragIdentity { id: 1, epoch: 1 }),
        )
        .expect_err("invalid MIME must fail before native context access");
        assert!(error.contains("External drag MIME"));
    }
}
