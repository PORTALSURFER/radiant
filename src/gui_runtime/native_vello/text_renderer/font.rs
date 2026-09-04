//! Fallback native-font discovery helpers for the Vello text renderer.

use crate::gui_runtime::{EmbeddedFont, NativeTextOptions};
use skrifa::{GlyphId, MetadataProvider};
use std::{collections::VecDeque, path::PathBuf, sync::Arc};
use vello::peniko::{Blob, FontData};

use super::model::LoadedFont;

/// Ordered, append-only font faces used to resolve text glyphs.
///
/// Valid embedded faces are retained immediately. File and native candidates
/// remain pending until a glyph miss needs another face; once loaded, a face is
/// appended and its index never changes, keeping cached glyph layouts stable.
pub(super) struct NativeFontStack {
    faces: Vec<LoadedFont>,
    pending_candidates: VecDeque<PathBuf>,
    generation: u64,
}

impl NativeFontStack {
    pub(super) fn with_options(options: &NativeTextOptions) -> Self {
        let faces = options
            .embedded_fonts
            .iter()
            .filter_map(font_data_from_embedded)
            .map(|font| LoadedFont { font })
            .collect::<Vec<_>>();
        let pending_candidates = deduped_candidates(native_font_candidates(&options.font_paths));
        let mut stack = Self {
            faces,
            pending_candidates,
            generation: 0,
        };
        if stack.faces.is_empty() {
            stack.load_next_candidate();
        }
        stack
    }

    pub(super) fn is_empty(&self) -> bool {
        self.faces.is_empty()
    }

    pub(super) fn face(&self, index: usize) -> Option<&FontData> {
        self.faces.get(index).map(|loaded| &loaded.font)
    }

    pub(super) fn face_data(&self, index: usize) -> Option<&FontData> {
        self.face(index)
    }

    pub(super) fn generation(&self) -> u64 {
        self.generation
    }

    #[cfg(test)]
    pub(super) fn face_count(&self) -> usize {
        self.faces.len()
    }

    #[cfg(test)]
    pub(super) fn pending_candidate_count(&self) -> usize {
        self.pending_candidates.len()
    }

    /// Resolve a scalar against every currently loaded face, loading pending
    /// candidates in order only when the existing stack has no glyph.
    pub(super) fn resolve_glyph(&mut self, character: char) -> Option<FontGlyph> {
        loop {
            if let Some(glyph) = self.find_loaded_glyph(character) {
                return Some(glyph);
            }
            if !self.load_next_candidate() {
                return None;
            }
        }
    }

    /// Return the first ordered question-mark glyph after all candidates have
    /// had an opportunity to provide the requested scalar.
    pub(super) fn fallback_glyph(&mut self) -> Option<FontGlyph> {
        while self.load_next_candidate() {}
        self.find_loaded_glyph('?')
    }

    /// Resolve the first ordered face that covers every scalar in one
    /// grapheme. A grapheme is never split across fallback faces.
    pub(super) fn resolve_grapheme_face(&mut self, grapheme: &str) -> Option<usize> {
        if grapheme.is_empty() {
            return None;
        }
        loop {
            if let Some(index) = self.faces.iter().enumerate().find_map(|(index, loaded)| {
                face_covers_text(&loaded.font, grapheme).then_some(index)
            }) {
                return Some(index);
            }
            if !self.load_next_candidate() {
                return None;
            }
        }
    }

