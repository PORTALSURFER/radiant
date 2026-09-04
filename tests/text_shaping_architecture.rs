//! Minimal compiled interface spike for the selected text-shaping stack.

use rustybuzz::{Face, UnicodeBuffer};
use unicode_bidi::BidiInfo;
use unicode_segmentation::UnicodeSegmentation;

const TEST_FONT: &[u8] = include_bytes!("fixtures/fonts/primary.ttf");

fn grapheme_boundaries(text: &str) -> Vec<(usize, &str)> {
    text.grapheme_indices(true).collect()
}

fn bidi_levels(text: &str) -> Vec<u8> {
    let info = BidiInfo::new(text, None);
    info.paragraphs
        .iter()
        .flat_map(|paragraph| info.levels[paragraph.range.clone()].iter())
        .map(|level| level.number())
        .collect()
}

fn shaped_glyphs(text: &str) -> Vec<(u32, i32)> {
    let face = Face::from_slice(TEST_FONT, 0).expect("checked-in fixture font has face 0");
    let mut buffer = UnicodeBuffer::new();
    buffer.push_str(text);
    let output = rustybuzz::shape(&face, &[], buffer);
    output
        .glyph_infos()
        .iter()
        .zip(output.glyph_positions())
        .map(|(glyph, position)| (glyph.glyph_id, position.x_advance))
        .collect()
}

#[test]
fn representative_text_results_are_deterministic() {
    for text in ["Café", "👩‍🔬", "שלום world"] {
        assert_eq!(grapheme_boundaries(text), grapheme_boundaries(text));
        assert_eq!(bidi_levels(text), bidi_levels(text));
        assert_eq!(shaped_glyphs(text), shaped_glyphs(text));
    }
}

#[test]
fn grapheme_spike_preserves_combining_and_emoji_clusters() {
    assert_eq!(grapheme_boundaries("Café").len(), 4);
    assert_eq!(grapheme_boundaries("👩‍🔬").len(), 1);
}

#[test]
fn bidi_spike_analyzes_mixed_direction_input() {
    let info = BidiInfo::new("שלום world", None);
    assert_eq!(info.paragraphs.len(), 1);
    assert!(!info.paragraphs[0].range.is_empty());
    assert!(bidi_levels("שלום world").iter().any(|level| *level > 0));
}

#[test]
fn shaping_spike_crosses_representative_boundaries() {
    for text in ["Hello", "Café", "👩‍🔬", "שלום world"] {
        assert!(!shaped_glyphs(text).is_empty(), "{text:?} should shape");
    }
}
