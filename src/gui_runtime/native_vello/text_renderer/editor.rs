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
    bytes: Range<usize>,
    glyphs: Vec<GlyphPlacement>,
}

impl NativeEditorClusterPayload {
    pub(in crate::gui_runtime::native_vello) fn source_cluster(&self) -> usize {
        self.source_cluster
    }

    pub(in crate::gui_runtime::native_vello) fn bytes(&self) -> &Range<usize> {
        &self.bytes
    }

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

    for geometry in geometry {
        let bytes = shifted_range(geometry.range, byte_offset)?;
        let bidi_level = pre_l1_bidi_level(shaped.source.as_ref(), presentation, geometry.range)?;
        let advance = (geometry.x_end - geometry.x_start).abs();
        if !advance.is_finite() {
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
            bytes,
            glyphs: shaped
                .glyphs
                .iter()
                .filter(|glyph| glyph.cluster.start == geometry.range.start)
                .map(|glyph| shifted_glyph(*glyph, byte_offset))
                .collect::<Option<Vec<_>>>()?,
        });
    }
    Some(())
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

fn shifted_glyph(mut glyph: GlyphPlacement, byte_offset: usize) -> Option<GlyphPlacement> {
    glyph.cluster = ShapeClusterRange {
        start: Utf8ByteOffset(glyph.cluster.start.0.checked_add(byte_offset)?),
        end: Utf8ByteOffset(glyph.cluster.end.0.checked_add(byte_offset)?),
    };
    Some(glyph)
}

fn pre_l1_bidi_level(
    source: &str,
    presentation: &TextPresentation,
    bytes: ShapeClusterRange,
) -> Option<u8> {
    let base = match presentation.direction {
        Some(crate::application::WritingDirection::Ltr) => Some(Level::ltr()),
        Some(crate::application::WritingDirection::Rtl) => Some(Level::rtl()),
        None => None,
    };
    let bidi = BidiInfo::new(source, base);
    let first = bidi.levels.get(bytes.start.0)?.number();
    bidi.levels
        .get(bytes.start.0..bytes.end.0)?
        .iter()
        .all(|level| level.number() == first)
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
    fn shaped_font_break_evidence_controls_editor_wrap_boundaries() {
        use super::{NativeEditorClusterPayload, append_shaped_clusters, compute_shaped_paragraph};
        use crate::gui_runtime::native_vello::text_renderer::font::NativeFontStack;
        use std::sync::Arc;

        let mut stack = NativeFontStack::from_test_bytes(&[include_bytes!(
            "../../../../tests/fixtures/fonts/primary.ttf"
        )]);
        let shaped =
            compute_shaped_paragraph(&mut stack, Arc::from("A B"), 20.0, &Default::default())
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
        for cluster in &clusters {
            assert_eq!(
                cluster.safe_break_after,
                shaped.safe_to_break_before(super::Utf8ByteOffset(cluster.bytes.end))
            );
        }
    }
}
