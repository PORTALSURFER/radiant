//! Platform application activation primitives.

use super::ApplicationActivationMethod;
use winit::event_loop::EventLoopBuilder;

#[cfg(target_os = "macos")]
use objc2::{runtime::NSObjectProtocol, sel};
#[cfg(target_os = "macos")]
use objc2_app_kit::{NSApplication, NSWorkspace};
#[cfg(target_os = "macos")]
use objc2_foundation::MainThreadMarker;
#[cfg(target_os = "macos")]
use winit::platform::macos::EventLoopBuilderExtMacOS;

#[cfg(target_os = "macos")]
pub(super) fn configure_event_loop_activation<T>(
    builder: &mut EventLoopBuilder<T>,
    activate_ignoring_other_apps: bool,
) {
    builder.with_activate_ignoring_other_apps(activate_ignoring_other_apps);
}

#[cfg(not(target_os = "macos"))]
pub(super) fn configure_event_loop_activation<T>(
    _builder: &mut EventLoopBuilder<T>,
    _activate_ignoring_other_apps: bool,
) {
}

#[cfg(target_os = "macos")]
pub(super) fn frontmost_process_id() -> Option<i32> {
    let _main_thread = MainThreadMarker::new()?;
    let workspace = unsafe { NSWorkspace::sharedWorkspace() };
    let application = unsafe { workspace.frontmostApplication()? };
    Some(unsafe { application.processIdentifier() })
}

#[cfg(not(target_os = "macos"))]
pub(super) const fn frontmost_process_id() -> Option<i32> {
    None
}

#[cfg(target_os = "macos")]
pub(super) fn application_is_active() -> bool {
    application().is_some_and(|application| unsafe { application.isActive() })
}

#[cfg(not(target_os = "macos"))]
pub(super) const fn application_is_active() -> bool {
    true
}

#[cfg(target_os = "macos")]
pub(super) fn request_application_activation() -> ApplicationActivationMethod {
    let Some(application) = application() else {
        return ApplicationActivationMethod::Unavailable;
    };
    if application.respondsToSelector(sel!(activate)) {
        unsafe { application.activate() };
        ApplicationActivationMethod::Modern
    } else {
        #[allow(deprecated)]
        application.activateIgnoringOtherApps(true);
        ApplicationActivationMethod::Compatibility
    }
}

#[cfg(not(target_os = "macos"))]
pub(super) const fn request_application_activation() -> ApplicationActivationMethod {
    ApplicationActivationMethod::Unavailable
}

#[cfg(target_os = "macos")]
fn application() -> Option<objc2::rc::Retained<NSApplication>> {
    MainThreadMarker::new().map(NSApplication::sharedApplication)
}
