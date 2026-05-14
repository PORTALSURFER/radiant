use crate::runtime::ExternalDragEffect;
use std::mem::ManuallyDrop;
use std::os::windows::ffi::OsStrExt;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use windows::Win32::Foundation::{HGLOBAL, POINT};
use windows::Win32::System::Com::{
    DVASPECT_CONTENT, FORMATETC, STGMEDIUM, STGMEDIUM_0, TYMED_HGLOBAL,
};
use windows::Win32::System::DataExchange::RegisterClipboardFormatW;
use windows::Win32::System::Memory::{
    GMEM_MOVEABLE, GMEM_ZEROINIT, GlobalAlloc, GlobalLock, GlobalUnlock,
};
use windows::Win32::System::Ole::{
    CF_HDROP, DROPEFFECT, DROPEFFECT_COPY, DROPEFFECT_LINK, DROPEFFECT_MOVE,
};
use windows::Win32::UI::Shell::DROPFILES;
use windows::core::w;

pub(super) fn build_file_format() -> FORMATETC {
    FORMATETC {
        cfFormat: CF_HDROP.0,
        ptd: std::ptr::null_mut(),
        dwAspect: DVASPECT_CONTENT.0,
        lindex: -1,
        tymed: TYMED_HGLOBAL.0 as u32,
    }
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
        unsafe {
            let _ = GlobalUnlock(handle);
        }
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
        unsafe {
            let _ = GlobalUnlock(handle);
        }
        return Err(std::io::Error::last_os_error());
    }
    unsafe {
        std::ptr::copy_nonoverlapping(payload.as_ptr(), ptr.cast::<u8>(), payload.len());
        let _ = GlobalUnlock(handle);
    }
    Ok(handle)
}

fn build_dropfiles_payload(paths: &[PathBuf]) -> Vec<u8> {
    let path_bytes = encode_drag_paths(paths);
    let mut payload = Vec::with_capacity(std::mem::size_of::<DROPFILES>() + path_bytes.len());
    payload.extend_from_slice(&dropfiles_header_bytes());
    payload.extend_from_slice(&path_bytes);
    payload
}

fn encode_drag_paths(paths: &[PathBuf]) -> Vec<u8> {
    let mut utf16_paths = Vec::new();
    for path in paths {
        utf16_paths.extend(
            path.as_os_str()
                .encode_wide()
                .chain(std::iter::once(0))
                .flat_map(u16::to_le_bytes),
        );
    }
    utf16_paths.extend_from_slice(&0u16.to_le_bytes());
    utf16_paths
}

fn dropfiles_header_bytes() -> [u8; std::mem::size_of::<DROPFILES>()] {
    let header = DROPFILES {
        pFiles: std::mem::size_of::<DROPFILES>() as u32,
        pt: POINT { x: 0, y: 0 },
        fNC: false.into(),
        fWide: true.into(),
    };
    let mut bytes = [0u8; std::mem::size_of::<DROPFILES>()];
    unsafe {
        std::ptr::copy_nonoverlapping(
            (&header as *const DROPFILES).cast::<u8>(),
            bytes.as_mut_ptr(),
            bytes.len(),
        );
    }
    bytes
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::windows::ffi::OsStringExt;

    fn decode_drag_paths(bytes: &[u8]) -> Vec<String> {
        let utf16 = bytes
            .chunks_exact(std::mem::size_of::<u16>())
            .map(|chunk| u16::from_le_bytes([chunk[0], chunk[1]]))
            .collect::<Vec<_>>();
        let mut paths = Vec::new();
        let mut start = 0usize;
        while start < utf16.len() {
            let Some(end) = utf16[start..].iter().position(|value| *value == 0) else {
                panic!("drag path payload must be null terminated");
            };
            if end == 0 {
                break;
            }
            paths.push(
                std::ffi::OsString::from_wide(&utf16[start..start + end])
                    .to_string_lossy()
                    .into_owned(),
            );
            start += end + 1;
        }
        paths
    }

    #[test]
    fn normalize_path_strips_windows_verbatim_prefix() {
        let normalized = normalize_path(Path::new(r"\\?\C:\samples\kick.wav"));
        assert_eq!(normalized, PathBuf::from(r"C:\samples\kick.wav"));
    }

    #[test]
    fn encode_drag_paths_double_null_terminates_multi_path_payload() {
        let encoded = encode_drag_paths(&[
            PathBuf::from(r"C:\samples\kick.wav"),
            PathBuf::from(r"D:\packs\snare.wav"),
        ]);

        assert!(encoded.ends_with(&[0, 0, 0, 0]));
        assert_eq!(
            decode_drag_paths(&encoded),
            vec![
                String::from(r"C:\samples\kick.wav"),
                String::from(r"D:\packs\snare.wav"),
            ]
        );
    }

    #[test]
    fn build_dropfiles_payload_prepends_wide_dropfiles_header() {
        let payload = build_dropfiles_payload(&[PathBuf::from(r"C:\samples\hat.wav")]);
        let header_len = std::mem::size_of::<DROPFILES>();
        let header = unsafe { std::ptr::read_unaligned(payload.as_ptr().cast::<DROPFILES>()) };

        assert_eq!(header.pFiles as usize, header_len);
        assert!(header.fWide.as_bool());
        assert!(!header.fNC.as_bool());
        assert_eq!(
            decode_drag_paths(&payload[header_len..]),
            vec![String::from(r"C:\samples\hat.wav")]
        );
    }
}
