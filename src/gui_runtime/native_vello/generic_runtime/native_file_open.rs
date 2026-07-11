//! Native file-open event routing.

use super::{GenericNativeVelloRunner, RuntimeUserEvent};
use crate::runtime::{NativeFileOpen, RuntimeBridge};
use std::path::PathBuf;
use winit::event_loop::{ActiveEventLoop, EventLoopProxy};

impl<Bridge, Message> GenericNativeVelloRunner<Bridge, Message>
where
    Bridge: RuntimeBridge<Message>,
{
    pub(super) fn handle_native_file_open(
        &mut self,
        event_loop: &ActiveEventLoop,
        paths: Vec<PathBuf>,
    ) {
        if paths.is_empty() {
            return;
        }
        tracing::info!(
            path_count = paths.len(),
            "radiant generic native vello: native file open"
        );
        let command = self
            .core
            .runtime
            .bridge_mut()
            .native_file_open(NativeFileOpen::new(paths));
        let outcome = self.core.runtime.execute_command(command);
        let routed = self.core.route_command_outcome(outcome);
        self.handle_route_outcome(event_loop, routed);
    }
}

pub(super) fn install_native_file_open_handler(
    proxy: EventLoopProxy<RuntimeUserEvent>,
) -> NativeFileOpenRegistration {
    platform::install_native_file_open_handler(proxy)
}

#[cfg(target_os = "macos")]
pub(super) struct NativeFileOpenRegistration {
    refcon: *mut EventLoopProxy<RuntimeUserEvent>,
    installed: bool,
}

#[cfg(target_os = "macos")]
impl Drop for NativeFileOpenRegistration {
    fn drop(&mut self) {
        platform::uninstall_native_file_open_handler(self);
    }
}

#[cfg(not(target_os = "macos"))]
pub(super) struct NativeFileOpenRegistration;

#[cfg(not(target_os = "macos"))]
impl Drop for NativeFileOpenRegistration {
    fn drop(&mut self) {}
}

#[cfg(target_os = "macos")]
mod platform {
    use super::{NativeFileOpenRegistration, RuntimeUserEvent};
    use std::{ffi::c_void, os::unix::ffi::OsStringExt, path::PathBuf, ptr};
    use winit::event_loop::EventLoopProxy;

    type OSType = u32;
    type OSErr = i16;
    type DescType = OSType;
    type AEKeyword = OSType;
    type Size = isize;
    type Boolean = u8;

    #[repr(C)]
    struct AEDesc {
        _descriptor_type: DescType,
        _data_handle: *mut c_void,
    }

    type AppleEvent = AEDesc;
    type AEEventHandler =
        Option<unsafe extern "C" fn(*const AppleEvent, *mut AppleEvent, isize) -> OSErr>;

    #[link(name = "ApplicationServices", kind = "framework")]
    unsafe extern "C" {
        fn AEInstallEventHandler(
            event_class: OSType,
            event_id: OSType,
            handler: AEEventHandler,
            handler_refcon: isize,
            is_sys_handler: Boolean,
        ) -> OSErr;
        fn AERemoveEventHandler(
            event_class: OSType,
            event_id: OSType,
            handler: AEEventHandler,
            is_sys_handler: Boolean,
        ) -> OSErr;
        fn AEGetParamDesc(
            event: *const AppleEvent,
            keyword: AEKeyword,
            desired_type: DescType,
            result: *mut AEDesc,
        ) -> OSErr;
        fn AECountItems(desc_list: *const AEDesc, count: *mut isize) -> OSErr;
        fn AEGetNthDesc(
            desc_list: *const AEDesc,
            index: isize,
            desired_type: DescType,
            keyword: *mut AEKeyword,
            result: *mut AEDesc,
        ) -> OSErr;
        fn AEGetDescDataSize(desc: *const AEDesc) -> Size;
        fn AEGetDescData(desc: *const AEDesc, data_ptr: *mut c_void, max_size: Size) -> OSErr;
        fn AEDisposeDesc(desc: *mut AEDesc) -> OSErr;
    }

    const NO_ERR: OSErr = 0;
    const ERR_AE_EVENT_NOT_HANDLED: OSErr = -1708;
    const K_CORE_EVENT_CLASS: OSType = 0x6165_7674; // aevt
    const K_AE_OPEN_DOCUMENTS: OSType = 0x6f64_6f63; // odoc
    const KEY_DIRECT_OBJECT: AEKeyword = 0x2d2d_2d2d; // ----
    const TYPE_AE_LIST: DescType = 0x6c69_7374; // list
    const TYPE_FILE_URL: DescType = 0x6675_726c; // furl

