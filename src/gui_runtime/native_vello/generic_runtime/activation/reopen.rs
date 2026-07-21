//! Explicit macOS application-reopen intent routing.

use super::super::RuntimeUserEvent;
use winit::event_loop::EventLoopProxy;

pub(super) fn install_application_reopen_handler(
    proxy: EventLoopProxy<RuntimeUserEvent>,
) -> ApplicationReopenRegistration {
    platform::install_application_reopen_handler(proxy)
}

#[cfg(target_os = "macos")]
pub(in crate::gui_runtime::native_vello::generic_runtime) struct ApplicationReopenRegistration {
    refcon: *mut EventLoopProxy<RuntimeUserEvent>,
    installed: bool,
}

#[cfg(target_os = "macos")]
impl Drop for ApplicationReopenRegistration {
    fn drop(&mut self) {
        platform::uninstall_application_reopen_handler(self);
    }
}

#[cfg(not(target_os = "macos"))]
pub(in crate::gui_runtime::native_vello::generic_runtime) struct ApplicationReopenRegistration;

#[cfg(not(target_os = "macos"))]
impl Drop for ApplicationReopenRegistration {
    fn drop(&mut self) {}
}

#[cfg(target_os = "macos")]
mod platform {
    use super::{ApplicationReopenRegistration, RuntimeUserEvent};
    use std::{ffi::c_void, ptr};
    use winit::event_loop::EventLoopProxy;

    type OSType = u32;
    type OSErr = i16;
    type Boolean = u8;

    type AppleEventHandler =
        Option<unsafe extern "C" fn(*const c_void, *mut c_void, isize) -> OSErr>;

    #[link(name = "ApplicationServices", kind = "framework")]
    unsafe extern "C" {
        fn AEInstallEventHandler(
            event_class: OSType,
            event_id: OSType,
            handler: AppleEventHandler,
            handler_refcon: isize,
            is_system_handler: Boolean,
        ) -> OSErr;
        fn AERemoveEventHandler(
            event_class: OSType,
            event_id: OSType,
            handler: AppleEventHandler,
            is_system_handler: Boolean,
        ) -> OSErr;
    }

    const NO_ERR: OSErr = 0;
    const ERR_AE_EVENT_NOT_HANDLED: OSErr = -1708;
    const CORE_EVENT_CLASS: OSType = 0x6165_7674; // aevt
    const REOPEN_APPLICATION_EVENT: OSType = 0x7261_7070; // rapp

    pub(super) fn install_application_reopen_handler(
        proxy: EventLoopProxy<RuntimeUserEvent>,
    ) -> ApplicationReopenRegistration {
        let refcon = Box::into_raw(Box::new(proxy));
        let status = unsafe {
            AEInstallEventHandler(
                CORE_EVENT_CLASS,
                REOPEN_APPLICATION_EVENT,
                Some(application_reopen_handler),
                refcon as isize,
                0,
            )
        };
        if status == NO_ERR {
            tracing::debug!("radiant generic native vello: installed macOS reopen handler");
            ApplicationReopenRegistration {
                refcon,
                installed: true,
            }
        } else {
            tracing::warn!(
                status,
                "radiant generic native vello: failed to install macOS reopen handler"
            );
            unsafe { drop(Box::from_raw(refcon)) };
            ApplicationReopenRegistration {
                refcon: ptr::null_mut(),
                installed: false,
            }
        }
    }

    pub(super) fn uninstall_application_reopen_handler(
        registration: &mut ApplicationReopenRegistration,
    ) {
        if registration.installed {
            let status = unsafe {
                AERemoveEventHandler(
                    CORE_EVENT_CLASS,
                    REOPEN_APPLICATION_EVENT,
                    Some(application_reopen_handler),
                    0,
                )
            };
            if status != NO_ERR {
                tracing::warn!(
                    status,
                    "radiant generic native vello: failed to remove macOS reopen handler; retaining callback refcon until process exit"
                );
                return;
            }
            registration.installed = false;
        }
        if !registration.refcon.is_null() {
            unsafe { drop(Box::from_raw(registration.refcon)) };
            registration.refcon = ptr::null_mut();
        }
    }

    unsafe extern "C" fn application_reopen_handler(
        _event: *const c_void,
        _reply: *mut c_void,
        refcon: isize,
    ) -> OSErr {
        if refcon == 0 {
            return ERR_AE_EVENT_NOT_HANDLED;
        }
        let proxy = unsafe { &*(refcon as *const EventLoopProxy<RuntimeUserEvent>) };
        if proxy
            .send_event(RuntimeUserEvent::ApplicationReopenRequested)
            .is_err()
        {
            return ERR_AE_EVENT_NOT_HANDLED;
        }
        NO_ERR
    }
}

#[cfg(not(target_os = "macos"))]
mod platform {
    use super::{ApplicationReopenRegistration, EventLoopProxy, RuntimeUserEvent};

    pub(super) fn install_application_reopen_handler(
        _proxy: EventLoopProxy<RuntimeUserEvent>,
    ) -> ApplicationReopenRegistration {
        ApplicationReopenRegistration
    }
}
