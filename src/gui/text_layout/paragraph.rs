//! Renderer-neutral, bounded paragraph geometry for shaped editor text.
//!
//! The native text provider owns shaping.  This module only validates its immutable
//! cluster snapshot, applies UAX #14 wrapping, and derives the one geometry model
//! used by painting, hit testing, and selection.
#![allow(
    missing_docs,
    reason = "the public integration facade is introduced with the text-editor widget"
)]

use crate::gui::types::{Point, Rect};
use std::{collections::BTreeSet, ops::Range, sync::Arc};
use unicode_bidi::{BidiClass, BidiInfo, Level};
use unicode_linebreak::{BreakOpportunity, linebreaks};
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

/// The paragraph embedding direction supplied to both shaping and geometry.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Default)]
pub enum ParagraphBaseDirection {
    #[default]
    Auto,
    Ltr,
    Rtl,
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
    pub base_direction: ParagraphBaseDirection,
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
    BidiLevelMismatch,
}

#[derive(Clone, Debug)]
struct HardParagraph {
    bytes: Range<usize>,
}

#[derive(Clone, Debug)]
struct PendingLine {
    bytes: Range<usize>,
    clusters: Range<usize>,
    paragraph: usize,
}

#[derive(Clone, Debug)]
struct ParagraphBidi {
    byte_start: usize,
    levels: Vec<Level>,
    classes: Vec<BidiClass>,
    base_level: Level,
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
    wrap_width: f32,
    width: f32,
    lines: Vec<ParagraphVisualLine>,
    placements: Vec<Vec<ParagraphClusterPlacement>>,
    carets: Vec<Vec<(ParagraphCaret, Point)>>,
    cluster_carets: Vec<Vec<ClusterCaretOffset>>,
}

impl ParagraphGeometry {
    pub fn build(input: ParagraphGeometryInput) -> Result<Self, ParagraphGeometryError> {
        validate_input(&input)?;
        let paragraphs = hard_paragraphs(input.source.as_ref());
        if paragraphs.len() > MAX_PARAGRAPH_LINES {
            return Err(ParagraphGeometryError::TooManyLines);
        }
        let soft_breaks = safe_breaks(input.source.as_ref());
        let bidi_paragraphs: Vec<ParagraphBidi> = paragraphs
            .iter()
            .map(|paragraph| {
                paragraph_bidi(
                    input.source.as_ref(),
                    paragraph.bytes.clone(),
                    input.base_direction,
                )
            })
            .collect::<Result<_, _>>()?;
        let mut logical_lines = Vec::new();
        for (paragraph, range) in paragraphs
            .iter()
            .map(|paragraph| paragraph.bytes.clone())
            .enumerate()
        {
            let first = input
                .clusters
                .partition_point(|cluster| cluster.bytes.end <= range.start);
            let end = input
                .clusters
                .partition_point(|cluster| cluster.bytes.start < range.end);
            let indices = first..end;
            logical_lines.extend(
                wrap_line(
                    range,
                    indices,
                    &input.clusters,
                    &soft_breaks,
                    input.wrap_width,
                )?
                .into_iter()
                .map(|(bytes, clusters)| PendingLine {
                    bytes,
                    clusters,
                    paragraph,
                }),
            );
            if logical_lines.len() > MAX_PARAGRAPH_LINES {
                return Err(ParagraphGeometryError::TooManyLines);
            }
        }

        if !((logical_lines.len() as f32) * input.line_height).is_finite() {
            return Err(ParagraphGeometryError::InvalidMetrics);
        }
        let mut width: f32 = 0.0;
        let mut lines = Vec::with_capacity(logical_lines.len());
        let mut placements = Vec::with_capacity(logical_lines.len());
        let mut carets = Vec::with_capacity(logical_lines.len());
        for (line_index, pending) in logical_lines.into_iter().enumerate() {
            let bytes = pending.bytes;
            let cluster_range = pending.clusters;
            let y = line_index as f32 * input.line_height;
            if !y.is_finite() {
                return Err(ParagraphGeometryError::InvalidMetrics);
            }
            let (line_placements, line_carets, line_width) = visual_line(
                input.source.as_ref(),
                bytes.clone(),
                cluster_range.clone(),
                &input.clusters,
                &bidi_paragraphs[pending.paragraph],
                y,
                input.line_height,
            )?;
            width = width.max(line_width);
            if !width.is_finite() {
                return Err(ParagraphGeometryError::InvalidMetrics);
            }
            lines.push(ParagraphVisualLine {
                bytes,
                y,
                width: line_width,
                cluster_range,
            });
            placements.push(line_placements);
            carets.push(line_carets);
        }
        let mut cluster_carets: Vec<_> = input
            .clusters
            .iter()
            .map(|cluster| cluster.carets.clone())
            .collect();
        for placement in placements.iter().flatten() {
            let cluster = &input.clusters[placement.source_cluster];
            if cluster.bidi_level % 2 != placement.bidi_level % 2 {
                for caret in &mut cluster_carets[placement.source_cluster] {
                    caret.x = cluster.advance - caret.x;
                }
            }
        }
        Ok(Self {
            key: input.key,
            source: input.source,
            line_height: input.line_height,
            wrap_width: input.wrap_width,
            width,
            lines,
            placements,
            carets,
            cluster_carets,
        })
    }