    pub(super) fn install_native_file_open_handler(
        proxy: EventLoopProxy<RuntimeUserEvent>,
    ) -> NativeFileOpenRegistration {
        let refcon = Box::into_raw(Box::new(proxy));
        let status = unsafe {
            AEInstallEventHandler(
                K_CORE_EVENT_CLASS,
                K_AE_OPEN_DOCUMENTS,
                Some(open_documents_handler),
                refcon as isize,
                0,
            )
        };
        if status == NO_ERR {
            tracing::debug!("radiant generic native vello: installed macOS file-open handler");
            NativeFileOpenRegistration {
                refcon,
                installed: true,
            }
        } else {
            tracing::warn!(
                status,
                "radiant generic native vello: failed to install macOS file-open handler"
            );
            unsafe {
                drop(Box::from_raw(refcon));
            }
            NativeFileOpenRegistration {
                refcon: ptr::null_mut(),
                installed: false,
            }
        }
    }

    pub(super) fn uninstall_native_file_open_handler(
        registration: &mut NativeFileOpenRegistration,
    ) {
        let mut removal_status = NO_ERR;
        let removed = teardown_native_file_open_handler(
            &mut registration.installed,
            &mut registration.refcon,
            || {
                removal_status = unsafe {
                    AERemoveEventHandler(
                        K_CORE_EVENT_CLASS,
                        K_AE_OPEN_DOCUMENTS,
                        Some(open_documents_handler),
                        0,
                    )
                };
                removal_status == NO_ERR
            },
            |refcon| unsafe {
                drop(Box::from_raw(refcon));
            },
        );
        if !removed {
            tracing::warn!(
                status = removal_status,
                "radiant generic native vello: failed to remove macOS file-open handler; retaining callback refcon until process exit"
            );
        }
    }

    fn teardown_native_file_open_handler<Refcon>(
        installed: &mut bool,
        refcon: &mut *mut Refcon,
        remove_handler: impl FnOnce() -> bool,
        release_refcon: impl FnOnce(*mut Refcon),
    ) -> bool {
        if *installed && !remove_handler() {
            return false;
        }
        *installed = false;
        if !refcon.is_null() {
            release_refcon(*refcon);
            *refcon = ptr::null_mut();
        }
        true
    }

    unsafe extern "C" fn open_documents_handler(
        event: *const AppleEvent,
        _reply: *mut AppleEvent,
        refcon: isize,
    ) -> OSErr {
        if event.is_null() || refcon == 0 {
            return ERR_AE_EVENT_NOT_HANDLED;
        }
        let paths = unsafe { document_paths_from_event(event) };
        if paths.is_empty() {
            return NO_ERR;
        }
        let proxy = unsafe { &*(refcon as *const EventLoopProxy<RuntimeUserEvent>) };
        if proxy
            .send_event(RuntimeUserEvent::OpenFiles(paths))
            .is_err()
        {
            return ERR_AE_EVENT_NOT_HANDLED;
        }
        NO_ERR
    }

    unsafe fn document_paths_from_event(event: *const AppleEvent) -> Vec<PathBuf> {
        let mut list = empty_desc();
        let status = unsafe { AEGetParamDesc(event, KEY_DIRECT_OBJECT, TYPE_AE_LIST, &mut list) };
        if status != NO_ERR {
            return Vec::new();
        }
        let paths = unsafe { paths_from_desc_list(&list) };
        let _ = unsafe { AEDisposeDesc(&mut list) };
        paths
    }

    unsafe fn paths_from_desc_list(list: &AEDesc) -> Vec<PathBuf> {
        let mut count = 0;
        if unsafe { AECountItems(list, &mut count) } != NO_ERR || count <= 0 {
            return Vec::new();
        }
        let mut paths = Vec::new();
        for index in 1..=count {
            if let Some(path) = unsafe { path_from_desc_list_item(list, index) } {
                paths.push(path);
            }
        }
        paths
    }

    unsafe fn path_from_desc_list_item(list: &AEDesc, index: isize) -> Option<PathBuf> {
        let mut keyword = 0;
        let mut item = empty_desc();
        let status = unsafe { AEGetNthDesc(list, index, TYPE_FILE_URL, &mut keyword, &mut item) };
        if status != NO_ERR {
            return None;
        }
        let path = unsafe { file_url_from_desc(&item) }.and_then(|url| file_url_to_path(&url));
        let _ = unsafe { AEDisposeDesc(&mut item) };
        path
    }

    unsafe fn file_url_from_desc(desc: &AEDesc) -> Option<String> {
        let size = unsafe { AEGetDescDataSize(desc) };
        if size <= 0 {
            return None;
        }
        let mut bytes = vec![0_u8; size as usize];
        let status = unsafe { AEGetDescData(desc, bytes.as_mut_ptr().cast::<c_void>(), size) };
        if status != NO_ERR {
            return None;
        }
        String::from_utf8(bytes).ok()
    }

