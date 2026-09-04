use crate::{
    gui::{
        paint::{TextAlign, TextRun},
        types::{Point, Rgba8},
    },
    runtime::PaintText,
    widgets::{TextWrap, WidgetId},
};
use std::{
    hash::{Hash, Hasher},
    ops::Range,
    sync::Arc,
};
use vello::peniko::FontData;

#[derive(Clone, Debug)]
pub(in crate::gui_runtime::native_vello) struct SceneTextRun {
    pub(in crate::gui_runtime::native_vello) text: PaintText,
    pub(in crate::gui_runtime::native_vello) position: Point,
    pub(in crate::gui_runtime::native_vello) font_size: f32,
    pub(in crate::gui_runtime::native_vello) color: Rgba8,
    pub(in crate::gui_runtime::native_vello) max_width: Option<f32>,
    pub(in crate::gui_runtime::native_vello) align: TextAlign,
    pub(in crate::gui_runtime::native_vello) widget_id: WidgetId,
    pub(in crate::gui_runtime::native_vello) wrap: TextWrap,
}

impl From<&TextRun> for SceneTextRun {
    fn from(run: &TextRun) -> Self {
        Self {
            text: run.text.as_str().into(),
            position: run.position,
            font_size: run.font_size,
            color: run.color,
            max_width: run.max_width,
            align: run.align,
            widget_id: 0,
            wrap: TextWrap::None,
        }
    }
}

#[cfg(test)]
#[derive(Clone, Debug)]
pub(in crate::gui_runtime::native_vello) struct GlyphLayout {
    /// Stable index into the append-only native font stack.
    pub(in crate::gui_runtime::native_vello) face_index: usize,
    /// Glyph identifier within the selected face.
    pub(in crate::gui_runtime::native_vello) id: u32,
    /// Logical x-position measured using the selected face's advance.
    pub(in crate::gui_runtime::native_vello) x: f32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::gui_runtime::native_vello) enum CaretAffinity {
    Upstream,
    Downstream,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::gui_runtime::native_vello) struct ScalarBoundary(
    pub(in crate::gui_runtime::native_vello) usize,
);

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(in crate::gui_runtime::native_vello) struct Utf8ByteOffset(
    pub(in crate::gui_runtime::native_vello) usize,
);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::gui_runtime::native_vello) struct GraphemeBoundary(
    pub(in crate::gui_runtime::native_vello) usize,
);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::gui_runtime::native_vello) struct ShapeClusterRange {
    pub(in crate::gui_runtime::native_vello) start: Utf8ByteOffset,
    pub(in crate::gui_runtime::native_vello) end: Utf8ByteOffset,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::gui_runtime::native_vello) enum BidiDirection {
    Ltr,
    Rtl,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::gui_runtime::native_vello) enum LineBreakKind {
    Mandatory,
    Allowed,
}

