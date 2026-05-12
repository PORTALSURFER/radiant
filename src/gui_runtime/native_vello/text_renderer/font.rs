//! Fallback native-font discovery helpers for the Vello text renderer.

use crate::gui_runtime::NativeTextOptions;
use std::path::PathBuf;
use vello::peniko::{Blob, FontData};

pub(super) fn load_native_font(options: &NativeTextOptions) -> Option<FontData> {
    for embedded in &options.embedded_fonts {
        if let Some(font) = font_data_from_bytes(embedded.bytes(), embedded.index()) {
            return Some(font);
        }
    }

    load_native_font_from_paths(&options.font_paths)
}

fn load_native_font_from_paths(preferred_paths: &[PathBuf]) -> Option<FontData> {
    for path in native_font_candidates(preferred_paths) {
        let Ok(bytes) = std::fs::read(&path) else {
            continue;
        };
        if let Some(font) = font_data_from_bytes(bytes, 0) {
            return Some(font);
        }
    }
    None
}

fn font_data_from_bytes(bytes: impl AsRef<[u8]>, index: u32) -> Option<FontData> {
    let bytes = bytes.as_ref();
    skrifa::FontRef::from_index(bytes, index).ok()?;
    Some(FontData::new(Blob::from(bytes.to_vec()), index))
}

pub(super) fn native_font_candidates(preferred_paths: &[PathBuf]) -> Vec<PathBuf> {
    let mut candidates = preferred_paths.to_vec();
    if let Ok(path) = std::env::var("RADIANT_NATIVE_FONT_PATH") {
        candidates.push(PathBuf::from(path));
    }

    #[cfg(target_os = "windows")]
    {
        if let Ok(windir) = std::env::var("WINDIR") {
            let base = PathBuf::from(windir).join("Fonts");
            // Prefer fixed-pitch UI glyph advances so dense rows stay visually even.
            candidates.push(base.join("consola.ttf"));
            candidates.push(base.join("segoeui.ttf"));
            candidates.push(base.join("arial.ttf"));
        }
    }
    #[cfg(target_os = "macos")]
    {
        // Prefer fixed-pitch fonts for deterministic row text spacing.
        candidates.push(PathBuf::from("/System/Library/Fonts/SFNSMono.ttf"));
        candidates.push(PathBuf::from(
            "/System/Library/Fonts/Supplemental/Menlo.ttc",
        ));
        candidates.push(PathBuf::from("/System/Library/Fonts/SFNS.ttf"));
        candidates.push(PathBuf::from(
            "/System/Library/Fonts/Supplemental/Arial Unicode.ttf",
        ));
        candidates.push(PathBuf::from("/Library/Fonts/Arial.ttf"));
    }
    #[cfg(any(target_os = "linux", target_os = "freebsd"))]
    {
        // Prefer fixed-pitch fonts for deterministic row text spacing.
        candidates.push(PathBuf::from(
            "/usr/share/fonts/truetype/dejavu/DejaVuSansMono.ttf",
        ));
        candidates.push(PathBuf::from("/usr/share/fonts/dejavu/DejaVuSansMono.ttf"));
        candidates.push(PathBuf::from("/usr/share/fonts/TTF/DejaVuSansMono.ttf"));
        candidates.push(PathBuf::from(
            "/usr/share/fonts/truetype/liberation/LiberationMono-Regular.ttf",
        ));
        candidates.push(PathBuf::from(
            "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
        ));
        candidates.push(PathBuf::from("/usr/share/fonts/dejavu/DejaVuSans.ttf"));
        candidates.push(PathBuf::from("/usr/share/fonts/TTF/DejaVuSans.ttf"));
        candidates.push(PathBuf::from(
            "/usr/share/fonts/truetype/liberation/LiberationSans-Regular.ttf",
        ));
    }

    candidates
}

#[cfg(test)]
mod tests {
    use super::{font_data_from_bytes, native_font_candidates};
    use std::path::PathBuf;

    #[test]
    fn preferred_font_paths_are_checked_before_fallbacks() {
        let candidates = native_font_candidates(&[PathBuf::from("host-font.ttf")]);

        assert_eq!(candidates.first(), Some(&PathBuf::from("host-font.ttf")));
    }

    #[test]
    fn invalid_font_bytes_are_rejected_before_renderer_use() {
        assert!(font_data_from_bytes(b"not a font", 0).is_none());
    }
}
