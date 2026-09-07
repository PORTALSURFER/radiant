use crate::gui_runtime::native_vello::generic_runtime::external_drag::platform::text_encoding::encode_unicode_text;
use crate::gui_runtime::native_vello::generic_runtime::external_drag::platform::url_encoding::encode_unicode_url;
use crate::runtime::ExternalDragEffect;
use std::mem::ManuallyDrop;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use windows::Win32::Foundation::{GlobalFree, HGLOBAL};
use windows::Win32::System::Com::{
    DVASPECT_CONTENT, FORMATETC, STGMEDIUM, STGMEDIUM_0, TYMED_HGLOBAL,
};
use windows::Win32::System::DataExchange::RegisterClipboardFormatW;
use windows::Win32::System::Memory::{
    GMEM_MOVEABLE, GMEM_ZEROINIT, GlobalAlloc, GlobalLock, GlobalUnlock,
};
use windows::Win32::System::Ole::{
    CF_HDROP, CF_UNICODETEXT, DROPEFFECT, DROPEFFECT_COPY, DROPEFFECT_LINK, DROPEFFECT_MOVE,
};
use windows::Win32::UI::Shell::CFSTR_INETURLW;
use windows::core::PCWSTR;
use windows::core::w;

#[path = "payload/dropfiles.rs"]
mod dropfiles;

use dropfiles::build_dropfiles_payload;

pub(super) fn build_file_format() -> FORMATETC {
    FORMATETC {
        cfFormat: CF_HDROP.0,
        ptd: std::ptr::null_mut(),
        dwAspect: DVASPECT_CONTENT.0,
        lindex: -1,
        tymed: TYMED_HGLOBAL.0 as u32,
    }
}

pub(super) fn build_text_format() -> FORMATETC {
    FORMATETC {
        cfFormat: CF_UNICODETEXT.0,
        ptd: std::ptr::null_mut(),
        dwAspect: DVASPECT_CONTENT.0,
        lindex: -1,
        tymed: TYMED_HGLOBAL.0 as u32,
    }
}

pub(super) fn build_url_format() -> Result<FORMATETC, String> {
    let format = unsafe { RegisterClipboardFormatW(CFSTR_INETURLW) };
    if format == 0 {
        return Err(String::from(
            "RegisterClipboardFormatW failed for UniformResourceLocatorW",
        ));
    }
    Ok(FORMATETC {
        cfFormat: format as u16,
        ptd: std::ptr::null_mut(),
        dwAspect: DVASPECT_CONTENT.0,
        lindex: -1,
        tymed: TYMED_HGLOBAL.0 as u32,
    })
}

pub(super) fn build_mime_format(name: &str) -> Result<FORMATETC, String> {
    let name = name
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect::<Vec<_>>();
    let format = unsafe { RegisterClipboardFormatW(PCWSTR(name.as_ptr())) };
    if format == 0 {
        return Err(String::from(
            "RegisterClipboardFormatW failed for external drag MIME type",
        ));
    }
    Ok(FORMATETC {
        cfFormat: format as u16,
        ptd: std::ptr::null_mut(),
        dwAspect: DVASPECT_CONTENT.0,
        lindex: -1,
        tymed: TYMED_HGLOBAL.0 as u32,
    })
}

pub(super) fn build_drop_effect_format(format: u16) -> FORMATETC {
    FORMATETC {
        cfFormat: format,
        ptd: std::ptr::null_mut(),
        dwAspect: DVASPECT_CONTENT.0,
        lindex: -1,
        tymed: TYMED_HGLOBAL.0 as u32,
    }
}

pub(super) fn drop_effect_formats() -> Result<(u16, u16), String> {
    static FORMATS: OnceLock<Result<(u16, u16), String>> = OnceLock::new();
    FORMATS
        .get_or_init(|| {
            let preferred = unsafe { RegisterClipboardFormatW(w!("Preferred DropEffect")) };
            let performed = unsafe { RegisterClipboardFormatW(w!("Performed DropEffect")) };
            if preferred == 0 || performed == 0 {
                Err(String::from("RegisterClipboardFormatW failed"))
            } else {
                Ok((preferred as u16, performed as u16))
            }
        })
        .clone()
}

pub(super) fn drop_effect_medium(effect: DROPEFFECT) -> windows::core::Result<STGMEDIUM> {
    let handle = unsafe { GlobalAlloc(GMEM_MOVEABLE | GMEM_ZEROINIT, std::mem::size_of::<u32>()) }
        .map_err(|_| windows::core::Error::from_thread())?;
    let ptr = unsafe { GlobalLock(handle) } as *mut u32;
    if ptr.is_null() {
        free_hglobal(handle);
        return Err(windows::core::Error::from_thread());
    }
    unsafe {
        *ptr = effect.0;
        let _ = GlobalUnlock(handle);
    }
    Ok(STGMEDIUM {
        tymed: TYMED_HGLOBAL.0 as u32,
        u: STGMEDIUM_0 { hGlobal: handle },
        pUnkForRelease: ManuallyDrop::new(None),
    })
}

