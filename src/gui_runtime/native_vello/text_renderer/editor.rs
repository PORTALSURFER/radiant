//! Native shaped input for the renderer-neutral editor paragraph geometry.

use super::{
    BidiDirection, GlyphPlacement, GraphemeGeometry, ShapeClusterRange, Utf8ByteOffset,
    font::NativeFontStack, layout::compute_shaped_paragraph, model::TextPresentation,
};
use crate::gui::text_layout::paragraph::{
    ClusterCaretOffset, MAX_PARAGRAPH_SOURCE_BYTES, ParagraphBaseDirection, ParagraphGeometry,
    ParagraphGeometryInput, ParagraphGeometryKey, ShapedLogicalCluster,
};
use unicode_bidi::{BidiInfo, Level};

use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
    ops::Range,
    sync::Arc,
};

/// Native paint data for one logical cluster in [`NativeEditorParagraph`].
///
/// A glyph is retained only at the logical cluster where its Rustybuzz source
/// cluster starts. That prevents ligatures from being painted twice while the
/// renderer-neutral geometry still exposes every grapheme caret.
#[derive(Clone, Debug)]
pub(in crate::gui_runtime::native_vello) struct NativeEditorClusterPayload {
    source_cluster: usize,
    glyphs: Vec<GlyphPlacement>,
}

impl NativeEditorClusterPayload {
    pub(in crate::gui_runtime::native_vello) fn glyphs(&self) -> &[GlyphPlacement] {
        &self.glyphs
    }
}

/// One editor paragraph shaped by the native font stack.
///
/// The geometry is the sole authority for layout, hit testing, and selection.
/// Native glyph payload is deliberately separate so a later renderer adapter
/// can paint the exact same cluster placements without estimating another
/// layout.
#[derive(Clone, Debug)]
pub(in crate::gui_runtime::native_vello) struct NativeEditorParagraph {
    geometry: Arc<ParagraphGeometry>,
    payloads: Vec<NativeEditorClusterPayload>,
}

impl NativeEditorParagraph {
    pub(super) fn estimated_bytes(&self) -> usize {
        self.geometry.estimated_bytes()
            + self.payloads.capacity() * std::mem::size_of::<NativeEditorClusterPayload>()
            + self
                .payloads
                .iter()
                .map(|payload| payload.glyphs.capacity() * std::mem::size_of::<GlyphPlacement>())
                .sum::<usize>()
    }
    pub(in crate::gui_runtime::native_vello) fn geometry(&self) -> &Arc<ParagraphGeometry> {
        &self.geometry
    }

    pub(in crate::gui_runtime::native_vello) fn payload_for(
        &self,
        source_cluster: usize,
    ) -> Option<&NativeEditorClusterPayload> {
        self.payloads
            .get(source_cluster)
            .filter(|payload| payload.source_cluster == source_cluster)
    }
}

pub(super) fn layout_editor_paragraph(
    font_stack: &mut NativeFontStack,
    presentation: &TextPresentation,
    source: Arc<str>,
    font_size: f32,
    wrap_width: f32,
    line_height: f32,
) -> Option<Arc<NativeEditorParagraph>> {
    if source.len() > MAX_PARAGRAPH_SOURCE_BYTES
        || !font_size.is_finite()
        || font_size <= 0.0
        || !wrap_width.is_finite()
        || wrap_width < 0.0
        || !line_height.is_finite()
        || line_height <= 0.0
    {
        return None;
    }

    let mut clusters = Vec::new();
    let mut payloads = Vec::new();
    for bytes in hard_paragraph_ranges(source.as_ref()) {
        if bytes.is_empty() {
            continue;
        }
        let shaped_source: Arc<str> = Arc::from(&source[bytes.clone()]);
        // Do not substitute compatibility metrics here: editor geometry must
        // either use the exact native shaping result or remain unavailable.
        let shaped =
            compute_shaped_paragraph(font_stack, shaped_source, font_size, presentation).ok()?;
        append_shaped_clusters(
            &shaped,
            bytes.start,
            presentation,
            &mut clusters,
            &mut payloads,
        )?;
    }

    let key = paragraph_key(
        source.as_ref(),
        font_size,
        wrap_width,
        line_height,
        font_stack.generation(),
        presentation,
    );
    let geometry = ParagraphGeometry::build(ParagraphGeometryInput {
        key,
        source,
        clusters,
        base_direction: base_direction(presentation),
        wrap_width,
        line_height,
    })
    .ok()?;
    Some(Arc::new(NativeEditorParagraph {
        geometry: Arc::new(geometry),
        payloads,
    }))
}