    /// Conservatively account retained allocations for bounded host caches.
    pub fn estimated_bytes(&self) -> usize {
        std::mem::size_of::<Self>()
            + self.source.len()
            + self.lines.capacity() * std::mem::size_of::<ParagraphVisualLine>()
            + self.placements.capacity() * std::mem::size_of::<Vec<ParagraphClusterPlacement>>()
            + self
                .placements
                .iter()
                .map(|v| v.capacity() * std::mem::size_of::<ParagraphClusterPlacement>())
                .sum::<usize>()
            + self.carets.capacity() * std::mem::size_of::<Vec<(ParagraphCaret, Point)>>()
            + self
                .carets
                .iter()
                .map(|v| v.capacity() * std::mem::size_of::<(ParagraphCaret, Point)>())
                .sum::<usize>()
            + self.cluster_carets.capacity() * std::mem::size_of::<Vec<ClusterCaretOffset>>()
            + self
                .cluster_carets
                .iter()
                .map(|v| v.capacity() * std::mem::size_of::<ClusterCaretOffset>())
                .sum::<usize>()
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
    /// Width used when resolving soft line breaks.
    pub fn wrap_width(&self) -> f32 {
        self.wrap_width
    }
    /// Fixed logical advance between visual lines.
    pub fn line_height(&self) -> f32 {
        self.line_height
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
        self.selection_rects_in_lines(selection, 0..self.lines.len())
    }
    /// Project selection only for visible visual lines, without allocating offscreen rectangles.
    pub fn selection_rects_in_lines(
        &self,
        selection: Range<usize>,
        lines: Range<usize>,
    ) -> Vec<Rect> {
        if selection.start >= selection.end {
            return Vec::new();
        }
        self.placements
            .get(lines)
            .unwrap_or_default()
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
    for paragraph in hard_paragraphs(source) {
        let end = paragraph.bytes.end;
        let next_start = hard_separator_end(source, end).unwrap_or(end);
        bytes.extend(end..next_start);
    }
    bytes
}

fn hard_paragraphs(source: &str) -> Vec<HardParagraph> {
    let mut paragraphs = Vec::new();
    let mut start = 0;
    let mut offset = 0;
    while offset < source.len() {
        if let Some(next) = hard_separator_end(source, offset) {
            paragraphs.push(HardParagraph {
                bytes: start..offset,
            });
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
    paragraphs.push(HardParagraph {
        bytes: start..source.len(),
    });
    paragraphs
}

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

fn paragraph_bidi(
    source: &str,
    bytes: Range<usize>,
    direction: ParagraphBaseDirection,
) -> Result<ParagraphBidi, ParagraphGeometryError> {
    let base = match direction {
        ParagraphBaseDirection::Auto => None,
        ParagraphBaseDirection::Ltr => Some(Level::ltr()),
        ParagraphBaseDirection::Rtl => Some(Level::rtl()),
    };
    let info = BidiInfo::new(&source[bytes.clone()], base);
    let base_level = info
        .paragraphs
        .first()
        .map(|paragraph| paragraph.level)
        .unwrap_or_else(|| base.unwrap_or_else(Level::ltr));
    Ok(ParagraphBidi {
        byte_start: bytes.start,
        levels: info.levels,
        classes: info.original_classes,
        base_level,
    })
}

fn apply_l1(text: &str, classes: &[BidiClass], levels: &mut [Level], base: Level) {
    let mut reset_from = Some(0usize);
    let mut reset_to = None;
    let mut previous = base;
    for ((offset, character), (_, length)) in text.char_indices().zip(
        text.char_indices()
            .map(|(offset, character)| (offset, character.len_utf8())),
    ) {
        match classes[offset] {
            BidiClass::B | BidiClass::S => {
                reset_to = Some(offset + character.len_utf8());
                if reset_from.is_none() {
                    reset_from = Some(offset);
                }
            }
            BidiClass::WS | BidiClass::FSI | BidiClass::LRI | BidiClass::RLI | BidiClass::PDI => {
                if reset_from.is_none() {
                    reset_from = Some(offset);
                }
            }
            BidiClass::RLE
            | BidiClass::LRE
            | BidiClass::RLO
            | BidiClass::LRO
            | BidiClass::PDF
            | BidiClass::BN => {
                if reset_from.is_none() {
                    reset_from = Some(offset);
                }
                for value in &mut levels[offset..offset + length] {
                    *value = previous;
                }
            }
            _ => reset_from = None,
        }
        if let (Some(from), Some(to)) = (reset_from, reset_to) {
            for value in &mut levels[from..to] {
                *value = base;
            }
            reset_from = None;
            reset_to = None;
        }
        previous = levels[offset];
    }
    if let Some(from) = reset_from {
        for value in &mut levels[from..] {
            *value = base;
        }
    }
}

// UTF-8 source range and corresponding logical cluster range.
type WrappedLine = (Range<usize>, Range<usize>);

fn wrap_line(
    range: Range<usize>,
    clusters: Range<usize>,
    source: &[ShapedLogicalCluster],
    breaks: &BTreeSet<usize>,
    width: f32,
) -> Result<Vec<WrappedLine>, ParagraphGeometryError> {
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
        if !(advance + cluster.advance).is_finite() {
            return Err(ParagraphGeometryError::InvalidMetrics);
        }
        advance += cluster.advance;
        if advance <= width && cluster.safe_break_after && breaks.contains(&cluster.bytes.end) {
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
            if cluster.safe_break_after && breaks.contains(&cluster.bytes.end) {
                // No fitting boundary preceded this one: retain the unavoidable
                // overflow segment, then let following words wrap independently.
                result.push((
                    source[start].bytes.start..cluster.bytes.end,
                    start..cursor + 1,
                ));
                start = cursor + 1;
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

type VisualLineGeometry = (
    Vec<ParagraphClusterPlacement>,
    Vec<(ParagraphCaret, Point)>,
    f32,
);

fn visual_line(
    source: &str,
    bytes: Range<usize>,
    clusters: Range<usize>,
    all: &[ShapedLogicalCluster],
    bidi: &ParagraphBidi,
    y: f32,
    line_height: f32,
) -> Result<VisualLineGeometry, ParagraphGeometryError> {
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
    let line = (bytes.start - bidi.byte_start)..(bytes.end - bidi.byte_start);
    let mut levels = bidi.levels[line.clone()].to_vec();
    apply_l1(
        &source[bytes.clone()],
        &bidi.classes[line.clone()],
        &mut levels,
        bidi.base_level,
    );
    let reordered_bytes = BidiInfo::reorder_visual(&levels);
    let mut visual_position = vec![usize::MAX; levels.len()];
    for (position, byte) in reordered_bytes.into_iter().enumerate() {
        visual_position[byte] = position;
    }
    let mut visual_indices: Vec<usize> = clusters.clone().collect();
    for index in &visual_indices {
        let cluster = &all[*index];
        let start = cluster.bytes.start - bytes.start;
        let end = cluster.bytes.end - bytes.start;
        let paragraph_start = cluster.bytes.start - bidi.byte_start;
        let paragraph_end = cluster.bytes.end - bidi.byte_start;
        let expected = bidi
            .levels
            .get(paragraph_start)
            .ok_or(ParagraphGeometryError::BidiBoundary)?
            .number();
        if expected != cluster.bidi_level
            || bidi.levels[paragraph_start..paragraph_end]
                .iter()
                .any(|level| level.number() != expected)
            || start >= end
            || end > levels.len()
        {
            return Err(ParagraphGeometryError::BidiLevelMismatch);
        }
    }
    visual_indices.sort_by_key(|index| visual_position[all[*index].bytes.start - bytes.start]);
    let mut x = 0.0;
    let mut placements = Vec::with_capacity(clusters.len());
    let mut carets = Vec::new();
    for index in visual_indices {
        let cluster = &all[index];
        let resolved_level = levels
            .get(cluster.bytes.start - bytes.start)
            .ok_or(ParagraphGeometryError::BidiBoundary)?
            .number();
        if !(x + cluster.advance).is_finite() {
            return Err(ParagraphGeometryError::InvalidMetrics);
        }
        let rect = Rect::from_xy_size(x, y, cluster.advance, line_height);
        placements.push(ParagraphClusterPlacement {
            source_cluster: index,
            bytes: cluster.bytes.clone(),
            rect,
            bidi_level: resolved_level,
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
                Point::new(
                    x + if cluster.bidi_level % 2 != resolved_level % 2 {
                        cluster.advance - offset.x
                    } else {
                        offset.x
                    },
                    y,
                ),
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
            bidi_level: matches!(
                source[bytes.clone()].chars().next(),
                Some('\u{590}'..='\u{8ff}')
            ) as u8,
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
            .filter(|(_, ch)| {
                !matches!(
                    ch,
                    '\u{b}' | '\u{c}' | '\r' | '\n' | '\u{85}' | '\u{2028}' | '\u{2029}'
                )
            })
            .map(|(start, ch)| cluster(text, start..start + ch.len_utf8(), 10.0, ch == ' '))
            .collect();
        ParagraphGeometry::build(ParagraphGeometryInput {
            key: ParagraphGeometryKey(1),
            source: Arc::from(text),
            clusters,
            base_direction: ParagraphBaseDirection::Auto,
            wrap_width: width,
            line_height: 12.0,
        })
        .unwrap()
    }
    #[test]
    fn wrapped_rtl_whitespace_uses_l1_caret_direction() {
        let cluster = |bytes: Range<usize>, safe| ShapedLogicalCluster {
            carets: vec![
                ClusterCaretOffset {
                    byte_offset: 0,
                    x: 8.0,
                },
                ClusterCaretOffset {
                    byte_offset: bytes.len() as u32,
                    x: 0.0,
                },
            ],
            bytes,
            advance: 8.0,
            bidi_level: 1,
            safe_break_after: safe,
        };
        let geometry = ParagraphGeometry::build(ParagraphGeometryInput {
            key: ParagraphGeometryKey(99),
            source: Arc::from("א ב"),
            clusters: vec![
                cluster(0..2, false),
                cluster(2..3, true),
                cluster(3..5, false),
            ],
            base_direction: ParagraphBaseDirection::Ltr,
            wrap_width: 16.0,
            line_height: 20.0,
        })
        .unwrap();
        assert_eq!(geometry.lines().len(), 2);
        assert_eq!(
            geometry.caret(ParagraphCaret {
                byte: 2,
                affinity: CaretAffinity::Downstream
            }),
            Some(Point::new(8.0, 0.0))
        );
        assert_eq!(
            geometry.caret(ParagraphCaret {
                byte: 3,
                affinity: CaretAffinity::Upstream
            }),
            Some(Point::new(16.0, 0.0))
        );
    }
    #[test]
    fn hard_and_trailing_lines() {
        let g = geometry("a\r\nb\n", 100.0);
        assert_eq!(g.lines().len(), 3);
        assert_eq!(g.lines()[2].bytes, 5..5);
    }
    #[test]
    fn all_uax14_mandatory_separators_create_hard_lines() {
        let g = geometry("a\u{b}b\u{c}c\u{85}d\u{2028}e\u{2029}", 100.0);
        assert_eq!(g.lines().len(), 6);
        assert!(g.lines().iter().all(|line| line.width <= 10.0));
    }
    #[test]
    fn soft_wraps_only_safe_boundary() {
        let g = geometry("a b", 15.0);
        assert_eq!(g.lines().len(), 2);
        let g = geometry("ab", 15.0);
        assert_eq!(g.lines().len(), 1);
    }
    #[test]
    fn wrap_uses_the_last_safe_boundary_that_still_fits() {
        let geometry = geometry("a b ", 25.0);
        assert_eq!(geometry.lines().len(), 2);
        assert_eq!(geometry.lines()[0].bytes, 0..2);
        assert_eq!(geometry.lines()[0].width, 20.0);
    }
    #[test]
    fn unavoidable_overflow_resets_at_its_next_safe_boundary() {
        let geometry = geometry("verylongword short short", 60.0);
        assert_eq!(geometry.lines().len(), 3);
        assert_eq!(geometry.lines()[0].bytes, 0..13);
        assert_eq!(geometry.lines()[1].width, 60.0);
        assert_eq!(geometry.lines()[2].width, 50.0);
    }
    #[test]
    fn combining_has_one_logical_grapheme_boundary() {
        let text = "e\u{301}";
        let bytes = 0..text.len();
        let input = ParagraphGeometryInput {
            key: ParagraphGeometryKey(1),
            source: Arc::from(text),
            clusters: vec![cluster(text, bytes, 10.0, false)],
            base_direction: ParagraphBaseDirection::Auto,
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
        assert_eq!(g.selection_rects(1..5).len(), 2);
    }
    #[test]
    fn rejects_provider_level_that_disagrees_with_l1_reordering() {
        let text = "aא";
        let mut right_to_left = cluster(text, 1..text.len(), 10.0, false);
        right_to_left.bidi_level = 0;
        let error = ParagraphGeometry::build(ParagraphGeometryInput {
            key: ParagraphGeometryKey(1),
            source: Arc::from(text),
            clusters: vec![cluster(text, 0..1, 10.0, false), right_to_left],
            base_direction: ParagraphBaseDirection::Ltr,
            wrap_width: 100.0,
            line_height: 12.0,
        })
        .unwrap_err();
        assert_eq!(error, ParagraphGeometryError::BidiLevelMismatch);
    }
    #[test]
    fn l1_adjusts_trailing_whitespace_from_pre_l1_level() {
        let mut levels = vec![Level::rtl(), Level::rtl(), Level::rtl()];
        apply_l1(
            "א ",
            &[BidiClass::R, BidiClass::R, BidiClass::WS],
            &mut levels,
            Level::ltr(),
        );
        assert_eq!(levels[0].number(), 1);
        assert_eq!(levels[2].number(), 0);
    }
    #[test]
    fn narrow_wrap_reuses_one_precomputed_paragraph_bidi_snapshot() {
        let text = "a ".repeat(1_024);
        let geometry = geometry(&text, 10.0);
        assert_eq!(geometry.lines().len(), 1_024);
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
            base_direction: ParagraphBaseDirection::Auto,
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
            base_direction: ParagraphBaseDirection::Auto,
            wrap_width: 1.0,
            line_height: 1.0,
        })
        .unwrap_err();
        assert_eq!(error, ParagraphGeometryError::TooManyClusters);
    }
    #[test]
    fn rejects_finite_advances_whose_sum_overflows() {
        let text = "ab";
        let error = ParagraphGeometry::build(ParagraphGeometryInput {
            key: ParagraphGeometryKey(1),
            source: Arc::from(text),
            clusters: vec![
                cluster(text, 0..1, f32::MAX, false),
                cluster(text, 1..2, f32::MAX, false),
            ],
            base_direction: ParagraphBaseDirection::Ltr,
            wrap_width: f32::MAX,
            line_height: 1.0,
        })
        .unwrap_err();
        assert_eq!(error, ParagraphGeometryError::InvalidMetrics);
    }
}
