//! Minimal compiled interface spike for the selected text-shaping stack.

use rustybuzz::{Face, UnicodeBuffer};
use unicode_bidi::BidiInfo;
use unicode_segmentation::UnicodeSegmentation;

const TEST_FONT: &[u8] = include_bytes!("fixtures/fonts/primary.ttf");
const SECONDARY_FONT: &[u8] = include_bytes!("fixtures/fonts/secondary.ttf");
const COMBINING_TEXT: &str = "Cafe\u{0301}";
const ZWJ_EMOJI: &str = "\u{1f469}\u{200d}\u{1f52c}";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct ShapedGlyph {
    glyph_id: u32,
    cluster: u32,
    x_advance: i32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Affinity {
    Upstream,
    Downstream,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct ScalarBoundary(usize);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Utf8ByteOffset(usize);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct GraphemeBoundary(usize);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct CanonicalCaret {
    scalar: ScalarBoundary,
    byte: Utf8ByteOffset,
    grapheme: GraphemeBoundary,
    affinity: Affinity,
}

#[derive(Debug, Eq, PartialEq)]
struct ReplacementGlyph {
    glyph_id: u32,
    cluster_range: (usize, usize),
    x_advance: i32,
}

fn glyph(glyph_id: u32, cluster: u32, x_advance: i32) -> ShapedGlyph {
    ShapedGlyph {
        glyph_id,
        cluster,
        x_advance,
    }
}

fn grapheme_boundaries(text: &str) -> Vec<(usize, &str)> {
    text.grapheme_indices(true).collect()
}

fn grapheme_boundary_bytes(text: &str) -> Vec<usize> {
    let mut boundaries = text
        .grapheme_indices(true)
        .map(|(byte, _)| byte)
        .collect::<Vec<_>>();
    boundaries.push(text.len());
    boundaries
}

fn scalar_byte_boundaries(text: &str) -> Vec<usize> {
    let mut boundaries = text
        .char_indices()
        .map(|(byte, _)| byte)
        .collect::<Vec<_>>();
    boundaries.push(text.len());
    boundaries
}

fn scalar_to_byte(text: &str, scalar: ScalarBoundary) -> Utf8ByteOffset {
    Utf8ByteOffset(
        *scalar_byte_boundaries(text)
            .get(scalar.0)
            .expect("scalar boundary must be within the source text"),
    )
}

fn byte_to_scalar(text: &str, byte: Utf8ByteOffset) -> ScalarBoundary {
    ScalarBoundary(
        scalar_byte_boundaries(text)
            .binary_search(&byte.0)
            .expect("byte offset must be a UTF-8 scalar boundary"),
    )
}

fn canonicalize_scalar_boundary(
    text: &str,
    scalar: ScalarBoundary,
    affinity: Affinity,
) -> CanonicalCaret {
    let byte = scalar_to_byte(text, scalar);
    let boundaries = grapheme_boundary_bytes(text);
    let (grapheme, canonical_byte) = match boundaries.binary_search(&byte.0) {
        Ok(grapheme) => (grapheme, byte.0),
        Err(insertion) => {
            assert!(
                insertion > 0 && insertion < boundaries.len(),
                "a valid scalar boundary must be inside or at the edge of a grapheme"
            );
            let grapheme = match affinity {
                Affinity::Upstream => insertion - 1,
                Affinity::Downstream => insertion,
            };
            (grapheme, boundaries[grapheme])
        }
    };

    let byte = Utf8ByteOffset(canonical_byte);
    CanonicalCaret {
        scalar: byte_to_scalar(text, byte),
        byte,
        grapheme: GraphemeBoundary(grapheme),
        affinity,
    }
}

fn normalize_selection(
    text: &str,
    first: ScalarBoundary,
    second: ScalarBoundary,
) -> std::ops::Range<usize> {
    let (lower, upper) = if first.0 <= second.0 {
        (first, second)
    } else {
        (second, first)
    };
    let start = canonicalize_scalar_boundary(text, lower, Affinity::Upstream)
        .byte
        .0;
    let end = canonicalize_scalar_boundary(text, upper, Affinity::Downstream)
        .byte
        .0;
    start..end
}

fn bidi_levels(text: &str) -> Vec<u8> {
    let info = BidiInfo::new(text, None);
    info.paragraphs
        .iter()
        .flat_map(|paragraph| info.levels[paragraph.range.clone()].iter())
        .map(|level| level.number())
        .collect()
}

fn shaped_glyphs(font: &[u8], text: &str) -> Vec<ShapedGlyph> {
    let face = Face::from_slice(font, 0).expect("checked-in fixture font has face 0");
    let mut buffer = UnicodeBuffer::new();
    buffer.push_str(text);
    let output = rustybuzz::shape(&face, &[], buffer);
    output
        .glyph_infos()
        .iter()
        .zip(output.glyph_positions())
        .map(|(info, position)| glyph(info.glyph_id, info.cluster, position.x_advance))
        .collect()
}

fn face_covers_grapheme(font: &[u8], grapheme: &str) -> bool {
    let face = Face::from_slice(font, 0).expect("checked-in fixture font has face 0");
    grapheme
        .chars()
        .all(|character| face.glyph_index(character).is_some())
}

fn compatibility_replacement_glyphs(text: &str, ordered_faces: &[&[u8]]) -> Vec<ReplacementGlyph> {
    let replacement = ordered_faces
        .iter()
        .find_map(|font| {
            let face = Face::from_slice(font, 0).expect("checked-in fixture font has face 0");
            face.glyph_index('?')?;
            shaped_glyphs(font, "?").into_iter().next()
        })
        .expect("the ordered fixture faces provide a deterministic replacement glyph");

    text.grapheme_indices(true)
        .map(|(start, grapheme)| {
            assert!(
                ordered_faces
                    .iter()
                    .all(|font| !face_covers_grapheme(font, grapheme)),
                "this fixture case must exercise missing-glyph fallback"
            );
            ReplacementGlyph {
                glyph_id: replacement.glyph_id,
                cluster_range: (start, start + grapheme.len()),
                x_advance: replacement.x_advance,
            }
        })
        .collect()
}

#[test]
fn covered_fixture_glyphs_have_stable_ids_clusters_and_advances() {
    assert_eq!(
        shaped_glyphs(TEST_FONT, "A"),
        vec![glyph(1, 0, 580)],
        "primary.ttf's covered Latin glyph must remain a stable fixture"
    );
    assert_eq!(
        shaped_glyphs(SECONDARY_FONT, "Ω"),
        vec![glyph(1, 0, 900)],
        "secondary.ttf's covered Greek glyph must remain a stable fallback fixture"
    );
}

#[test]
fn missing_clusters_have_explicit_notdef_and_compatibility_replacement_output() {
    assert_eq!(
        shaped_glyphs(TEST_FONT, COMBINING_TEXT),
        vec![
            glyph(0, 0, 580),
            glyph(0, 1, 580),
            glyph(0, 2, 580),
            glyph(0, 3, 580),
            glyph(0, 3, 580),
        ],
        "the raw shaper result must expose the uncovered combining sequence as .notdef"
    );
    assert_eq!(
        shaped_glyphs(TEST_FONT, ZWJ_EMOJI),
        vec![glyph(0, 0, 580), glyph(0, 0, 580)],
        "the raw shaper result must expose the uncovered ZWJ sequence as .notdef"
    );

    let ordered_faces = [TEST_FONT, SECONDARY_FONT];
    assert_eq!(
        compatibility_replacement_glyphs(COMBINING_TEXT, &ordered_faces),
        vec![
            ReplacementGlyph {
                glyph_id: 2,
                cluster_range: (0, 1),
                x_advance: 580,
            },
            ReplacementGlyph {
                glyph_id: 2,
                cluster_range: (1, 2),
                x_advance: 580,
            },
            ReplacementGlyph {
                glyph_id: 2,
                cluster_range: (2, 3),
                x_advance: 580,
            },
            ReplacementGlyph {
                glyph_id: 2,
                cluster_range: (3, 6),
                x_advance: 580,
            },
        ],
        "missing scalars must use one stable replacement per missing grapheme range"
    );
    assert_eq!(
        compatibility_replacement_glyphs(ZWJ_EMOJI, &ordered_faces),
        vec![ReplacementGlyph {
            glyph_id: 2,
            cluster_range: (0, 11),
            x_advance: 580,
        }],
        "the whole ZWJ emoji grapheme must use one stable replacement cluster"
    );
    assert_eq!(
        compatibility_replacement_glyphs("שלום Ж", &ordered_faces),
        vec![
            ReplacementGlyph {
                glyph_id: 2,
                cluster_range: (0, 2),
                x_advance: 580,
            },
            ReplacementGlyph {
                glyph_id: 2,
                cluster_range: (2, 4),
                x_advance: 580,
            },
            ReplacementGlyph {
                glyph_id: 2,
                cluster_range: (4, 6),
                x_advance: 580,
            },
            ReplacementGlyph {
                glyph_id: 2,
                cluster_range: (6, 8),
                x_advance: 580,
            },
            ReplacementGlyph {
                glyph_id: 2,
                cluster_range: (8, 9),
                x_advance: 580,
            },
            ReplacementGlyph {
                glyph_id: 2,
                cluster_range: (9, 11),
                x_advance: 580,
            },
        ],
        "mixed-direction missing graphemes must retain exact UTF-8 ranges and stable fallback metrics"
    );
}

#[test]
fn scalar_compatibility_boundaries_round_trip_and_use_explicit_affinity() {
    for text in [COMBINING_TEXT, ZWJ_EMOJI] {
        for scalar in 0..=text.chars().count() {
            let scalar = ScalarBoundary(scalar);
            let byte = scalar_to_byte(text, scalar);
            assert_eq!(byte_to_scalar(text, byte), scalar);
        }
        for byte in grapheme_boundary_bytes(text) {
            let byte = Utf8ByteOffset(byte);
            assert_eq!(scalar_to_byte(text, byte_to_scalar(text, byte)), byte);
        }
    }

    assert_eq!(
        canonicalize_scalar_boundary(COMBINING_TEXT, ScalarBoundary(4), Affinity::Upstream),
        CanonicalCaret {
            scalar: ScalarBoundary(3),
            byte: Utf8ByteOffset(3),
            grapheme: GraphemeBoundary(3),
            affinity: Affinity::Upstream,
        }
    );
    assert_eq!(
        canonicalize_scalar_boundary(COMBINING_TEXT, ScalarBoundary(4), Affinity::Downstream),
        CanonicalCaret {
            scalar: ScalarBoundary(5),
            byte: Utf8ByteOffset(6),
            grapheme: GraphemeBoundary(4),
            affinity: Affinity::Downstream,
        }
    );

    for scalar in [ScalarBoundary(1), ScalarBoundary(2)] {
        assert_eq!(
            canonicalize_scalar_boundary(ZWJ_EMOJI, scalar, Affinity::Upstream),
            CanonicalCaret {
                scalar: ScalarBoundary(0),
                byte: Utf8ByteOffset(0),
                grapheme: GraphemeBoundary(0),
                affinity: Affinity::Upstream,
            }
        );
        assert_eq!(
            canonicalize_scalar_boundary(ZWJ_EMOJI, scalar, Affinity::Downstream),
            CanonicalCaret {
                scalar: ScalarBoundary(3),
                byte: Utf8ByteOffset(11),
                grapheme: GraphemeBoundary(1),
                affinity: Affinity::Downstream,
            }
        );
    }

    assert_eq!(
        normalize_selection(COMBINING_TEXT, ScalarBoundary(4), ScalarBoundary(5)),
        3..6
    );
    assert_eq!(
        normalize_selection(COMBINING_TEXT, ScalarBoundary(5), ScalarBoundary(4)),
        3..6
    );
    assert_eq!(
        normalize_selection(ZWJ_EMOJI, ScalarBoundary(1), ScalarBoundary(2)),
        0..11
    );
}

#[test]
fn representative_text_results_are_deterministic() {
    for text in [COMBINING_TEXT, ZWJ_EMOJI, "שלום world"] {
        assert_eq!(grapheme_boundaries(text), grapheme_boundaries(text));
        assert_eq!(bidi_levels(text), bidi_levels(text));
        assert_eq!(
            shaped_glyphs(TEST_FONT, text),
            shaped_glyphs(TEST_FONT, text)
        );
    }
}

#[test]
fn grapheme_spike_preserves_combining_and_emoji_clusters() {
    assert_eq!(grapheme_boundaries(COMBINING_TEXT).len(), 4);
    assert_eq!(grapheme_boundaries(ZWJ_EMOJI).len(), 1);
}

#[test]
fn bidi_spike_analyzes_mixed_direction_input() {
    let info = BidiInfo::new("שלום world", None);
    assert_eq!(info.paragraphs.len(), 1);
    assert!(!info.paragraphs[0].range.is_empty());
    assert!(bidi_levels("שלום world").iter().any(|level| *level > 0));
}
