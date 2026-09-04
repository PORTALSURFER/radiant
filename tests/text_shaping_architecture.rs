//! Minimal compiled interface spike for the selected text-shaping stack.

use rustybuzz::{Face, UnicodeBuffer};
use unicode_bidi::BidiInfo;
use unicode_linebreak::{BreakOpportunity, linebreaks};
use unicode_segmentation::UnicodeSegmentation;

const TEST_FONT: &[u8] = include_bytes!("fixtures/fonts/primary.ttf");
const SECONDARY_FONT: &[u8] = include_bytes!("fixtures/fonts/secondary.ttf");
const COMBINING_TEXT: &str = "Cafe\u{0301}";
const COMBINING_TEXT_WITH_WRAP: &str = "Cafe\u{0301} x";
const ZWJ_EMOJI: &str = "\u{1f469}\u{200d}\u{1f52c}";
const ZWJ_EMOJI_WITH_WRAP: &str = "\u{1f469}\u{200d}\u{1f52c} x";
const MIXED_DIRECTION_TEXT: &str = "שלום world";
const LINE_BREAK_POLICY_ID: &str =
    "uax14:unicode-linebreak@0.1.5:unicode@15.0.0:default-sa-to-al:v1";

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
enum LineBreakKind {
    Mandatory,
    Allowed,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct LineBreakRecord {
    grapheme: GraphemeBoundary,
    byte: Utf8ByteOffset,
    kind: LineBreakKind,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct LineBreakDecision {
    policy_id: &'static str,
    breaks: Vec<LineBreakRecord>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum LineBreakMappingError {
    NotUtf8Boundary(Utf8ByteOffset),
    NotGraphemeBoundary(Utf8ByteOffset),
}

struct LineBreakPolicy;

impl LineBreakPolicy {
    const ID: &'static str = LINE_BREAK_POLICY_ID;

    fn classify(text: &str) -> Result<LineBreakDecision, LineBreakMappingError> {
        Ok(LineBreakDecision {
            policy_id: Self::ID,
            breaks: map_provider_breaks(text, linebreaks(text))?,
        })
    }
}

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

fn grapheme_to_byte(text: &str, grapheme: GraphemeBoundary) -> Utf8ByteOffset {
    Utf8ByteOffset(
        *grapheme_boundary_bytes(text)
            .get(grapheme.0)
            .expect("grapheme boundary must be within the source text"),
    )
}

fn byte_to_grapheme(text: &str, byte: Utf8ByteOffset) -> GraphemeBoundary {
    GraphemeBoundary(
        grapheme_boundary_bytes(text)
            .binary_search(&byte.0)
            .expect("byte offset must be a grapheme boundary"),
    )
}

fn map_provider_breaks(
    text: &str,
    opportunities: impl IntoIterator<Item = (usize, BreakOpportunity)>,
) -> Result<Vec<LineBreakRecord>, LineBreakMappingError> {
    let stored_grapheme_bytes = grapheme_boundary_bytes(text);

    opportunities
        .into_iter()
        .map(|(byte, opportunity)| {
            let byte = Utf8ByteOffset(byte);
            if !text.is_char_boundary(byte.0) {
                return Err(LineBreakMappingError::NotUtf8Boundary(byte));
            }
            let grapheme = stored_grapheme_bytes
                .binary_search(&byte.0)
                .map(GraphemeBoundary)
                .map_err(|_| LineBreakMappingError::NotGraphemeBoundary(byte))?;
            let kind = match opportunity {
                BreakOpportunity::Mandatory => LineBreakKind::Mandatory,
                BreakOpportunity::Allowed => LineBreakKind::Allowed,
            };
            Ok(LineBreakRecord {
                grapheme,
                byte,
                kind,
            })
        })
        .collect()
}

fn assert_grapheme_safe_breaks(text: &str, expected: &[LineBreakRecord]) {
    let decision = LineBreakPolicy::classify(text).expect("fixture provider output must map");
    assert_eq!(decision.breaks, expected);

    let stored_grapheme_bytes = grapheme_boundary_bytes(text);
    for record in decision.breaks {
        assert!(text.is_char_boundary(record.byte.0));
        assert_eq!(
            stored_grapheme_bytes.binary_search(&record.byte.0),
            Ok(record.grapheme.0),
            "provider output must map to a stored grapheme/UTF-8 boundary"
        );
    }
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
fn grapheme_boundaries_round_trip_through_typed_utf8_offsets() {
    for text in [COMBINING_TEXT, ZWJ_EMOJI, MIXED_DIRECTION_TEXT] {
        let grapheme_count = text.graphemes(true).count();
        for ordinal in 0..=grapheme_count {
            let grapheme = GraphemeBoundary(ordinal);
            let byte = grapheme_to_byte(text, grapheme);
            assert_eq!(byte_to_grapheme(text, byte), grapheme);
        }

        assert_eq!(
            grapheme_to_byte(text, GraphemeBoundary(grapheme_count)),
            Utf8ByteOffset(text.len()),
            "the terminal grapheme boundary must map to the terminal UTF-8 offset"
        );
    }
}

#[test]
fn line_break_provider_is_deterministic_and_includes_terminal_mandatory_output() {
    let text = "abc";
    let expected = LineBreakDecision {
        policy_id: "uax14:unicode-linebreak@0.1.5:unicode@15.0.0:default-sa-to-al:v1",
        breaks: vec![LineBreakRecord {
            grapheme: GraphemeBoundary(3),
            byte: Utf8ByteOffset(3),
            kind: LineBreakKind::Mandatory,
        }],
    };

    let first = LineBreakPolicy::classify(text).expect("fixture provider output must map");
    let second = LineBreakPolicy::classify(text).expect("fixture provider output must map");
    assert_eq!(first, expected);
    assert_eq!(second, expected);
    assert_eq!(first, second);
}

#[test]
fn line_break_provider_treats_crlf_as_one_hard_break_and_preserves_terminal_behavior() {
    assert_eq!(
        LineBreakPolicy::classify("a\r\nb")
            .expect("fixture provider output must map")
            .breaks,
        vec![
            LineBreakRecord {
                grapheme: GraphemeBoundary(2),
                byte: Utf8ByteOffset(3),
                kind: LineBreakKind::Mandatory,
            },
            LineBreakRecord {
                grapheme: GraphemeBoundary(3),
                byte: Utf8ByteOffset(4),
                kind: LineBreakKind::Mandatory,
            },
        ]
    );

    assert_eq!(
        LineBreakPolicy::classify("a\r\n")
            .expect("fixture provider output must map")
            .breaks,
        vec![LineBreakRecord {
            grapheme: GraphemeBoundary(2),
            byte: Utf8ByteOffset(3),
            kind: LineBreakKind::Mandatory,
        }],
        "the provider reports trailing CRLF and its terminal opportunity at one offset"
    );
}

#[test]
fn line_break_provider_maps_combining_zwj_and_mixed_direction_text_to_graphemes() {
    assert_grapheme_safe_breaks(
        COMBINING_TEXT_WITH_WRAP,
        &[
            LineBreakRecord {
                grapheme: GraphemeBoundary(5),
                byte: Utf8ByteOffset(7),
                kind: LineBreakKind::Allowed,
            },
            LineBreakRecord {
                grapheme: GraphemeBoundary(6),
                byte: Utf8ByteOffset(8),
                kind: LineBreakKind::Mandatory,
            },
        ],
    );
    assert_grapheme_safe_breaks(
        ZWJ_EMOJI_WITH_WRAP,
        &[
            LineBreakRecord {
                grapheme: GraphemeBoundary(2),
                byte: Utf8ByteOffset(12),
                kind: LineBreakKind::Allowed,
            },
            LineBreakRecord {
                grapheme: GraphemeBoundary(3),
                byte: Utf8ByteOffset(13),
                kind: LineBreakKind::Mandatory,
            },
        ],
    );
    assert_grapheme_safe_breaks(
        MIXED_DIRECTION_TEXT,
        &[
            LineBreakRecord {
                grapheme: GraphemeBoundary(5),
                byte: Utf8ByteOffset(9),
                kind: LineBreakKind::Allowed,
            },
            LineBreakRecord {
                grapheme: GraphemeBoundary(10),
                byte: Utf8ByteOffset(14),
                kind: LineBreakKind::Mandatory,
            },
        ],
    );
}

#[test]
fn line_break_adapter_rejects_unstored_provider_offsets() {
    assert_eq!(
        map_provider_breaks(COMBINING_TEXT, [(4, BreakOpportunity::Allowed)]),
        Err(LineBreakMappingError::NotGraphemeBoundary(Utf8ByteOffset(
            4
        )))
    );
    assert_eq!(
        map_provider_breaks(ZWJ_EMOJI, [(1, BreakOpportunity::Allowed)]),
        Err(LineBreakMappingError::NotUtf8Boundary(Utf8ByteOffset(1)))
    );
}

#[test]
fn representative_text_results_are_deterministic() {
    assert_eq!(
        grapheme_boundaries(COMBINING_TEXT),
        vec![(0, "C"), (1, "a"), (2, "f"), (3, "e\u{0301}")]
    );
    assert_eq!(bidi_levels(COMBINING_TEXT), vec![0; COMBINING_TEXT.len()]);

    assert_eq!(grapheme_boundaries(ZWJ_EMOJI), vec![(0, ZWJ_EMOJI)]);
    assert_eq!(bidi_levels(ZWJ_EMOJI), vec![0; ZWJ_EMOJI.len()]);

    assert_eq!(
        grapheme_boundaries(MIXED_DIRECTION_TEXT),
        vec![
            (0, "ש"),
            (2, "ל"),
            (4, "ו"),
            (6, "ם"),
            (8, " "),
            (9, "w"),
            (10, "o"),
            (11, "r"),
            (12, "l"),
            (13, "d"),
        ]
    );
    assert_eq!(
        bidi_levels(MIXED_DIRECTION_TEXT),
        [vec![1; 9], vec![2; 5]].concat()
    );
}

#[test]
fn grapheme_spike_preserves_combining_and_emoji_clusters() {
    assert_eq!(grapheme_boundaries(COMBINING_TEXT).len(), 4);
    assert_eq!(grapheme_boundaries(ZWJ_EMOJI).len(), 1);
}

#[test]
fn bidi_spike_analyzes_mixed_direction_input() {
    let info = BidiInfo::new(MIXED_DIRECTION_TEXT, None);
    assert_eq!(info.paragraphs.len(), 1);
    assert!(!info.paragraphs[0].range.is_empty());
    assert!(
        bidi_levels(MIXED_DIRECTION_TEXT)
            .iter()
            .any(|level| *level > 0)
    );
}
