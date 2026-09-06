//! Renderer-neutral, bounded paragraph geometry for shaped editor text.
//!
//! The native text provider owns shaping.  This module only validates its immutable
//! cluster snapshot, applies UAX #14 wrapping, and derives the one geometry model
//! used by painting, hit testing, and selection.

use crate::gui::types::{Point, Rect};
use std::{collections::BTreeSet, ops::Range, sync::Arc};
use unicode_bidi::BidiInfo;
use unicode_linebreak::{linebreaks, BreakOpportunity};
use unicode_segmentation::UnicodeSegmentation;

pub const MAX_PARAGRAPH_SOURCE_BYTES: usize = 1024 * 1024;
pub const MAX_PARAGRAPH_CLUSTERS: usize = 65_536;
pub const MAX_PARAGRAPH_LINES: usize = 65_536;

/// Stable, provider-fenced identity for a shaped paragraph input.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub struct ParagraphGeometryKey(pub u64);

/// A caret position at a byte boundary. Affinity distinguishes both sides of a wrap.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub struct ParagraphCaret {
    pub byte: usize,
    pub affinity: CaretAffinity,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum CaretAffinity {
    Upstream,
    Downstream,
}

/// A provider-supplied caret position inside one logical shaped cluster.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ClusterCaretOffset {
    /// Offset from the cluster's UTF-8 start, always a grapheme boundary.
    pub byte_offset: u32,
    /// Horizontal offset from the cluster's visual left edge.
    pub x: f32,
}

/// One logical shaping cluster. The provider must not make these overlap.
#[derive(Clone, Debug, PartialEq)]
pub struct ShapedLogicalCluster {
    pub bytes: Range<usize>,
    pub advance: f32,
    /// Resolved shaping level retained for the renderer. Paragraph reordering applies L1.
    pub bidi_level: u8,
    /// The provider has verified that its shaped boundary can be broken after this cluster.
    pub safe_break_after: bool,
    /// A complete set of grapheme caret boundaries, including zero and cluster length.
    pub carets: Vec<ClusterCaretOffset>,
}

#[derive(Clone, Debug)]
pub struct ParagraphGeometryInput {
    pub key: ParagraphGeometryKey,
    pub source: Arc<str>,
    pub clusters: Vec<ShapedLogicalCluster>,
    pub wrap_width: f32,
    pub line_height: f32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ParagraphGeometryError {
    SourceTooLarge,
    TooManyClusters,
    InvalidMetrics,
    InvalidCluster,
    IncompleteCoverage,
    TooManyLines,
    InvalidByteRange,
    InvalidCaret,
    BidiBoundary,
}

/// Logical line metadata. `bytes` excludes its terminating hard break.
#[derive(Clone, Debug, PartialEq)]
pub struct ParagraphVisualLine {
    pub bytes: Range<usize>,
    pub y: f32,
    pub width: f32,
    pub cluster_range: Range<usize>,
}

/// A visual placement that the renderer consumes directly; no reshaping is required.
#[derive(Clone, Debug, PartialEq)]
pub struct ParagraphClusterPlacement {
    pub source_cluster: usize,
    pub bytes: Range<usize>,
    pub rect: Rect,
    pub bidi_level: u8,
}

#[derive(Clone, Debug)]
pub struct ParagraphGeometry {
    key: ParagraphGeometryKey,
    source: Arc<str>,
    line_height: f32,
    width: f32,
    lines: Vec<ParagraphVisualLine>,
    placements: Vec<Vec<ParagraphClusterPlacement>>,
    carets: Vec<Vec<(ParagraphCaret, Point)>>,
    cluster_carets: Vec<Vec<ClusterCaretOffset>>,
}

impl ParagraphGeometry {
    pub fn build(input: ParagraphGeometryInput) -> Result<Self, ParagraphGeometryError> {
        validate_input(&input)?;
        let hard_lines = hard_lines(input.source.as_ref());
        if hard_lines.len() > MAX_PARAGRAPH_LINES {
            return Err(ParagraphGeometryError::TooManyLines);
        }
        let soft_breaks = safe_breaks(input.source.as_ref());
        let mut logical_lines = Vec::new();
        for range in hard_lines {
            let first = input
                .clusters
                .partition_point(|cluster| cluster.bytes.end <= range.start);
            let end = input
                .clusters
                .partition_point(|cluster| cluster.bytes.start < range.end);
            let indices = first..end;
            logical_lines.extend(wrap_line(
                range,
                indices,
                &input.clusters,
                &soft_breaks,
                input.wrap_width,
            )?);
            if logical_lines.len() > MAX_PARAGRAPH_LINES {
                return Err(ParagraphGeometryError::TooManyLines);
            }
        }

        let mut width: f32 = 0.0;
        let mut lines = Vec::with_capacity(logical_lines.len());
        let mut placements = Vec::with_capacity(logical_lines.len());
        let mut carets = Vec::with_capacity(logical_lines.len());
        for (line_index, (bytes, cluster_range)) in logical_lines.into_iter().enumerate() {
            let y = line_index as f32 * input.line_height;
            let (line_placements, line_carets, line_width) = visual_line(
                input.source.as_ref(),
                bytes.clone(),
                cluster_range.clone(),
                &input.clusters,
                y,
                input.line_height,
            )?;
            width = width.max(line_width);
            lines.push(ParagraphVisualLine {
                bytes,
                y,
                width: line_width,
                cluster_range,
            });
            placements.push(line_placements);
            carets.push(line_carets);
        }
        Ok(Self {
            key: input.key,
            source: input.source,
            line_height: input.line_height,
            width,
            lines,
            placements,
            carets,
            cluster_carets: input
                .clusters
                .iter()
                .map(|cluster| cluster.carets.clone())
                .collect(),
        })
    }