/// Stable identity for the selected pure-Rust UAX #14 adapter.
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub(in crate::gui_runtime::native_vello) struct LineBreakPolicyId(&'static str);

pub(in crate::gui_runtime::native_vello) const LINE_BREAK_POLICY_ID: LineBreakPolicyId =
    LineBreakPolicyId("uax14:unicode-linebreak@0.1.5:unicode@15.0.0:default-sa-to-al:v1");

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::gui_runtime::native_vello) struct LineBreakRecord {
    pub(in crate::gui_runtime::native_vello) grapheme: GraphemeBoundary,
    pub(in crate::gui_runtime::native_vello) byte: Utf8ByteOffset,
    pub(in crate::gui_runtime::native_vello) kind: LineBreakKind,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(in crate::gui_runtime::native_vello) struct ResolvedFontRun {
    pub(in crate::gui_runtime::native_vello) range: Range<usize>,
    pub(in crate::gui_runtime::native_vello) face_index: Option<usize>,
    pub(in crate::gui_runtime::native_vello) direction: BidiDirection,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(in crate::gui_runtime::native_vello) struct BidiRun {
    pub(in crate::gui_runtime::native_vello) range: Range<usize>,
    pub(in crate::gui_runtime::native_vello) level: u8,
    pub(in crate::gui_runtime::native_vello) direction: BidiDirection,
    pub(in crate::gui_runtime::native_vello) visual_index: usize,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(in crate::gui_runtime::native_vello) struct GlyphPlacement {
    pub(in crate::gui_runtime::native_vello) face_index: usize,
    pub(in crate::gui_runtime::native_vello) glyph_id: u32,
    pub(in crate::gui_runtime::native_vello) cluster: ShapeClusterRange,
    pub(in crate::gui_runtime::native_vello) x: f32,
    pub(in crate::gui_runtime::native_vello) y_offset: f32,
    pub(in crate::gui_runtime::native_vello) x_offset: f32,
    pub(in crate::gui_runtime::native_vello) advance: f32,
    pub(in crate::gui_runtime::native_vello) run_index: usize,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(in crate::gui_runtime::native_vello) struct GraphemeGeometry {
    pub(in crate::gui_runtime::native_vello) range: ShapeClusterRange,
    pub(in crate::gui_runtime::native_vello) grapheme_index: usize,
    pub(in crate::gui_runtime::native_vello) x_start: f32,
    pub(in crate::gui_runtime::native_vello) x_end: f32,
    pub(in crate::gui_runtime::native_vello) direction: BidiDirection,
    pub(in crate::gui_runtime::native_vello) visual_index: usize,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(in crate::gui_runtime::native_vello) struct CaretStopGeometry {
    pub(in crate::gui_runtime::native_vello) byte: Utf8ByteOffset,
    pub(in crate::gui_runtime::native_vello) scalar: ScalarBoundary,
    pub(in crate::gui_runtime::native_vello) grapheme: GraphemeBoundary,
    pub(in crate::gui_runtime::native_vello) upstream_x: Option<f32>,
    pub(in crate::gui_runtime::native_vello) downstream_x: Option<f32>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::gui_runtime::native_vello) enum SnapshotQuality {
    Shaped,
    CompatibilityFallback,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(in crate::gui_runtime::native_vello) struct TextQuality {
    pub(in crate::gui_runtime::native_vello) unsupported_shaping_runs: u64,
    pub(in crate::gui_runtime::native_vello) unsupported_shaping_scalars: u64,
    pub(in crate::gui_runtime::native_vello) fallback_glyphs: u64,
    pub(in crate::gui_runtime::native_vello) missing_glyphs: u64,
}

/// Immutable shaped paragraph data retained independently from width/view projection.
#[derive(Clone, Debug)]
pub(in crate::gui_runtime::native_vello) struct ShapedParagraph {
    pub(in crate::gui_runtime::native_vello) source: Arc<str>,
    pub(in crate::gui_runtime::native_vello) source_identity: u64,
    pub(in crate::gui_runtime::native_vello) revision: u64,
    pub(in crate::gui_runtime::native_vello) font_size_bits: u32,
    pub(in crate::gui_runtime::native_vello) scalar_boundaries: Vec<Utf8ByteOffset>,
    pub(in crate::gui_runtime::native_vello) grapheme_boundaries: Vec<Utf8ByteOffset>,
    pub(in crate::gui_runtime::native_vello) breaks: Vec<LineBreakRecord>,
    pub(in crate::gui_runtime::native_vello) break_policy_id: LineBreakPolicyId,
    pub(in crate::gui_runtime::native_vello) resolved_font_runs: Vec<ResolvedFontRun>,
    pub(in crate::gui_runtime::native_vello) bidi_runs: Vec<BidiRun>,
    pub(in crate::gui_runtime::native_vello) glyphs: Vec<GlyphPlacement>,
    pub(in crate::gui_runtime::native_vello) grapheme_geometry: Vec<GraphemeGeometry>,
    pub(in crate::gui_runtime::native_vello) caret_geometry: Vec<CaretStopGeometry>,
    pub(in crate::gui_runtime::native_vello) logical_to_visual: Vec<usize>,
    pub(in crate::gui_runtime::native_vello) visual_to_logical: Vec<usize>,
    pub(in crate::gui_runtime::native_vello) width: f32,
    pub(in crate::gui_runtime::native_vello) quality: TextQuality,
    pub(in crate::gui_runtime::native_vello) quality_kind: SnapshotQuality,
}

impl ShapedParagraph {
    pub(in crate::gui_runtime::native_vello) fn estimated_bytes(&self) -> usize {
        self.source.len()
            + self.scalar_boundaries.len() * std::mem::size_of::<Utf8ByteOffset>()
            + self.grapheme_boundaries.len() * std::mem::size_of::<Utf8ByteOffset>()
            + self.breaks.len() * std::mem::size_of::<LineBreakRecord>()
            + self.resolved_font_runs.len() * std::mem::size_of::<ResolvedFontRun>()
            + self.bidi_runs.len() * std::mem::size_of::<BidiRun>()
            + self.glyphs.len() * std::mem::size_of::<GlyphPlacement>()
            + self.grapheme_geometry.len() * std::mem::size_of::<GraphemeGeometry>()
            + self.caret_geometry.len() * std::mem::size_of::<CaretStopGeometry>()
            + self.logical_to_visual.len() * std::mem::size_of::<usize>()
            + self.visual_to_logical.len() * std::mem::size_of::<usize>()
    }
}

/// The sole immutable geometry authority shared by paint, editing, and IME.
#[derive(Clone, Debug)]
pub(in crate::gui_runtime::native_vello) struct ParagraphSnapshot {
    pub(in crate::gui_runtime::native_vello) source: Arc<str>,
    pub(in crate::gui_runtime::native_vello) source_identity: u64,
    pub(in crate::gui_runtime::native_vello) revision: u64,
    pub(in crate::gui_runtime::native_vello) font_size_bits: u32,
    pub(in crate::gui_runtime::native_vello) scalar_boundaries: Vec<Utf8ByteOffset>,
    pub(in crate::gui_runtime::native_vello) grapheme_boundaries: Vec<Utf8ByteOffset>,
    pub(in crate::gui_runtime::native_vello) breaks: Vec<LineBreakRecord>,
    pub(in crate::gui_runtime::native_vello) break_policy_id: LineBreakPolicyId,
    pub(in crate::gui_runtime::native_vello) resolved_font_runs: Vec<ResolvedFontRun>,
    pub(in crate::gui_runtime::native_vello) bidi_runs: Vec<BidiRun>,
    pub(in crate::gui_runtime::native_vello) glyphs: Vec<GlyphPlacement>,
    pub(in crate::gui_runtime::native_vello) grapheme_geometry: Vec<GraphemeGeometry>,
    pub(in crate::gui_runtime::native_vello) caret_geometry: Vec<CaretStopGeometry>,
    pub(in crate::gui_runtime::native_vello) logical_to_visual: Vec<usize>,
    pub(in crate::gui_runtime::native_vello) visual_to_logical: Vec<usize>,
    pub(in crate::gui_runtime::native_vello) width: f32,
    pub(in crate::gui_runtime::native_vello) available_width: Option<f32>,
    pub(in crate::gui_runtime::native_vello) alignment_offset: f32,
    pub(in crate::gui_runtime::native_vello) quality: TextQuality,
    pub(in crate::gui_runtime::native_vello) quality_kind: SnapshotQuality,
    pub(in crate::gui_runtime::native_vello) shaped: Arc<ShapedParagraph>,
}

impl ParagraphSnapshot {
    pub(in crate::gui_runtime::native_vello) fn from_shaped(
        shaped: Arc<ShapedParagraph>,
        available_width: Option<f32>,
        align: TextAlign,
    ) -> Arc<Self> {
        let alignment_offset = available_width
            .filter(|width| width.is_finite() && *width > shaped.width)
            .map(|width| {
                let extra = width - shaped.width;
                match align {
                    TextAlign::Left => 0.0,
                    TextAlign::Center => extra * 0.5,
                    TextAlign::Right => extra,
                }
            })
            .unwrap_or(0.0);
        let offset = |x: f32| x + alignment_offset;
        Arc::new(Self {
            source: shaped.source.clone(),
            source_identity: shaped.source_identity,
            revision: shaped.revision,
            font_size_bits: shaped.font_size_bits,
            scalar_boundaries: shaped.scalar_boundaries.clone(),
            grapheme_boundaries: shaped.grapheme_boundaries.clone(),
            breaks: shaped.breaks.clone(),
            break_policy_id: shaped.break_policy_id,
            resolved_font_runs: shaped.resolved_font_runs.clone(),
            bidi_runs: shaped.bidi_runs.clone(),
            glyphs: shaped
                .glyphs
                .iter()
                .map(|glyph| GlyphPlacement {
                    x: offset(glyph.x),
                    ..*glyph
                })
                .collect(),
            grapheme_geometry: shaped
                .grapheme_geometry
                .iter()
                .map(|geometry| GraphemeGeometry {
                    x_start: offset(geometry.x_start),
                    x_end: offset(geometry.x_end),
                    ..*geometry
                })
                .collect(),
            caret_geometry: shaped
                .caret_geometry
                .iter()
                .map(|caret| CaretStopGeometry {
                    upstream_x: caret.upstream_x.map(offset),
                    downstream_x: caret.downstream_x.map(offset),
                    ..*caret
                })
                .collect(),
            logical_to_visual: shaped.logical_to_visual.clone(),
            visual_to_logical: shaped.visual_to_logical.clone(),
            width: shaped.width,
            available_width,
            alignment_offset,
            quality: shaped.quality,
            quality_kind: shaped.quality_kind,
            shaped,
        })
    }

    pub(in crate::gui_runtime::native_vello) fn empty(text: &str) -> Arc<Self> {
        let source: Arc<str> = Arc::from(text);
        let scalar_boundaries = boundaries_for_scalars(text);
        let grapheme_boundaries = boundaries_for_graphemes(text);
        let terminal = Utf8ByteOffset(text.len());
        let scalar = ScalarBoundary(scalar_boundaries.len().saturating_sub(1));
        let shaped = Arc::new(ShapedParagraph {
            source: source.clone(),
            source_identity: source_identity(text),
            revision: source_revision(text, 0, 0),
            font_size_bits: 0,
            scalar_boundaries,
            grapheme_boundaries: grapheme_boundaries.clone(),
            breaks: vec![LineBreakRecord {
                grapheme: GraphemeBoundary(grapheme_boundaries.len().saturating_sub(1)),
                byte: terminal,
                kind: LineBreakKind::Mandatory,
            }],
            break_policy_id: LINE_BREAK_POLICY_ID,
            resolved_font_runs: Vec::new(),
            bidi_runs: Vec::new(),
            glyphs: Vec::new(),
            grapheme_geometry: Vec::new(),
            caret_geometry: vec![CaretStopGeometry {
                byte: terminal,
                scalar,
                grapheme: GraphemeBoundary(grapheme_boundaries.len().saturating_sub(1)),
                upstream_x: Some(0.0),
                downstream_x: Some(0.0),
            }],
            logical_to_visual: Vec::new(),
            visual_to_logical: Vec::new(),
            width: 0.0,
            quality: TextQuality::default(),
            quality_kind: SnapshotQuality::CompatibilityFallback,
        });
        Self::from_shaped(shaped, None, TextAlign::Left)
    }

    pub(in crate::gui_runtime::native_vello) fn matches_source(
        &self,
        text: &str,
        revision: u64,
    ) -> bool {
        self.source_identity == source_identity(text)
            && self.revision == revision
            && self.source.as_ref() == text
    }

    pub(in crate::gui_runtime::native_vello) fn is_usable_for(&self, font_size: f32) -> bool {
        let mappings_are_bijective = self.logical_to_visual.len() == self.grapheme_geometry.len()
            && self.visual_to_logical.len() == self.grapheme_geometry.len()
            && self
                .logical_to_visual
                .iter()
                .enumerate()
                .all(|(logical, visual)| self.visual_to_logical.get(*visual) == Some(&logical));
        let view_is_valid = self
            .available_width
            .is_none_or(|width| width.is_finite() && width >= 0.0);
        let quality_is_known = matches!(
            self.quality_kind,
            SnapshotQuality::Shaped | SnapshotQuality::CompatibilityFallback
        );
        self.font_size_bits == font_size.to_bits()
            && self.break_policy_id == LINE_BREAK_POLICY_ID
            && self.source_identity == source_identity(self.source.as_ref())
            && self.shaped.source.as_ref() == self.source.as_ref()
            && self.shaped.source_identity == self.source_identity
            && self.shaped.revision == self.revision
            && self.shaped.font_size_bits == self.font_size_bits
            && self.shaped.break_policy_id == self.break_policy_id
            && self.shaped.width == self.width
            && self.width.is_finite()
            && self.width >= 0.0
            && self.alignment_offset.is_finite()
            && view_is_valid
            && mappings_are_bijective
            && quality_is_known
    }

    pub(in crate::gui_runtime::native_vello) fn canonical_byte(
        &self,
        byte: usize,
        affinity: CaretAffinity,
    ) -> usize {
        let byte = byte.min(self.source.len());
        match self
            .grapheme_boundaries
            .binary_search_by_key(&byte, |offset| offset.0)
        {
            Ok(index) => self.grapheme_boundaries[index].0,
            Err(insertion) => {
                if insertion == 0 {
                    return self.grapheme_boundaries[0].0;
                }
                if insertion >= self.grapheme_boundaries.len() {
                    return self
                        .grapheme_boundaries
                        .last()
                        .copied()
                        .unwrap_or(Utf8ByteOffset(0))
                        .0;
                }
                match affinity {
                    CaretAffinity::Upstream => self.grapheme_boundaries[insertion - 1].0,
                    CaretAffinity::Downstream => self.grapheme_boundaries[insertion].0,
                }
            }
        }
    }

    pub(in crate::gui_runtime::native_vello) fn caret_x(
        &self,
        byte: usize,
        affinity: CaretAffinity,
    ) -> f32 {
        let canonical = self.canonical_byte(byte, affinity);
        let Some(caret) = self
            .caret_geometry
            .iter()
            .find(|caret| caret.byte.0 == canonical)
        else {
            return self.width + self.alignment_offset;
        };
        match affinity {
            CaretAffinity::Upstream => caret
                .upstream_x
                .or(caret.downstream_x)
                .unwrap_or(self.width + self.alignment_offset),
            CaretAffinity::Downstream => caret
                .downstream_x
                .or(caret.upstream_x)
                .unwrap_or(self.width + self.alignment_offset),
        }
    }

    pub(in crate::gui_runtime::native_vello) fn hit_test(&self, x: f32) -> (usize, CaretAffinity) {
        if self.caret_geometry.is_empty() {
            return (0, CaretAffinity::Downstream);
        }
        let target = if x.is_finite() { x } else { 0.0 };
        let mut best = (f32::INFINITY, 0usize, CaretAffinity::Downstream);
        for caret in &self.caret_geometry {
            for (affinity, candidate) in [
                (CaretAffinity::Upstream, caret.upstream_x),
                (CaretAffinity::Downstream, caret.downstream_x),
            ] {
                let Some(candidate) = candidate.filter(|candidate| candidate.is_finite()) else {
                    continue;
                };
                let distance = (candidate - target).abs();
                let scalar = caret.scalar.0;
                if distance < best.0
                    || (distance == best.0
                        && (scalar < best.1
                            || (scalar == best.1
                                && affinity == CaretAffinity::Downstream
                                && best.2 == CaretAffinity::Upstream)))
                {
                    best = (distance, scalar, affinity);
                }
            }
        }
        (best.1, best.2)
    }

    pub(in crate::gui_runtime::native_vello) fn selection_rects(
        &self,
        start: usize,
        end: usize,
    ) -> Vec<(f32, f32)> {
        let lower = start.min(end);
        let upper = start.max(end);
        let start = self.canonical_byte(lower, CaretAffinity::Upstream);
        let end = self.canonical_byte(upper, CaretAffinity::Downstream);
        if start >= end {
            return Vec::new();
        }
        let mut ranges = self
            .grapheme_geometry
            .iter()
            .filter(|geometry| geometry.range.start.0 < end && geometry.range.end.0 > start)
            .map(|geometry| {
                (
                    geometry.x_start.min(geometry.x_end),
                    geometry.x_start.max(geometry.x_end),
                )
            })
            .filter(|(start, end)| start.is_finite() && end.is_finite() && end > start)
            .collect::<Vec<_>>();
        ranges.sort_by(|left, right| left.0.total_cmp(&right.0));
        let mut merged: Vec<(f32, f32)> = Vec::with_capacity(ranges.len());
        for (start, end) in ranges {
            if let Some(previous) = merged.last_mut()
                && start <= previous.1 + 0.001
            {
                previous.1 = previous.1.max(end);
            } else {
                merged.push((start, end));
            }
        }
        merged
    }

    pub(in crate::gui_runtime::native_vello) fn visible_byte_range(
        &self,
        scroll_x: f32,
        width: f32,
    ) -> (usize, usize) {
        let left = scroll_x.max(0.0);
        let right = left + width.max(0.0);
        let mut start = self.source.len();
        let mut end = 0;
        for geometry in &self.grapheme_geometry {
            let min_x = geometry.x_start.min(geometry.x_end);
            let max_x = geometry.x_start.max(geometry.x_end);
            if max_x >= left && min_x <= right {
                start = start.min(geometry.range.start.0);
                end = end.max(geometry.range.end.0);
            }
        }
        if start == self.source.len() {
            let (scalar, _) = self.hit_test(left);
            let byte = self
                .scalar_boundaries
                .get(scalar)
                .copied()
                .unwrap_or(Utf8ByteOffset(self.source.len()))
                .0;
            return (byte, byte);
        }
        (start, end)
    }

    pub(in crate::gui_runtime::native_vello) fn estimated_local_bytes(&self) -> usize {
        self.scalar_boundaries.len() * std::mem::size_of::<Utf8ByteOffset>()
            + self.grapheme_boundaries.len() * std::mem::size_of::<Utf8ByteOffset>()
            + self.breaks.len() * std::mem::size_of::<LineBreakRecord>()
            + self.resolved_font_runs.len() * std::mem::size_of::<ResolvedFontRun>()
            + self.bidi_runs.len() * std::mem::size_of::<BidiRun>()
            + self.glyphs.len() * std::mem::size_of::<GlyphPlacement>()
            + self.grapheme_geometry.len() * std::mem::size_of::<GraphemeGeometry>()
            + self.caret_geometry.len() * std::mem::size_of::<CaretStopGeometry>()
            + self.logical_to_visual.len() * std::mem::size_of::<usize>()
            + self.visual_to_logical.len() * std::mem::size_of::<usize>()
    }

    pub(in crate::gui_runtime::native_vello) fn estimated_bytes(&self) -> usize {
        self.estimated_local_bytes()
            .saturating_add(self.shaped.estimated_bytes())
    }
}

#[derive(Clone, Debug)]
pub(in crate::gui_runtime::native_vello) struct TextLayout {
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "legacy width-independent layout remains covered by native text tests"
        )
    )]
    pub(in crate::gui_runtime::native_vello) width: f32,
    #[cfg(test)]
    pub(in crate::gui_runtime::native_vello) glyphs: Vec<GlyphLayout>,
    #[cfg(test)]
    pub(in crate::gui_runtime::native_vello) cursor_stops: Vec<TextCursorStop>,
    pub(in crate::gui_runtime::native_vello) unsupported_shaping_runs: u64,
    pub(in crate::gui_runtime::native_vello) unsupported_shaping_scalars: u64,
    pub(in crate::gui_runtime::native_vello) fallback_glyphs: u64,
    pub(in crate::gui_runtime::native_vello) missing_glyphs: u64,
    pub(in crate::gui_runtime::native_vello) snapshot: Arc<ParagraphSnapshot>,
}

impl TextLayout {
    pub(in crate::gui_runtime::native_vello) fn from_snapshot(
        snapshot: Arc<ParagraphSnapshot>,
    ) -> Self {
        #[cfg(test)]
        let glyphs = snapshot
            .glyphs
            .iter()
            .map(|glyph| GlyphLayout {
                face_index: glyph.face_index,
                id: glyph.glyph_id,
                x: glyph.x,
            })
            .collect();
        #[cfg(test)]
        let cursor_stops = snapshot
            .caret_geometry
            .iter()
            .map(|caret| TextCursorStop {
                byte_index: caret.byte.0,
                x: caret
                    .downstream_x
                    .or(caret.upstream_x)
                    .unwrap_or(snapshot.width),
            })
            .collect();
        Self {
            width: snapshot.width,
            #[cfg(test)]
            glyphs,
            #[cfg(test)]
            cursor_stops,
            unsupported_shaping_runs: snapshot.quality.unsupported_shaping_runs,
            unsupported_shaping_scalars: snapshot.quality.unsupported_shaping_scalars,
            fallback_glyphs: snapshot.quality.fallback_glyphs,
            missing_glyphs: snapshot.quality.missing_glyphs,
            snapshot,
        }
    }

    #[cfg(test)]
    pub(in crate::gui_runtime::native_vello) fn empty_for(text: &str) -> Self {
        Self::from_snapshot(ParagraphSnapshot::empty(text))
    }

    pub(in crate::gui_runtime::native_vello) fn snapshot(&self) -> Arc<ParagraphSnapshot> {
        self.snapshot.clone()
    }

    pub(in crate::gui_runtime::native_vello) fn estimated_bytes(&self) -> usize {
        self.estimated_local_bytes()
            .saturating_add(self.snapshot.shaped.estimated_bytes())
    }

    pub(in crate::gui_runtime::native_vello) fn estimated_local_bytes(&self) -> usize {
        let legacy_bytes = {
            #[cfg(test)]
            {
                self.glyphs.len() * std::mem::size_of::<GlyphLayout>()
            }
            #[cfg(not(test))]
            {
                0
            }
        };
        self.snapshot.estimated_local_bytes() + legacy_bytes
    }
}

#[cfg(test)]
#[derive(Clone, Debug, PartialEq)]
pub(in crate::gui_runtime::native_vello) struct TextCursorStop {
    pub(in crate::gui_runtime::native_vello) byte_index: usize,
    pub(in crate::gui_runtime::native_vello) x: f32,
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub(in crate::gui_runtime::native_vello) struct TextLayoutKey {
    pub(in crate::gui_runtime::native_vello) text: Arc<str>,
    pub(in crate::gui_runtime::native_vello) font_size_bits: u32,
    pub(in crate::gui_runtime::native_vello) font_generation: u64,
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub(in crate::gui_runtime::native_vello) struct TextViewKey {
    pub(in crate::gui_runtime::native_vello) shape: TextLayoutKey,
    pub(in crate::gui_runtime::native_vello) width_bits: u32,
    pub(in crate::gui_runtime::native_vello) align: TextAlign,
    pub(in crate::gui_runtime::native_vello) wrap: TextWrap,
    pub(in crate::gui_runtime::native_vello) break_policy_id: LineBreakPolicyId,
}

#[derive(Clone)]
pub(in crate::gui_runtime::native_vello) struct LoadedFont {
    pub(in crate::gui_runtime::native_vello) font: FontData,
}

pub(in crate::gui_runtime::native_vello) fn source_identity(text: &str) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    text.hash(&mut hasher);
    hasher.finish()
}

pub(in crate::gui_runtime::native_vello) fn source_revision(
    text: &str,
    font_size_bits: u32,
    font_generation: u64,
) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    text.hash(&mut hasher);
    font_size_bits.hash(&mut hasher);
    font_generation.hash(&mut hasher);
    hasher.finish()
}

fn boundaries_for_scalars(text: &str) -> Vec<Utf8ByteOffset> {
    let mut boundaries = text
        .char_indices()
        .map(|(byte, _)| Utf8ByteOffset(byte))
        .collect::<Vec<_>>();
    boundaries.push(Utf8ByteOffset(text.len()));
    boundaries
}

fn boundaries_for_graphemes(text: &str) -> Vec<Utf8ByteOffset> {
    use unicode_segmentation::UnicodeSegmentation;

    let mut boundaries = text
        .grapheme_indices(true)
        .map(|(byte, _)| Utf8ByteOffset(byte))
        .collect::<Vec<_>>();
    boundaries.push(Utf8ByteOffset(text.len()));
    boundaries
}