fn append_shaped_clusters(
    shaped: &super::ShapedParagraph,
    byte_offset: usize,
    presentation: &TextPresentation,
    clusters: &mut Vec<ShapedLogicalCluster>,
    payloads: &mut Vec<NativeEditorClusterPayload>,
) -> Option<()> {
    let mut geometry = shaped.grapheme_geometry.clone();
    geometry.sort_by_key(|geometry| geometry.range.start);
    if geometry
        .windows(2)
        .any(|pair| pair[0].range.end > pair[1].range.start)
    {
        return None;
    }
    let pre_l1_levels = pre_l1_levels(shaped.source.as_ref(), presentation)?;
    let mut glyphs = shaped.glyphs.clone();
    glyphs.sort_by_key(|glyph| glyph.cluster.start);
    let mut glyph_cursor = 0;

    for geometry in geometry {
        let bytes = shifted_range(geometry.range, byte_offset)?;
        let bidi_level = pre_l1_bidi_level(&pre_l1_levels, geometry.range)?;
        let advance = (geometry.x_end - geometry.x_start).abs();
        if !advance.is_finite() {
            return None;
        }
        if glyphs
            .get(glyph_cursor)
            .is_some_and(|glyph| glyph.cluster.start < geometry.range.start)
        {
            return None;
        }
        let glyph_start = glyph_cursor;
        while glyphs
            .get(glyph_cursor)
            .is_some_and(|glyph| glyph.cluster.start == geometry.range.start)
        {
            glyph_cursor += 1;
        }
        let source_cluster_glyphs = &glyphs[glyph_start..glyph_cursor];
        let glyph_origin = geometry.x_start.min(geometry.x_end);
        if !glyph_origin.is_finite() {
            return None;
        }

        let source_cluster = clusters.len();
        clusters.push(ShapedLogicalCluster {
            bytes: bytes.clone(),
            advance,
            bidi_level,
            safe_break_after: shaped.quality.fallback_glyphs == 0
                && shaped.quality.missing_glyphs == 0
                && shaped.safe_to_break_before(geometry.range.end),
            carets: cluster_carets(geometry, advance),
        });
        payloads.push(NativeEditorClusterPayload {
            source_cluster,
            glyphs: source_cluster_glyphs
                .iter()
                .map(|glyph| shifted_glyph(*glyph, byte_offset, glyph_origin))
                .collect::<Option<Vec<_>>>()?,
        });
    }
    (glyph_cursor == glyphs.len()).then_some(())
}

fn cluster_carets(geometry: GraphemeGeometry, advance: f32) -> Vec<ClusterCaretOffset> {
    let (start, end) = match geometry.direction {
        BidiDirection::Ltr => (0.0, advance),
        BidiDirection::Rtl => (advance, 0.0),
    };
    vec![
        ClusterCaretOffset {
            byte_offset: 0,
            x: start,
        },
        ClusterCaretOffset {
            byte_offset: (geometry.range.end.0 - geometry.range.start.0) as u32,
            x: end,
        },
    ]
}

fn shifted_range(range: ShapeClusterRange, byte_offset: usize) -> Option<Range<usize>> {
    Some(range.start.0.checked_add(byte_offset)?..range.end.0.checked_add(byte_offset)?)
}

fn shifted_glyph(
    mut glyph: GlyphPlacement,
    byte_offset: usize,
    source_cluster_origin: f32,
) -> Option<GlyphPlacement> {
    glyph.cluster = ShapeClusterRange {
        start: Utf8ByteOffset(glyph.cluster.start.0.checked_add(byte_offset)?),
        end: Utf8ByteOffset(glyph.cluster.end.0.checked_add(byte_offset)?),
    };
    glyph.x -= source_cluster_origin;
    glyph.x.is_finite().then_some(glyph)
}

fn pre_l1_levels(source: &str, presentation: &TextPresentation) -> Option<Vec<u8>> {
    let base = match presentation.direction {
        Some(crate::application::WritingDirection::Ltr) => Some(Level::ltr()),
        Some(crate::application::WritingDirection::Rtl) => Some(Level::rtl()),
        None => None,
    };
    let bidi = BidiInfo::new(source, base);
    (bidi.levels.len() == source.len()).then(|| {
        bidi.levels
            .iter()
            .map(|level| level.number())
            .collect::<Vec<_>>()
    })
}

fn pre_l1_bidi_level(levels: &[u8], bytes: ShapeClusterRange) -> Option<u8> {
    let first = *levels.get(bytes.start.0)?;
    levels
        .get(bytes.start.0..bytes.end.0)?
        .iter()
        .all(|level| *level == first)
        .then_some(first)
}

fn base_direction(presentation: &TextPresentation) -> ParagraphBaseDirection {
    match presentation.direction {
        Some(crate::application::WritingDirection::Ltr) => ParagraphBaseDirection::Ltr,
        Some(crate::application::WritingDirection::Rtl) => ParagraphBaseDirection::Rtl,
        None => ParagraphBaseDirection::Auto,
    }
}

