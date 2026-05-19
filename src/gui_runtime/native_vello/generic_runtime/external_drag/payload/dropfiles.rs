use std::os::windows::ffi::OsStrExt;
use std::path::PathBuf;

use windows::Win32::Foundation::POINT;
use windows::Win32::UI::Shell::DROPFILES;

pub(super) fn build_dropfiles_payload(paths: &[PathBuf]) -> Vec<u8> {
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