    #[cfg(test)]
    pub(super) fn from_test_bytes(bytes: &[&'static [u8]]) -> Self {
        Self {
            faces: bytes
                .iter()
                .filter_map(|bytes| font_data_from_bytes(bytes, 0))
                .map(|font| LoadedFont { font })
                .collect(),
            pending_candidates: VecDeque::new(),
            generation: 0,
        }
    }

    #[cfg(test)]
    pub(super) fn append_test_bytes(&mut self, bytes: &'static [u8]) -> Option<usize> {
        let font = font_data_from_bytes(bytes, 0)?;
        let index = self.faces.len();
        self.faces.push(LoadedFont { font });
        self.generation = self.generation.saturating_add(1);
        Some(index)
    }

    pub(super) fn glyph_advance(&self, glyph: FontGlyph, font_size: f32) -> f32 {
        let Some(font) = self.face(glyph.face_index) else {
            return font_size * 0.55;
        };
        let Ok(font_ref) = skrifa::FontRef::from_index(font.data.as_ref(), font.index) else {
            return font_size * 0.55;
        };
        let metrics = font_ref.glyph_metrics(
            skrifa::instance::Size::new(font_size),
            skrifa::instance::LocationRef::default(),
        );
        metrics
            .advance_width(GlyphId::new(glyph.glyph_id))
            .unwrap_or(font_size * 0.55)
            .max(0.0)
    }

    fn find_loaded_glyph(&self, character: char) -> Option<FontGlyph> {
        self.faces
            .iter()
            .enumerate()
            .find_map(|(face_index, loaded)| {
                let font_ref =
                    skrifa::FontRef::from_index(loaded.font.data.as_ref(), loaded.font.index)
                        .ok()?;
                let glyph_id = font_ref.charmap().map(character)?;
                Some(FontGlyph {
                    face_index,
                    glyph_id: glyph_id.to_u32(),
                })
            })
    }

    fn load_next_candidate(&mut self) -> bool {
        while let Some(path) = self.pending_candidates.pop_front() {
            let Ok(bytes) = std::fs::read(path) else {
                continue;
            };
            if let Some(font) = font_data_from_bytes(bytes, 0) {
                self.faces.push(LoadedFont { font });
                self.generation = self.generation.saturating_add(1);
                return true;
            }
        }
        false
    }
}

fn face_covers_text(font: &FontData, text: &str) -> bool {
    let Ok(font_ref) = skrifa::FontRef::from_index(font.data.as_ref(), font.index) else {
        return false;
    };
    text.chars()
        .all(|character| font_ref.charmap().map(character).is_some())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct FontGlyph {
    pub(super) face_index: usize,
    pub(super) glyph_id: u32,
}

fn deduped_candidates(candidates: Vec<PathBuf>) -> VecDeque<PathBuf> {
    let mut deduped = Vec::with_capacity(candidates.len());
    for candidate in candidates {
        if !deduped.iter().any(|existing| existing == &candidate) {
            deduped.push(candidate);
        }
    }
    deduped.into()
}

pub(super) fn font_data_from_bytes(bytes: impl AsRef<[u8]>, index: u32) -> Option<FontData> {
    let bytes = bytes.as_ref();
    skrifa::FontRef::from_index(bytes, index).ok()?;
    Some(FontData::new(Blob::from(bytes.to_vec()), index))
}

fn font_data_from_embedded(font: &EmbeddedFont) -> Option<FontData> {
    skrifa::FontRef::from_index(font.bytes(), font.index()).ok()?;
    Some(FontData::new(
        Blob::new(Arc::new(SharedFontBytes(font.shared_bytes()))),
        font.index(),
    ))
}

struct SharedFontBytes(Arc<[u8]>);

impl AsRef<[u8]> for SharedFontBytes {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
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
    use super::{NativeFontStack, font_data_from_bytes, native_font_candidates};
    use crate::gui_runtime::{EmbeddedFont, NativeTextOptions};
    use std::path::PathBuf;

    fn fixture_path(label: &str) -> PathBuf {
        let directory = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".tmp/fixtures");
        std::fs::create_dir_all(&directory).expect("create disposable fixture directory");
        directory.join(format!(
            "radiant-glyph-fallback-{}-{label}.ttf",
            std::process::id()
        ))
    }

    #[test]
    fn preferred_font_paths_are_checked_before_fallbacks() {
        let candidates = native_font_candidates(&[PathBuf::from("host-font.ttf")]);

        assert_eq!(candidates.first(), Some(&PathBuf::from("host-font.ttf")));
    }

    #[test]
    fn invalid_font_bytes_are_rejected_before_renderer_use() {
        assert!(font_data_from_bytes(b"not a font", 0).is_none());
    }

    #[test]
    fn embedded_faces_are_loaded_in_order_and_candidates_remain_pending() {
        let options = NativeTextOptions {
            embedded_fonts: vec![
                EmbeddedFont::from_static(b"invalid"),
                EmbeddedFont::from_static(include_bytes!(
                    "../../../../tests/fixtures/fonts/primary.ttf"
                )),
            ],
            font_paths: vec![PathBuf::from("first.ttf"), PathBuf::from("first.ttf")],
        };
        let mut stack = NativeFontStack::with_options(&options);

        assert_eq!(stack.face_count(), 1);
        assert_eq!(
            stack.fallback_glyph().map(|glyph| glyph.face_index),
            Some(0)
        );
        assert!(
            stack.pending_candidate_count() <= native_font_candidates(&options.font_paths).len()
        );
    }

    #[test]
    fn test_font_stack_keeps_stable_face_indices_when_appending() {
        let mut stack = NativeFontStack::from_test_bytes(&[
            include_bytes!("../../../../tests/fixtures/fonts/primary.ttf"),
            include_bytes!("../../../../tests/fixtures/fonts/secondary.ttf"),
        ]);

        assert_eq!(stack.face_count(), 2);
        assert_eq!(
            stack.fallback_glyph().map(|glyph| glyph.face_index),
            Some(0)
        );
    }

    #[test]
    fn path_faces_load_lazily_after_embedded_faces_and_keep_deduped_order() {
        let path = fixture_path("secondary");
        std::fs::write(
            &path,
            include_bytes!("../../../../tests/fixtures/fonts/secondary.ttf"),
        )
        .expect("write secondary fixture");
        let options = NativeTextOptions {
            embedded_fonts: vec![EmbeddedFont::from_static(include_bytes!(
                "../../../../tests/fixtures/fonts/primary.ttf"
            ))],
            font_paths: vec![path.clone(), path.clone()],
        };
        let mut stack = NativeFontStack::with_options(&options);

        assert_eq!(stack.face_count(), 1);
        let pending_before = stack.pending_candidate_count();
        assert_eq!(
            stack.resolve_glyph('A').map(|glyph| glyph.face_index),
            Some(0)
        );
        assert_eq!(stack.face_count(), 1);
        assert_eq!(stack.pending_candidate_count(), pending_before);
        assert_eq!(
            stack.resolve_glyph('Ω').map(|glyph| glyph.face_index),
            Some(1)
        );
        assert_eq!(stack.face_count(), 2);
        assert!(stack.pending_candidate_count() < pending_before);

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn invalid_candidates_are_skipped_before_the_first_valid_path_face() {
        let invalid_path = fixture_path("invalid");
        let valid_path = fixture_path("valid");
        std::fs::write(&invalid_path, b"not a font").expect("write invalid fixture");
        std::fs::write(
            &valid_path,
            include_bytes!("../../../../tests/fixtures/fonts/secondary.ttf"),
        )
        .expect("write secondary fixture");
        let options = NativeTextOptions {
            embedded_fonts: Vec::new(),
            font_paths: vec![invalid_path.clone(), valid_path.clone()],
        };
        let mut stack = NativeFontStack::with_options(&options);

        assert_eq!(stack.face_count(), 1);
        assert_eq!(
            stack.resolve_glyph('Ω').map(|glyph| glyph.face_index),
            Some(0)
        );

        let _ = std::fs::remove_file(invalid_path);
        let _ = std::fs::remove_file(valid_path);
    }
}