fn paragraph_key(
    source: &str,
    font_size: f32,
    wrap_width: f32,
    line_height: f32,
    font_generation: u64,
    presentation: &TextPresentation,
) -> ParagraphGeometryKey {
    let mut hasher = DefaultHasher::new();
    source.hash(&mut hasher);
    font_size.to_bits().hash(&mut hasher);
    wrap_width.to_bits().hash(&mut hasher);
    line_height.to_bits().hash(&mut hasher);
    font_generation.hash(&mut hasher);
    presentation.hash(&mut hasher);
    ParagraphGeometryKey(hasher.finish())
}

fn hard_paragraph_ranges(source: &str) -> Vec<Range<usize>> {
    let mut paragraphs = Vec::new();
    let mut start = 0;
    let mut offset = 0;
    while offset < source.len() {
        if let Some(next) = hard_separator_end(source, offset) {
            paragraphs.push(start..offset);
            start = next;
            offset = next;
        } else {
            offset += source[offset..]
                .chars()
                .next()
                .expect("valid UTF-8")
                .len_utf8();
        }
    }
    paragraphs.push(start..source.len());
    paragraphs
}

// Keep this exact separator set in sync with `gui::text_layout::paragraph`.
// The shared kernel owns the resulting line geometry; this split is solely to
// feed its existing single-hard-paragraph native shaper.
fn hard_separator_end(source: &str, offset: usize) -> Option<usize> {
    let rest = source.get(offset..)?;
    if rest.starts_with("\r\n") {
        Some(offset + 2)
    } else if matches!(
        rest.chars().next(),
        Some('\u{b}' | '\u{c}' | '\r' | '\n' | '\u{85}' | '\u{2028}' | '\u{2029}')
    ) {
        Some(offset + rest.chars().next()?.len_utf8())
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::hard_paragraph_ranges;

    #[test]
    fn native_shaping_uses_all_shared_hard_paragraph_separators() {
        let source = "a\r\nb\u{b}c\u{c}d\re\nf\u{85}g\u{2028}h\u{2029}";
        assert_eq!(
            hard_paragraph_ranges(source),
            vec![
                0..1,
                3..4,
                5..6,
                7..8,
                9..10,
                11..12,
                14..15,
                18..19,
                22..22
            ]
        );
    }

    #[test]
    fn ligature_payload_coordinates_are_relative_without_splitting_membership() {
        use super::{GlyphPlacement, ShapeClusterRange, Utf8ByteOffset, shifted_glyph};

        let cluster = ShapeClusterRange {
            start: Utf8ByteOffset(0),
            end: Utf8ByteOffset(3),
        };
        let glyphs = [
            GlyphPlacement {
                face_index: 0,
                glyph_id: 10,
                cluster,
                x: 24.0,
                y_offset: 0.0,
                x_offset: 0.0,
                advance: 4.0,
                run_index: 0,
            },
            GlyphPlacement {
                face_index: 0,
                glyph_id: 11,
                cluster,
                x: 28.0,
                y_offset: 0.0,
                x_offset: 0.0,
                advance: 3.0,
                run_index: 0,
            },
        ];
        let payload = glyphs
            .into_iter()
            .map(|glyph| shifted_glyph(glyph, 7, 24.0).expect("finite glyph"))
            .collect::<Vec<_>>();

        assert_eq!(payload[0].cluster.start, Utf8ByteOffset(7));
        assert_eq!(payload[0].cluster.end, Utf8ByteOffset(10));
        assert_eq!(payload[1].cluster, payload[0].cluster);
        assert_eq!(payload[0].x, 0.0);
        assert_eq!(payload[1].x, 4.0);
    }

    #[test]
    fn shaped_font_break_evidence_controls_editor_wrap_boundaries() {
        use super::{NativeEditorClusterPayload, append_shaped_clusters, compute_shaped_paragraph};
        use crate::gui_runtime::native_vello::text_renderer::font::NativeFontStack;
        use std::sync::Arc;

        let mut stack = NativeFontStack::from_test_bytes(&[include_bytes!(
            "../../../../tests/fixtures/fonts/primary.ttf"
        )]);
        let shaped =
            compute_shaped_paragraph(&mut stack, Arc::from("A?A"), 20.0, &Default::default())
                .expect("fixture font shapes simple Latin text");
        let mut clusters = Vec::new();
        let mut payloads: Vec<NativeEditorClusterPayload> = Vec::new();
        append_shaped_clusters(
            &shaped,
            0,
            &Default::default(),
            &mut clusters,
            &mut payloads,
        )
        .expect("shaped text has valid editor clusters");

        assert!(clusters.iter().any(|cluster| cluster.safe_break_after));
        assert!(clusters.iter().any(|cluster| !cluster.safe_break_after));
        for cluster in &clusters {
            assert_eq!(
                cluster.safe_break_after,
                shaped.safe_to_break_before(super::Utf8ByteOffset(cluster.bytes.end))
            );
        }
    }

    #[test]
    fn rtl_multigrapheme_ligature_payload_uses_logical_geometry_origin() {
        use super::super::model::{
            BidiDirection, BidiRun, GlyphPlacement, GraphemeBoundary, GraphemeGeometry,
            LineBreakKind, LineBreakRecord, ResolvedFontRun, ShapeClusterRange,
            ShapedBreakBoundary, ShapedParagraph, SnapshotQuality, TextPresentation, TextQuality,
            Utf8ByteOffset,
        };
        use super::{NativeEditorClusterPayload, append_shaped_clusters};
        use crate::{
            application::WritingDirection,
            gui::text_layout::paragraph::{
                ParagraphBaseDirection, ParagraphGeometry, ParagraphGeometryInput,
                ParagraphGeometryKey,
            },
        };
        use std::sync::Arc;

        let source: Arc<str> = Arc::from("אב");
        let shaped = ShapedParagraph {
            source: Arc::clone(&source),
            source_identity: 1,
            revision: 1,
            font_size_bits: 16.0_f32.to_bits(),
            scalar_boundaries: vec![Utf8ByteOffset(0), Utf8ByteOffset(2), Utf8ByteOffset(4)],
            grapheme_boundaries: vec![Utf8ByteOffset(0), Utf8ByteOffset(2), Utf8ByteOffset(4)],
            breaks: vec![LineBreakRecord {
                grapheme: GraphemeBoundary(2),
                byte: Utf8ByteOffset(4),
                kind: LineBreakKind::Mandatory,
            }],
            break_policy_id: super::super::model::LINE_BREAK_POLICY_ID,
            resolved_font_runs: vec![ResolvedFontRun {
                range: 0..4,
                face_index: Some(0),
                direction: BidiDirection::Rtl,
            }],
            bidi_runs: vec![BidiRun {
                range: 0..4,
                level: 1,
                direction: BidiDirection::Rtl,
                visual_index: 0,
            }],
            glyphs: vec![GlyphPlacement {
                face_index: 0,
                glyph_id: 1,
                cluster: ShapeClusterRange {
                    start: Utf8ByteOffset(0),
                    end: Utf8ByteOffset(4),
                },
                x: 20.0,
                y_offset: 0.0,
                x_offset: 0.0,
                advance: 20.0,
                run_index: 0,
            }],
            break_safety: vec![
                ShapedBreakBoundary {
                    byte: Utf8ByteOffset(2),
                    safe_to_break_before: true,
                },
                ShapedBreakBoundary {
                    byte: Utf8ByteOffset(4),
                    safe_to_break_before: false,
                },
            ],
            grapheme_geometry: vec![
                GraphemeGeometry {
                    range: ShapeClusterRange {
                        start: Utf8ByteOffset(0),
                        end: Utf8ByteOffset(2),
                    },
                    grapheme_index: 0,
                    x_start: 20.0,
                    x_end: 10.0,
                    direction: BidiDirection::Rtl,
                    visual_index: 1,
                },
                GraphemeGeometry {
                    range: ShapeClusterRange {
                        start: Utf8ByteOffset(2),
                        end: Utf8ByteOffset(4),
                    },
                    grapheme_index: 1,
                    x_start: 10.0,
                    x_end: 0.0,
                    direction: BidiDirection::Rtl,
                    visual_index: 0,
                },
            ],
            caret_geometry: Vec::new(),
            logical_to_visual: vec![1, 0],
            visual_to_logical: vec![1, 0],
            width: 20.0,
            quality: TextQuality::default(),
            quality_kind: SnapshotQuality::Shaped,
        };
        let presentation = TextPresentation {
            locale: None,
            direction: Some(WritingDirection::Rtl),
        };
        let mut clusters = Vec::new();
        let mut payloads: Vec<NativeEditorClusterPayload> = Vec::new();

        append_shaped_clusters(&shaped, 0, &presentation, &mut clusters, &mut payloads)
            .expect("synthetic RTL ligature has complete geometry");
        assert_eq!(payloads[0].glyphs()[0].x, 10.0);
        assert!(payloads[1].glyphs().is_empty());

        let geometry = ParagraphGeometry::build(ParagraphGeometryInput {
            key: ParagraphGeometryKey(1),
            source,
            clusters,
            base_direction: ParagraphBaseDirection::Rtl,
            wrap_width: f32::MAX,
            line_height: 16.0,
        })
        .expect("RTL clusters remain valid paragraph input");
        assert_eq!(geometry.lines().len(), 1);
        assert_eq!(geometry.width(), 20.0);
    }
}