    pub fn key(&self) -> ParagraphGeometryKey {
        self.key
    }
    pub fn source(&self) -> &str {
        self.source.as_ref()
    }
    pub fn lines(&self) -> &[ParagraphVisualLine] {
        &self.lines
    }
    pub fn placements(&self, line: usize) -> Option<&[ParagraphClusterPlacement]> {
        self.placements.get(line).map(Vec::as_slice)
    }
    pub fn width(&self) -> f32 {
        self.width
    }
    pub fn height(&self) -> f32 {
        self.lines.len() as f32 * self.line_height
    }

    pub fn caret(&self, caret: ParagraphCaret) -> Option<Point> {
        self.carets
            .iter()
            .flatten()
            .find_map(|(candidate, point)| (*candidate == caret).then_some(*point))
            .or_else(|| {
                self.carets
                    .iter()
                    .flatten()
                    .find_map(|(candidate, point)| (candidate.byte == caret.byte).then_some(*point))
            })
    }

    pub fn hit_test(&self, point: Point) -> ParagraphCaret {
        let line = if self.lines.is_empty() {
            return ParagraphCaret {
                byte: 0,
                affinity: CaretAffinity::Downstream,
            };
        } else {
            (point.y / self.line_height)
                .floor()
                .clamp(0.0, (self.lines.len() - 1) as f32) as usize
        };
        self.carets[line]
            .iter()
            .min_by(|a, b| {
                (a.1.x - point.x)
                    .abs()
                    .partial_cmp(&(b.1.x - point.x).abs())
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|entry| entry.0)
            .unwrap_or(ParagraphCaret {
                byte: self.lines[line].bytes.start,
                affinity: CaretAffinity::Downstream,
            })
    }

    pub fn selection_rects(&self, selection: Range<usize>) -> Vec<Rect> {
        if selection.start >= selection.end {
            return Vec::new();
        }
        self.placements
            .iter()
            .flatten()
            .filter_map(|placement| self.selection_rect_for_placement(selection.clone(), placement))
            .collect()
    }

    fn selection_rect_for_placement(
        &self,
        selection: Range<usize>,
        placement: &ParagraphClusterPlacement,
    ) -> Option<Rect> {
        if selection.start >= placement.bytes.end || selection.end <= placement.bytes.start {
            return None;
        }
        let start = selection.start.max(placement.bytes.start);
        let end = selection.end.min(placement.bytes.end);
        let offsets = self.cluster_carets.get(placement.source_cluster)?;
        let offset_at = |byte: usize| {
            offsets
                .iter()
                .find(|offset| placement.bytes.start + offset.byte_offset as usize == byte)
                .map(|offset| placement.rect.min.x + offset.x)
        };
        let start_x = offset_at(start)?;
        let end_x = offset_at(end)?;
        Some(Rect::from_min_max(
            Point::new(start_x.min(end_x), placement.rect.min.y),
            Point::new(start_x.max(end_x), placement.rect.max.y),
        ))
    }
}

fn validate_input(input: &ParagraphGeometryInput) -> Result<(), ParagraphGeometryError> {
    if input.source.len() > MAX_PARAGRAPH_SOURCE_BYTES {
        return Err(ParagraphGeometryError::SourceTooLarge);
    }
    if input.clusters.len() > MAX_PARAGRAPH_CLUSTERS {
        return Err(ParagraphGeometryError::TooManyClusters);
    }
    if !input.wrap_width.is_finite()
        || input.wrap_width < 0.0
        || !input.line_height.is_finite()
        || input.line_height <= 0.0
    {
        return Err(ParagraphGeometryError::InvalidMetrics);
    }
    let graphemes = grapheme_boundaries(input.source.as_ref());
    let hard = hard_break_bytes(input.source.as_ref());
    let mut covered = BTreeSet::new();
    let mut previous = 0;
    for cluster in &input.clusters {
        if cluster.bytes.start >= cluster.bytes.end
            || cluster.bytes.end > input.source.len()
            || !input.source.is_char_boundary(cluster.bytes.start)
            || !input.source.is_char_boundary(cluster.bytes.end)
            || cluster.bytes.start < previous
            || !cluster.advance.is_finite()
            || cluster.advance < 0.0
            || cluster.bidi_level > 125
        {
            return Err(ParagraphGeometryError::InvalidCluster);
        }
        previous = cluster.bytes.end;
        if hard.range(cluster.bytes.clone()).next().is_some() {
            return Err(ParagraphGeometryError::InvalidCluster);
        }
        let expected: Vec<usize> = graphemes
            .iter()
            .copied()
            .filter(|offset| *offset >= cluster.bytes.start && *offset <= cluster.bytes.end)
            .collect();
        let actual: Vec<usize> = cluster
            .carets
            .iter()
            .map(|caret| cluster.bytes.start + caret.byte_offset as usize)
            .collect();
        if expected != actual
            || cluster
                .carets
                .iter()
                .any(|caret| !caret.x.is_finite() || caret.x < 0.0 || caret.x > cluster.advance)
        {
            return Err(ParagraphGeometryError::InvalidCaret);
        }
        covered.extend(cluster.bytes.clone());
    }
    if (0..input.source.len()).any(|byte| !hard.contains(&byte) && !covered.contains(&byte)) {
        return Err(ParagraphGeometryError::IncompleteCoverage);
    }
    Ok(())
}

fn hard_break_bytes(source: &str) -> BTreeSet<usize> {
    let mut bytes = BTreeSet::new();
    let raw = source.as_bytes();
    let mut offset = 0;
    while offset < raw.len() {
        if raw[offset] == b'\r' {
            bytes.insert(offset);
            if raw.get(offset + 1) == Some(&b'\n') {
                bytes.insert(offset + 1);
                offset += 2;
            } else {
                offset += 1;
            }
        } else if raw[offset] == b'\n' {
            bytes.insert(offset);
            offset += 1;
        } else {
            offset += source[offset..].chars().next().unwrap().len_utf8();
        }
    }
    bytes
}

fn hard_lines(source: &str) -> Vec<Range<usize>> {
    let raw = source.as_bytes();
    let mut lines = Vec::new();
    let mut start = 0;
    let mut offset = 0;
    while offset < raw.len() {
        if raw[offset] == b'\r' || raw[offset] == b'\n' {
            lines.push(start..offset);
            offset += usize::from(raw[offset] == b'\r' && raw.get(offset + 1) == Some(&b'\n')) + 1;
            start = offset;
        } else {
            offset += source[offset..].chars().next().unwrap().len_utf8();
        }
    }
    lines.push(start..source.len());
    lines
}

fn grapheme_boundaries(source: &str) -> BTreeSet<usize> {
    let mut result = BTreeSet::new();
    result.insert(0);
    result.insert(source.len());
    result.extend(source.grapheme_indices(true).map(|(offset, _)| offset));
    result
}

fn safe_breaks(source: &str) -> BTreeSet<usize> {
    linebreaks(source)
        .filter_map(|(offset, opportunity)| {
            matches!(opportunity, BreakOpportunity::Allowed).then_some(offset)
        })
        .collect()
}

fn wrap_line(
    range: Range<usize>,
    clusters: Range<usize>,
    source: &[ShapedLogicalCluster],
    breaks: &BTreeSet<usize>,
    width: f32,
) -> Result<Vec<(Range<usize>, Range<usize>)>, ParagraphGeometryError> {
    if clusters.is_empty() {
        return Ok(vec![(
            range.start..range.start,
            clusters.start..clusters.start,
        )]);
    }
    let mut result = Vec::new();
    let mut start = clusters.start;
    let mut cursor = start;
    let mut advance = 0.0;
    let mut last_break = None;
    while cursor < clusters.end {
        let cluster = &source[cursor];
        advance += cluster.advance;
        if cluster.safe_break_after && breaks.contains(&cluster.bytes.end) {
            last_break = Some(cursor + 1);
        }
        if advance > width && cursor > start {
            if let Some(end) = last_break.filter(|end| *end > start) {
                result.push((
                    source[start].bytes.start..source[end - 1].bytes.end,
                    start..end,
                ));
                start = end;
                cursor = start;
                advance = 0.0;
                last_break = None;
                continue;
            }
        }
        cursor += 1;
    }
    result.push((
        source[start].bytes.start..source[clusters.end - 1].bytes.end,
        start..clusters.end,
    ));
    if result
        .iter()
        .any(|(bytes, _)| bytes.start < range.start || bytes.end > range.end)
    {
        return Err(ParagraphGeometryError::InvalidByteRange);
    }
    Ok(result)
}

fn visual_line(
    source: &str,
    bytes: Range<usize>,
    clusters: Range<usize>,
    all: &[ShapedLogicalCluster],
    y: f32,
    line_height: f32,
) -> Result<
    (
        Vec<ParagraphClusterPlacement>,
        Vec<(ParagraphCaret, Point)>,
        f32,
    ),
    ParagraphGeometryError,
> {
    if clusters.is_empty() {
        let caret = (
            ParagraphCaret {
                byte: bytes.start,
                affinity: CaretAffinity::Downstream,
            },
            Point::new(0.0, y),
        );
        return Ok((Vec::new(), vec![caret], 0.0));
    }
    // Resolve the full hard paragraph, then ask unicode-bidi to reorder this line.
    // Re-resolving an already wrapped substring loses surrounding embedding context.
    let paragraph_bytes = hard_lines(source)
        .into_iter()
        .find(|paragraph| paragraph.start <= bytes.start && bytes.end <= paragraph.end)
        .ok_or(ParagraphGeometryError::BidiBoundary)?;
    let paragraph_text = &source[paragraph_bytes.clone()];
    let line = (bytes.start - paragraph_bytes.start)..(bytes.end - paragraph_bytes.start);
    let bidi = BidiInfo::new(paragraph_text, None);
    let paragraph = bidi
        .paragraphs
        .first()
        .ok_or(ParagraphGeometryError::BidiBoundary)?;
    let (_, runs) = bidi.visual_runs(paragraph, line);
    let mut visual_indices = Vec::with_capacity(clusters.len());
    for run in runs {
        let run_start = paragraph_bytes.start + run.start;
        let run_end = paragraph_bytes.start + run.end;
        let mut members: Vec<usize> = clusters
            .clone()
            .filter(|index| {
                all[*index].bytes.start >= run_start && all[*index].bytes.end <= run_end
            })
            .collect();
        if members
            .iter()
            .any(|index| all[*index].bytes.start < run_start || all[*index].bytes.end > run_end)
        {
            return Err(ParagraphGeometryError::BidiBoundary);
        }
        let level = bidi
            .levels
            .get(run.start)
            .ok_or(ParagraphGeometryError::BidiBoundary)?
            .number();
        if level % 2 == 1 {
            members.reverse();
        }
        visual_indices.extend(members);
    }
    if visual_indices.len() != clusters.len() {
        return Err(ParagraphGeometryError::BidiBoundary);
    }
    let mut x = 0.0;
    let mut placements = Vec::with_capacity(clusters.len());
    let mut carets = Vec::new();
    for index in visual_indices {
        let cluster = &all[index];
        let rect = Rect::from_xy_size(x, y, cluster.advance, line_height);
        placements.push(ParagraphClusterPlacement {
            source_cluster: index,
            bytes: cluster.bytes.clone(),
            rect,
            bidi_level: cluster.bidi_level,
        });
        for offset in &cluster.carets {
            let byte = cluster.bytes.start + offset.byte_offset as usize;
            let affinity = if byte == cluster.bytes.start {
                CaretAffinity::Downstream
            } else {
                CaretAffinity::Upstream
            };
            carets.push((
                ParagraphCaret { byte, affinity },
                Point::new(x + offset.x, y),
            ));
        }
        x += cluster.advance;
    }
    carets.sort_by_key(|entry| {
        (
            entry.0.byte,
            matches!(entry.0.affinity, CaretAffinity::Downstream),
        )
    });
    Ok((placements, carets, x))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn cluster(
        source: &str,
        bytes: Range<usize>,
        advance: f32,
        safe: bool,
    ) -> ShapedLogicalCluster {
        ShapedLogicalCluster {
            bytes: bytes.clone(),
            advance,
            bidi_level: 0,
            safe_break_after: safe,
            carets: source[bytes.clone()]
                .grapheme_indices(true)
                .map(|(i, _)| ClusterCaretOffset {
                    byte_offset: i as u32,
                    x: i as f32 * advance / bytes.len() as f32,
                })
                .chain(std::iter::once(ClusterCaretOffset {
                    byte_offset: bytes.len() as u32,
                    x: advance,
                }))
                .collect(),
        }
    }
    fn geometry(text: &str, width: f32) -> ParagraphGeometry {
        let clusters = text
            .char_indices()
            .filter_map(|(start, ch)| {
                (!matches!(ch, '\r' | '\n'))
                    .then(|| cluster(text, start..start + ch.len_utf8(), 10.0, ch == ' '))
            })
            .collect();
        ParagraphGeometry::build(ParagraphGeometryInput {
            key: ParagraphGeometryKey(1),
            source: Arc::from(text),
            clusters,
            wrap_width: width,
            line_height: 12.0,
        })
        .unwrap()
    }
    #[test]
    fn hard_and_trailing_lines() {
        let g = geometry("a\r\nb\n", 100.0);
        assert_eq!(g.lines().len(), 3);
        assert_eq!(g.lines()[2].bytes, 5..5);
    }
    #[test]
    fn soft_wraps_only_safe_boundary() {
        let g = geometry("a b", 15.0);
        assert_eq!(g.lines().len(), 2);
        let g = geometry("ab", 15.0);
        assert_eq!(g.lines().len(), 1);
    }
    #[test]
    fn combining_has_one_logical_grapheme_boundary() {
        let text = "e\u{301}";
        let bytes = 0..text.len();
        let input = ParagraphGeometryInput {
            key: ParagraphGeometryKey(1),
            source: Arc::from(text),
            clusters: vec![cluster(text, bytes, 10.0, false)],
            wrap_width: 1.0,
            line_height: 12.0,
        };
        assert_eq!(
            ParagraphGeometry::build(input)
                .unwrap()
                .hit_test(Point::new(9.0, 0.0))
                .byte,
            text.len()
        );
    }
    #[test]
    fn bidi_and_selection_use_same_placements() {
        let g = geometry("aאב", 100.0);
        assert_eq!(g.placements(0).unwrap().len(), 3);
        assert_eq!(g.selection_rects(1..3).len(), 2);
    }
    #[test]
    fn rejects_incomplete_or_nonfinite_input() {
        let source: Arc<str> = Arc::from("ab");
        let input = ParagraphGeometryInput {
            key: ParagraphGeometryKey(1),
            source,
            clusters: vec![ShapedLogicalCluster {
                bytes: 0..1,
                advance: f32::NAN,
                bidi_level: 0,
                safe_break_after: false,
                carets: vec![],
            }],
            wrap_width: 1.0,
            line_height: 1.0,
        };
        assert!(ParagraphGeometry::build(input).is_err());
    }

    #[test]
    fn rejects_cluster_cap_before_attempting_partial_geometry() {
        let cluster = ShapedLogicalCluster {
            bytes: 0..1,
            advance: 1.0,
            bidi_level: 0,
            safe_break_after: false,
            carets: vec![],
        };
        let error = ParagraphGeometry::build(ParagraphGeometryInput {
            key: ParagraphGeometryKey(1),
            source: Arc::from(""),
            clusters: vec![cluster; MAX_PARAGRAPH_CLUSTERS + 1],
            wrap_width: 1.0,
            line_height: 1.0,
        })
        .unwrap_err();
        assert_eq!(error, ParagraphGeometryError::TooManyClusters);
    }
}