    fn empty_desc() -> AEDesc {
        AEDesc {
            _descriptor_type: 0,
            _data_handle: ptr::null_mut(),
        }
    }

    fn file_url_to_path(url: &str) -> Option<PathBuf> {
        let raw = url.strip_prefix("file://")?;
        let raw = raw.strip_prefix("localhost").unwrap_or(raw);
        if !raw.starts_with('/') {
            return None;
        }
        percent_decode(raw.as_bytes())
            .map(std::ffi::OsString::from_vec)
            .map(PathBuf::from)
    }

    fn percent_decode(input: &[u8]) -> Option<Vec<u8>> {
        let mut decoded = Vec::with_capacity(input.len());
        let mut index = 0;
        while index < input.len() {
            if input[index] != b'%' {
                decoded.push(input[index]);
                index += 1;
                continue;
            }
            let hi = *input.get(index + 1)?;
            let lo = *input.get(index + 2)?;
            decoded.push(hex_value(hi)? << 4 | hex_value(lo)?);
            index += 3;
        }
        Some(decoded)
    }

    fn hex_value(byte: u8) -> Option<u8> {
        match byte {
            b'0'..=b'9' => Some(byte - b'0'),
            b'a'..=b'f' => Some(byte - b'a' + 10),
            b'A'..=b'F' => Some(byte - b'A' + 10),
            _ => None,
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use std::sync::Arc;

        #[test]
        fn failed_handler_removal_keeps_refcon_backing_alive() {
            let backing = Arc::new(());
            let backing_weak = Arc::downgrade(&backing);
            let mut refcon = Box::into_raw(Box::new(backing));
            let mut installed = true;
            let mut releases = 0;

            let removed = teardown_native_file_open_handler(
                &mut installed,
                &mut refcon,
                || false,
                |_| releases += 1,
            );

            assert!(!removed);
            assert!(installed);
            assert!(!refcon.is_null());
            assert!(backing_weak.upgrade().is_some());
            assert_eq!(releases, 0);

            unsafe {
                drop(Box::from_raw(refcon));
            }
        }

        #[test]
        fn successful_and_repeated_teardown_release_refcon_exactly_once() {
            let backing = Arc::new(());
            let backing_weak = Arc::downgrade(&backing);
            let mut refcon = Box::into_raw(Box::new(backing));
            let mut installed = true;
            let mut removals = 0;
            let mut releases = 0;

            assert!(teardown_native_file_open_handler(
                &mut installed,
                &mut refcon,
                || {
                    removals += 1;
                    true
                },
                |refcon| {
                    releases += 1;
                    unsafe {
                        drop(Box::from_raw(refcon));
                    }
                },
            ));
            assert!(!installed);
            assert!(refcon.is_null());
            assert!(backing_weak.upgrade().is_none());

            assert!(teardown_native_file_open_handler(
                &mut installed,
                &mut refcon,
                || {
                    removals += 1;
                    true
                },
                |_| releases += 1,
            ));
            assert_eq!(removals, 1);
            assert_eq!(releases, 1);
        }

        #[test]
        fn file_url_to_path_decodes_percent_escaped_paths() {
            assert_eq!(
                file_url_to_path("file:///Users/test/Drum%20Loop.wav"),
                Some(PathBuf::from("/Users/test/Drum Loop.wav"))
            );
        }

        #[test]
        fn file_url_to_path_accepts_localhost_authority() {
            assert_eq!(
                file_url_to_path("file://localhost/Volumes/Samples/kick.wav"),
                Some(PathBuf::from("/Volumes/Samples/kick.wav"))
            );
        }

        #[test]
        fn percent_decode_rejects_truncated_escape() {
            assert_eq!(percent_decode(b"/tmp/bad%2"), None);
        }
    }
}

#[cfg(not(target_os = "macos"))]
mod platform {
    use super::{NativeFileOpenRegistration, RuntimeUserEvent};
    use winit::event_loop::EventLoopProxy;

    pub(super) fn install_native_file_open_handler(
        _proxy: EventLoopProxy<RuntimeUserEvent>,
    ) -> NativeFileOpenRegistration {
        // Non-macOS runtimes have no native open-document callback, but the
        // shared event enum still owns the route used by the macOS adapter.
        let _open_files_event_constructor: fn(Vec<std::path::PathBuf>) -> RuntimeUserEvent =
            RuntimeUserEvent::OpenFiles;
        NativeFileOpenRegistration
    }
}