pub(super) fn create_hglobal_for_paths(paths: &[PathBuf]) -> Result<HGLOBAL, std::io::Error> {
    let payload = build_dropfiles_payload(paths);
    let handle = unsafe { GlobalAlloc(GMEM_MOVEABLE | GMEM_ZEROINIT, payload.len()) }
        .map_err(last_error_from_win32)?;
    let ptr = unsafe { GlobalLock(handle) };
    if ptr.is_null() {
        free_hglobal(handle);
        return Err(std::io::Error::last_os_error());
    }
    unsafe {
        std::ptr::copy_nonoverlapping(payload.as_ptr(), ptr.cast::<u8>(), payload.len());
        let _ = GlobalUnlock(handle);
    }
    Ok(handle)
}

pub(super) fn create_hglobal_for_text(text: &str) -> Result<HGLOBAL, std::io::Error> {
    let payload = encode_unicode_text(text);
    let handle = unsafe { GlobalAlloc(GMEM_MOVEABLE | GMEM_ZEROINIT, payload.len()) }
        .map_err(last_error_from_win32)?;
    let ptr = unsafe { GlobalLock(handle) };
    if ptr.is_null() {
        free_hglobal(handle);
        return Err(std::io::Error::last_os_error());
    }
    unsafe {
        std::ptr::copy_nonoverlapping(payload.as_ptr(), ptr.cast::<u8>(), payload.len());
        let _ = GlobalUnlock(handle);
    }
    Ok(handle)
}

pub(super) fn create_hglobal_for_url(url: &str) -> Result<HGLOBAL, std::io::Error> {
    let payload = encode_unicode_url(url);
    let handle = unsafe { GlobalAlloc(GMEM_MOVEABLE | GMEM_ZEROINIT, payload.len()) }
        .map_err(last_error_from_win32)?;
    let ptr = unsafe { GlobalLock(handle) };
    if ptr.is_null() {
        free_hglobal(handle);
        return Err(std::io::Error::last_os_error());
    }
    unsafe {
        std::ptr::copy_nonoverlapping(payload.as_ptr(), ptr.cast::<u8>(), payload.len());
        let _ = GlobalUnlock(handle);
    }
    Ok(handle)
}

/// Allocate one movable `HGLOBAL` containing exact caller-owned bytes.
///
/// A zero-byte movable block is valid for an empty MIME representation. It
/// must remain unlocked: locking a zero-byte movable allocation is not a data
/// copy and may fail on Windows.
pub(super) fn create_hglobal_for_bytes(bytes: &[u8]) -> Result<HGLOBAL, std::io::Error> {
    let handle = unsafe { GlobalAlloc(GMEM_MOVEABLE | GMEM_ZEROINIT, bytes.len()) }
        .map_err(last_error_from_win32)?;
    if bytes.is_empty() {
        return Ok(handle);
    }
    let ptr = unsafe { GlobalLock(handle) };
    if ptr.is_null() {
        free_hglobal(handle);
        return Err(std::io::Error::last_os_error());
    }
    unsafe {
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), ptr.cast::<u8>(), bytes.len());
        let _ = GlobalUnlock(handle);
    }
    Ok(handle)
}

pub(super) fn external_drag_effect(effect: DROPEFFECT) -> ExternalDragEffect {
    if effect.0 & DROPEFFECT_MOVE.0 != 0 {
        ExternalDragEffect::Move
    } else if effect.0 & DROPEFFECT_COPY.0 != 0 {
        ExternalDragEffect::Copy
    } else if effect.0 & DROPEFFECT_LINK.0 != 0 {
        ExternalDragEffect::Link
    } else {
        ExternalDragEffect::None
    }
}

pub(super) fn normalize_path(path: &Path) -> PathBuf {
    let absolute = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    let text = absolute.as_os_str().to_string_lossy();
    let verbatim_prefix = "\\\\?\\";
    if text.starts_with(verbatim_prefix) {
        PathBuf::from(text.trim_start_matches(verbatim_prefix))
    } else {
        absolute
    }
}

fn last_error_from_win32(err: windows::core::Error) -> std::io::Error {
    std::io::Error::from_raw_os_error(err.code().0)
}

fn free_hglobal(handle: HGLOBAL) {
    unsafe {
        let _ = GlobalFree(Some(handle));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_path_strips_windows_verbatim_prefix() {
        let normalized = normalize_path(Path::new(r"\\?\C:\samples\kick.wav"));
        assert_eq!(normalized, PathBuf::from(r"C:\samples\kick.wav"));
    }

    #[test]
    fn url_format_uses_one_url_hglobal_slot() {
        let format = build_url_format().expect("UniformResourceLocatorW format");

        assert_eq!(format.lindex, -1);
        assert_eq!(format.dwAspect, DVASPECT_CONTENT.0);
        assert_eq!(format.tymed, TYMED_HGLOBAL.0 as u32);
    }

    #[test]
    fn mime_format_uses_one_custom_hglobal_slot() {
        let format = build_mime_format("application/x-radiant").expect("MIME format");

        assert_eq!(format.lindex, -1);
        assert_eq!(format.dwAspect, DVASPECT_CONTENT.0);
        assert_eq!(format.tymed, TYMED_HGLOBAL.0 as u32);
    }

    #[test]
    fn empty_mime_bytes_allocate_without_global_locking() {
        let handle = create_hglobal_for_bytes(&[]).expect("empty MIME HGLOBAL");

        assert!(!handle.0.is_null());
        free_hglobal(handle);
    }
}
