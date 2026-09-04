//! Retained Unicode paragraph shaping and geometry construction.

use super::{
    BidiDirection, BidiRun, CaretStopGeometry, GlyphPlacement, GraphemeBoundary, GraphemeGeometry,
    LineBreakKind, LineBreakPolicyId, LineBreakRecord, ResolvedFontRun, ScalarBoundary,
    ShapeClusterRange, ShapedParagraph, SnapshotQuality, TextQuality, Utf8ByteOffset,
    font::NativeFontStack,
    model::{LINE_BREAK_POLICY_ID, source_revision},
};
#[cfg(test)]
use super::{ParagraphSnapshot, TextLayout};
#[cfg(test)]
use crate::gui::paint::TextAlign;
use std::{ops::Range, sync::Arc};
use unicode_bidi::BidiInfo;
use unicode_linebreak::{BreakOpportunity, linebreaks};
use unicode_script::{Script, UnicodeScript};
use unicode_segmentation::UnicodeSegmentation;

const COMPATIBILITY_REPLACEMENT_WIDTH_EM: f32 = 0.5;
const TAB_WIDTH_EM: f32 = 2.0;

#[cfg(test)]
pub(super) fn compute_layout(
    font_stack: &mut NativeFontStack,
    text: &str,
    font_size: f32,
) -> Option<TextLayout> {
    if !font_size.is_finite() || font_size <= 0.0 {
        return None;
    }
    let source: Arc<str> = Arc::from(text);
    let shaped = compute_shaped_paragraph(font_stack, source.clone(), font_size)
        .unwrap_or_else(|_| compute_compatibility_paragraph(font_stack, source, font_size));
    Some(TextLayout::from_snapshot(ParagraphSnapshot::from_shaped(
        shaped,
        None,
        TextAlign::Left,
    )))
}

pub(super) fn compute_shaped_paragraph(
    font_stack: &mut NativeFontStack,
    source: Arc<str>,
    font_size: f32,
) -> Result<Arc<ShapedParagraph>, ()> {
    if !font_size.is_finite() || font_size <= 0.0 {
        return Err(());
    }
    let scalar_boundaries = scalar_boundaries(source.as_ref());
    let grapheme_boundaries = grapheme_boundaries(source.as_ref());
    let breaks = line_break_records(source.as_ref(), &grapheme_boundaries)?;
    let active_range = first_line_range(source.as_ref());
    let mut resolved_font_runs = Vec::new();
    let mut bidi_runs = Vec::new();
    let mut glyphs = Vec::new();
    let mut grapheme_geometry = Vec::new();
    let mut width = 0.0;
    let mut fallback_glyphs = 0;
    let mut missing_glyphs = 0;

    if !active_range.is_empty() {
        let active = &source[active_range.clone()];
        let bidi = BidiInfo::new(active, None);
        let Some(paragraph) = bidi.paragraphs.first() else {
            return Err(());
        };
        let (levels, visual_runs) = bidi.visual_runs(paragraph, paragraph.range.clone());
        for (visual_index, visual_run) in visual_runs.into_iter().enumerate() {
            if visual_run.is_empty()
                || visual_run.start >= active.len()
                || visual_run.end > active.len()
                || !active.is_char_boundary(visual_run.start)
                || !active.is_char_boundary(visual_run.end)
            {
                return Err(());
            }
            let level = levels.get(visual_run.start).ok_or(())?.number();
            let direction = if level.is_multiple_of(2) {
                BidiDirection::Ltr
            } else {
                BidiDirection::Rtl
            };
            let global_run =
                (active_range.start + visual_run.start)..(active_range.start + visual_run.end);
            let bidi_run_index = bidi_runs.len();
            bidi_runs.push(BidiRun {
                range: global_run.clone(),
                level,
                direction,
                visual_index,
            });

            let segments = font_segments(
                font_stack,
                source.as_ref(),
                global_run.clone(),
                &grapheme_boundaries,
            )?;
            let mut fragments = Vec::with_capacity(segments.len());
            for segment in segments {
                let fragment = if segment.special {
                    special_fragment(
                        &segment.range,
                        source.as_ref(),
                        &grapheme_boundaries,
                        font_size,
                    )
                } else if segment.face_index.is_some() {
                    shape_face_fragment(
                        font_stack,
                        source.as_ref(),
                        &segment,
                        direction,
                        &grapheme_boundaries,
                        font_size,
                    )?
                } else {
                    let (fragment, used_replacement) = missing_fragment(
                        font_stack,
                        segment.range.clone(),
                        direction,
                        &grapheme_boundaries,
                        font_size,
                    );
                    if used_replacement {
                        fallback_glyphs += 1;
                    } else {
                        missing_glyphs += 1;
                    }
                    fragment
                };
                fragments.push((segment, fragment));
            }

            let run_width = fragments
                .iter()
                .map(|(_, fragment)| fragment.width)
                .sum::<f32>();
            if !run_width.is_finite() || run_width < 0.0 {
                return Err(());
            }
            let mut run_x = width;
            let fragment_indices: Vec<usize> = if direction == BidiDirection::Rtl {
                (0..fragments.len()).rev().collect()
            } else {
                (0..fragments.len()).collect()
            };
            for fragment_index in fragment_indices {
                let (segment, fragment) = &fragments[fragment_index];
                resolved_font_runs.push(ResolvedFontRun {
                    range: segment.range.clone(),
                    face_index: segment.face_index,
                    direction,
                });
                for glyph in &fragment.glyphs {
                    glyphs.push(GlyphPlacement {
                        face_index: glyph.face_index,
                        glyph_id: glyph.glyph_id,
                        cluster: glyph.cluster,
                        x: glyph.x + run_x,
                        y_offset: glyph.y_offset,
                        x_offset: glyph.x_offset,
                        advance: glyph.advance,
                        run_index: bidi_run_index,
                    });
                }
                for geometry in &fragment.grapheme_geometry {
                    grapheme_geometry.push(GraphemeGeometry {
                        range: geometry.range,
                        grapheme_index: geometry.grapheme_index,
                        x_start: geometry.x_start + run_x,
                        x_end: geometry.x_end + run_x,
                        direction: geometry.direction,
                        visual_index: 0,
                    });
                }
                run_x += fragment.width;
            }
            width += run_width;
        }
    }

    if width.is_nan() || width.is_infinite() {
        return Err(());
    }
    finish_geometry(GeometryInput {
        source,
        font_size,
        scalar_boundaries,
        grapheme_boundaries,
        breaks,
        break_policy_id: LINE_BREAK_POLICY_ID,
        resolved_font_runs,
        bidi_runs,
        glyphs,
        grapheme_geometry,
        width,
        quality: TextQuality {
            unsupported_shaping_runs: 0,
            unsupported_shaping_scalars: 0,
            fallback_glyphs,
            missing_glyphs,
        },
        quality_kind: SnapshotQuality::Shaped,
        font_generation: font_stack.generation(),
    })
}

pub(super) fn compute_compatibility_paragraph(
    font_stack: &mut NativeFontStack,
    source: Arc<str>,
    font_size: f32,
) -> Arc<ShapedParagraph> {
    let font_size = if font_size.is_finite() && font_size > 0.0 {
        font_size
    } else {
        1.0
    };
    let scalar_boundaries = scalar_boundaries(source.as_ref());
    let grapheme_boundaries = grapheme_boundaries(source.as_ref());
    let breaks = line_break_records_lossy(source.as_ref(), &grapheme_boundaries);
    let active_range = first_line_range(source.as_ref());
    let active_end = active_range.end;
    let mut glyphs = Vec::new();
    let mut grapheme_geometry = Vec::new();
    let mut resolved_font_runs = Vec::new();
    let mut x = 0.0;
    let mut fallback_glyphs = 0;
    let mut missing_glyphs = 0;
    let active_graphemes = grapheme_ranges(&grapheme_boundaries, active_range.clone());

    for range in active_graphemes {
        let grapheme = &source[range.clone()];
        let grapheme_start =
            grapheme_index_for_byte(&grapheme_boundaries, range.start).unwrap_or(0);
        let before = x;
        let mut resolved_face = None;
        if grapheme == "\t" {
            x += font_size * TAB_WIDTH_EM;
        } else if grapheme.chars().all(char::is_control) {
            // Keep compatibility geometry deterministic without drawing controls.
        } else {
            resolved_face = font_stack.resolve_grapheme_face(grapheme);
            let covered_glyphs = resolved_face.and_then(|face_index| {
                grapheme
                    .char_indices()
                    .map(|(offset, character)| {
                        let glyph = font_stack.resolve_glyph_in_face(face_index, character)?;
                        let advance = font_stack.glyph_advance(glyph, font_size);
                        (advance.is_finite() && advance >= 0.0)
                            .then_some((offset, character, glyph, advance))
                    })
                    .collect::<Option<Vec<_>>>()
            });
            if let Some(covered_glyphs) = covered_glyphs {
                for (offset, character, font_glyph, advance) in covered_glyphs {
                    glyphs.push(GlyphPlacement {
                        face_index: font_glyph.face_index,
                        glyph_id: font_glyph.glyph_id,
                        cluster: ShapeClusterRange {
                            start: Utf8ByteOffset(range.start + offset),
                            end: Utf8ByteOffset(range.start + offset + character.len_utf8()),
                        },
                        x,
                        y_offset: 0.0,
                        x_offset: 0.0,
                        advance,
                        run_index: 0,
                    });
                    x += advance;
                }
            } else {
                resolved_face = None;
                let replacement = font_stack.fallback_glyph();
                if let Some(replacement) = replacement {
                    fallback_glyphs += 1;
                    let advance = font_stack.glyph_advance(replacement, font_size);
                    glyphs.push(GlyphPlacement {
                        face_index: replacement.face_index,
                        glyph_id: replacement.glyph_id,
                        cluster: ShapeClusterRange {
                            start: Utf8ByteOffset(range.start),
                            end: Utf8ByteOffset(range.end),
                        },
                        x,
                        y_offset: 0.0,
                        x_offset: 0.0,
                        advance,
                        run_index: 0,
                    });
                    x += advance;
                } else {
                    missing_glyphs += 1;
                    x += font_size * COMPATIBILITY_REPLACEMENT_WIDTH_EM;
                }
            }
        }
        if x < before {
            x = before;
        }
        grapheme_geometry.push(GraphemeGeometry {
            range: ShapeClusterRange {
                start: Utf8ByteOffset(range.start),
                end: Utf8ByteOffset(range.end),
            },
            grapheme_index: grapheme_start,
            x_start: before,
            x_end: x,
            direction: BidiDirection::Ltr,
            visual_index: 0,
        });
        resolved_font_runs.push(ResolvedFontRun {
            range,
            face_index: resolved_face,
            direction: BidiDirection::Ltr,
        });
    }

    let unsupported_scalars = source[..active_end].chars().count() as u64;
    let bidi_runs = if active_range.is_empty() {
        Vec::new()
    } else {
        vec![BidiRun {
            range: active_range,
            level: 0,
            direction: BidiDirection::Ltr,
            visual_index: 0,
        }]
    };
    let emergency_source = source.clone();
    finish_geometry(GeometryInput {
        source,
        font_size,
        scalar_boundaries,
        grapheme_boundaries,
        breaks,
        break_policy_id: LINE_BREAK_POLICY_ID,
        resolved_font_runs,
        bidi_runs,
        glyphs,
        grapheme_geometry,
        width: x,
        quality: TextQuality {
            unsupported_shaping_runs: u64::from(active_end > 0),
            unsupported_shaping_scalars: unsupported_scalars,
            fallback_glyphs,
            missing_glyphs,
        },
        quality_kind: SnapshotQuality::CompatibilityFallback,
        font_generation: font_stack.generation(),
    })
    .unwrap_or_else(|_| {
        emergency_compatibility_paragraph(
            emergency_source,
            font_size,
            fallback_glyphs,
            missing_glyphs,
            font_stack.generation(),
        )
    })
}

fn emergency_compatibility_paragraph(
    source: Arc<str>,
    font_size: f32,
    fallback_glyphs: u64,
    missing_glyphs: u64,
    font_generation: u64,
) -> Arc<ShapedParagraph> {
    let font_size = if font_size.is_finite() && font_size > 0.0 {
        font_size
    } else {
        1.0
    };
    let scalar_boundaries = scalar_boundaries(source.as_ref());
    let grapheme_boundaries = grapheme_boundaries(source.as_ref());
    let active_range = first_line_range(source.as_ref());
    let mut x = 0.0;
    let mut geometry = Vec::new();
    for range in grapheme_ranges(&grapheme_boundaries, active_range.clone()) {
        let width = if &source[range.clone()] == "\t" {
            font_size * TAB_WIDTH_EM
        } else {
            font_size * COMPATIBILITY_REPLACEMENT_WIDTH_EM
        };
        geometry.push(GraphemeGeometry {
            range: ShapeClusterRange {
                start: Utf8ByteOffset(range.start),
                end: Utf8ByteOffset(range.end),
            },
            grapheme_index: grapheme_index_for_byte(&grapheme_boundaries, range.start).unwrap_or(0),
            x_start: x,
            x_end: x + width,
            direction: BidiDirection::Ltr,
            visual_index: 0,
        });
        x += width;
    }
    let bidi_runs = if active_range.is_empty() {
        Vec::new()
    } else {
        vec![BidiRun {
            range: active_range,
            level: 0,
            direction: BidiDirection::Ltr,
            visual_index: 0,
        }]
    };
    match finish_geometry(GeometryInput {
        source: source.clone(),
        font_size,
        scalar_boundaries,
        grapheme_boundaries: grapheme_boundaries.clone(),
        breaks: line_break_records_lossy(source.as_ref(), &grapheme_boundaries),
        break_policy_id: LINE_BREAK_POLICY_ID,
        resolved_font_runs: Vec::new(),
        bidi_runs,
        glyphs: Vec::new(),
        grapheme_geometry: geometry,
        width: x,
        quality: TextQuality {
            unsupported_shaping_runs: u64::from(!source.is_empty()),
            unsupported_shaping_scalars: source.chars().count() as u64,
            fallback_glyphs,
            missing_glyphs,
        },
        quality_kind: SnapshotQuality::CompatibilityFallback,
        font_generation,
    }) {
        Ok(paragraph) => paragraph,
        Err(()) => minimal_compatibility_paragraph(source, font_size, font_generation),
    }
}

fn minimal_compatibility_paragraph(
    source: Arc<str>,
    font_size: f32,
    font_generation: u64,
) -> Arc<ShapedParagraph> {
    let scalar_boundaries = scalar_boundaries(source.as_ref());
    let grapheme_boundaries = grapheme_boundaries(source.as_ref());
    let breaks = hard_break_records(source.as_ref(), &grapheme_boundaries);
    let quality = TextQuality {
        unsupported_shaping_runs: u64::from(!source.is_empty()),
        unsupported_shaping_scalars: source.chars().count() as u64,
        fallback_glyphs: 0,
        missing_glyphs: source.chars().count() as u64,
    };
    finish_geometry(GeometryInput {
        source: source.clone(),
        font_size,
        scalar_boundaries,
        grapheme_boundaries,
        breaks,
        break_policy_id: LINE_BREAK_POLICY_ID,
        resolved_font_runs: Vec::new(),
        bidi_runs: Vec::new(),
        glyphs: Vec::new(),
        grapheme_geometry: Vec::new(),
        width: 0.0,
        quality,
        quality_kind: SnapshotQuality::CompatibilityFallback,
        font_generation,
    })
    .unwrap_or_else(|()| {
        Arc::new(empty_compatibility_shape(
            source,
            font_size,
            quality,
            font_generation,
        ))
    })
}

fn empty_compatibility_shape(
    source: Arc<str>,
    font_size: f32,
    quality: TextQuality,
    font_generation: u64,
) -> ShapedParagraph {
    let scalar_boundaries = scalar_boundaries(source.as_ref());
    let grapheme_boundaries = grapheme_boundaries(source.as_ref());
    ShapedParagraph {
        source_identity: super::model::source_identity(source.as_ref()),
        revision: source_revision(source.as_ref(), font_size.to_bits(), font_generation),
        font_size_bits: font_size.to_bits(),
        breaks: hard_break_records(source.as_ref(), &grapheme_boundaries),
        source,
        scalar_boundaries,
        grapheme_boundaries,
        break_policy_id: LINE_BREAK_POLICY_ID,
        resolved_font_runs: Vec::new(),
        bidi_runs: Vec::new(),
        glyphs: Vec::new(),
        grapheme_geometry: Vec::new(),
        caret_geometry: Vec::new(),
        logical_to_visual: Vec::new(),
        visual_to_logical: Vec::new(),
        width: 0.0,
        quality,
        quality_kind: SnapshotQuality::CompatibilityFallback,
    }
}

struct GeometryInput {
    source: Arc<str>,
    font_size: f32,
    scalar_boundaries: Vec<Utf8ByteOffset>,
    grapheme_boundaries: Vec<Utf8ByteOffset>,
    breaks: Vec<LineBreakRecord>,
    break_policy_id: LineBreakPolicyId,
    resolved_font_runs: Vec<ResolvedFontRun>,
    bidi_runs: Vec<BidiRun>,
    glyphs: Vec<GlyphPlacement>,
    grapheme_geometry: Vec<GraphemeGeometry>,
    width: f32,
    quality: TextQuality,
    quality_kind: SnapshotQuality,
    font_generation: u64,
}

fn finish_geometry(input: GeometryInput) -> Result<Arc<ShapedParagraph>, ()> {
    let GeometryInput {
        source,
        font_size,
        scalar_boundaries,
        grapheme_boundaries,
        breaks,
        break_policy_id,
        resolved_font_runs,
        bidi_runs,
        mut glyphs,
        mut grapheme_geometry,
        width,
        quality,
        quality_kind,
        font_generation,
    } = input;
    if !width.is_finite()
        || width < 0.0
        || scalar_boundaries.is_empty()
        || grapheme_boundaries.is_empty()
        || scalar_boundaries.first() != Some(&Utf8ByteOffset(0))
        || scalar_boundaries.last() != Some(&Utf8ByteOffset(source.len()))
        || grapheme_boundaries.first() != Some(&Utf8ByteOffset(0))
        || grapheme_boundaries.last() != Some(&Utf8ByteOffset(source.len()))
        || scalar_boundaries
            .windows(2)
            .any(|window| window[0].0 >= window[1].0)
        || grapheme_boundaries
            .windows(2)
            .any(|window| window[0].0 >= window[1].0)
        || scalar_boundaries
            .iter()
            .any(|boundary| boundary.0 > source.len() || !source.is_char_boundary(boundary.0))
        || grapheme_boundaries
            .iter()
            .any(|boundary| boundary.0 > source.len() || !source.is_char_boundary(boundary.0))
        || break_policy_id != LINE_BREAK_POLICY_ID
        || breaks_are_invalid(&source, &grapheme_boundaries, &breaks)
        || grapheme_geometry
            .iter()
            .any(|geometry| !geometry.x_start.is_finite() || !geometry.x_end.is_finite())
        || glyphs.iter().any(|glyph| {
            !glyph.x.is_finite()
                || !glyph.advance.is_finite()
                || glyph.advance < 0.0
                || !glyph.x_offset.is_finite()
                || !glyph.y_offset.is_finite()
                || glyph.cluster.start.0 >= glyph.cluster.end.0
                || glyph.cluster.end.0 > source.len()
                || !source.is_char_boundary(glyph.cluster.start.0)
                || !source.is_char_boundary(glyph.cluster.end.0)
                || !is_grapheme_boundary(&grapheme_boundaries, glyph.cluster.start.0)
                || !is_grapheme_boundary(&grapheme_boundaries, glyph.cluster.end.0)
        })
        || resolved_font_runs.iter().any(|run| {
            run.range.is_empty()
                || run.range.end > source.len()
                || !source.is_char_boundary(run.range.start)
                || !source.is_char_boundary(run.range.end)
                || !is_grapheme_boundary(&grapheme_boundaries, run.range.start)
                || !is_grapheme_boundary(&grapheme_boundaries, run.range.end)
        })
        || bidi_runs.iter().any(|run| {
            run.range.is_empty()
                || run.range.end > source.len()
                || !source.is_char_boundary(run.range.start)
                || !source.is_char_boundary(run.range.end)
                || run.level > 125
        })
    {
        return Err(());
    }

    grapheme_geometry.sort_by_key(|geometry| geometry.grapheme_index);
    let expected_geometry = grapheme_geometry
        .iter()
        .enumerate()
        .all(|(index, geometry)| {
            geometry.grapheme_index == index
                && grapheme_boundaries
                    .get(index)
                    .is_some_and(|start| geometry.range.start == *start)
                && grapheme_boundaries
                    .get(index + 1)
                    .is_some_and(|end| geometry.range.end == *end)
        });
    if !expected_geometry || grapheme_geometry.len() > grapheme_boundaries.len().saturating_sub(1) {
        return Err(());
    }

    let mut visual_order = (0..grapheme_geometry.len()).collect::<Vec<_>>();
    visual_order.sort_by(|left, right| {
        let left_geometry = &grapheme_geometry[*left];
        let right_geometry = &grapheme_geometry[*right];
        left_geometry
            .x_start
            .min(left_geometry.x_end)
            .total_cmp(&right_geometry.x_start.min(right_geometry.x_end))
            .then_with(|| left.cmp(right))
    });
    let mut logical_to_visual = vec![usize::MAX; grapheme_geometry.len()];
    for (visual_index, logical_index) in visual_order.iter().copied().enumerate() {
        logical_to_visual[logical_index] = visual_index;
        grapheme_geometry[logical_index].visual_index = visual_index;
    }

    let mut caret_geometry = Vec::with_capacity(grapheme_boundaries.len());
    for (boundary_index, byte) in grapheme_boundaries.iter().copied().enumerate() {
        let scalar = scalar_boundaries
            .binary_search(&byte)
            .map(ScalarBoundary)
            .map_err(|_| ())?;
        let previous = boundary_index
            .checked_sub(1)
            .and_then(|index| grapheme_geometry.get(index));
        let current = grapheme_geometry.get(boundary_index);
        let upstream_x = previous
            .map(grapheme_trailing_edge)
            .or_else(|| current.map(grapheme_leading_edge))
            .or(Some(width));
        let downstream_x = current
            .map(grapheme_leading_edge)
            .or_else(|| previous.map(grapheme_trailing_edge))
            .or(Some(width));
        caret_geometry.push(CaretStopGeometry {
            byte,
            scalar,
            grapheme: GraphemeBoundary(boundary_index),
            upstream_x,
            downstream_x,
        });
    }

    for glyph in &mut glyphs {
        if glyph.run_index >= bidi_runs.len() && !bidi_runs.is_empty() {
            return Err(());
        }
    }
    if glyphs.iter().any(|_| bidi_runs.is_empty())
        || logical_to_visual.len() != grapheme_geometry.len()
        || visual_order.len() != grapheme_geometry.len()
        || visual_order.iter().enumerate().any(|(visual, logical)| {
            *logical >= grapheme_geometry.len() || logical_to_visual.get(*logical) != Some(&visual)
        })
        || caret_geometry_is_invalid(&scalar_boundaries, &grapheme_boundaries, &caret_geometry)
    {
        return Err(());
    }
    let source_identity = super::model::source_identity(source.as_ref());
    let revision = source_revision(source.as_ref(), font_size.to_bits(), font_generation);
    Ok(Arc::new(ShapedParagraph {
        source,
        source_identity,
        revision,
        font_size_bits: font_size.to_bits(),
        scalar_boundaries,
        grapheme_boundaries,
        breaks,
        break_policy_id,
        resolved_font_runs,
        bidi_runs,
        glyphs,
        grapheme_geometry,
        caret_geometry,
        logical_to_visual,
        visual_to_logical: visual_order,
        width,
        quality,
        quality_kind,
    }))
}

fn breaks_are_invalid(
    source: &str,
    grapheme_boundaries: &[Utf8ByteOffset],
    breaks: &[LineBreakRecord],
) -> bool {
    breaks.is_empty()
        || breaks
            .windows(2)
            .any(|window| window[0].byte >= window[1].byte)
        || breaks.iter().any(|record| {
            record.byte.0 > source.len()
                || !source.is_char_boundary(record.byte.0)
                || !is_grapheme_boundary(grapheme_boundaries, record.byte.0)
                || grapheme_boundaries
                    .get(record.grapheme.0)
                    .is_none_or(|boundary| boundary.0 != record.byte.0)
        })
        || breaks.last().is_none_or(|record| {
            record.byte.0 != source.len() || record.kind != LineBreakKind::Mandatory
        })
}

fn caret_geometry_is_invalid(
    scalar_boundaries: &[Utf8ByteOffset],
    grapheme_boundaries: &[Utf8ByteOffset],
    caret_geometry: &[CaretStopGeometry],
) -> bool {
    caret_geometry.len() != grapheme_boundaries.len()
        || caret_geometry.iter().enumerate().any(|(index, caret)| {
            caret.grapheme.0 != index
                || caret.byte != grapheme_boundaries[index]
                || scalar_boundaries.binary_search(&caret.byte).is_err()
                || [caret.upstream_x, caret.downstream_x]
                    .into_iter()
                    .flatten()
                    .any(|x| !x.is_finite())
        })
}

fn grapheme_leading_edge(geometry: &GraphemeGeometry) -> f32 {
    match geometry.direction {
        BidiDirection::Ltr => geometry.x_start,
        BidiDirection::Rtl => geometry.x_end,
    }
}

fn grapheme_trailing_edge(geometry: &GraphemeGeometry) -> f32 {
    match geometry.direction {
        BidiDirection::Ltr => geometry.x_end,
        BidiDirection::Rtl => geometry.x_start,
    }
}

#[derive(Clone, Debug)]
struct FontSegment {
    range: Range<usize>,
    face_index: Option<usize>,
    special: bool,
    script: Script,
}

#[derive(Clone, Debug)]
struct Fragment {
    glyphs: Vec<GlyphPlacement>,
    grapheme_geometry: Vec<GraphemeGeometry>,
    width: f32,
}

fn font_segments(
    font_stack: &mut NativeFontStack,
    source: &str,
    range: Range<usize>,
    grapheme_boundaries: &[Utf8ByteOffset],
) -> Result<Vec<FontSegment>, ()> {
    let ranges = grapheme_ranges(grapheme_boundaries, range.clone());
    let scripts = ranges
        .iter()
        .map(|grapheme_range| grapheme_script(&source[grapheme_range.clone()]))
        .collect::<Vec<_>>();
    let scripts = contextualize_scripts(&scripts);
    let mut segments: Vec<FontSegment> = Vec::new();
    for (grapheme_range, script) in ranges.into_iter().zip(scripts) {
        let grapheme = &source[grapheme_range.clone()];
        let special = grapheme == "\t" || grapheme.chars().all(char::is_control);
        let face_index = if special {
            None
        } else {
            font_stack.resolve_grapheme_face(grapheme)
        };
        if face_index.is_some()
            && let Some(previous) = segments.last_mut()
            && previous.face_index == face_index
            && previous.script == script
            && previous.range.end == grapheme_range.start
        {
            previous.range.end = grapheme_range.end;
        } else {
            segments.push(FontSegment {
                range: grapheme_range,
                face_index,
                special,
                script,
            });
        }
    }
    Ok(segments)
}

fn grapheme_script(grapheme: &str) -> Script {
    grapheme
        .chars()
        .map(|character| character.script())
        .find(|script| !matches!(script, Script::Common | Script::Inherited))
        .unwrap_or(Script::Common)
}

fn contextualize_scripts(scripts: &[Script]) -> Vec<Script> {
    let mut next_strong = vec![None; scripts.len()];
    let mut following = None;
    for (index, script) in scripts.iter().enumerate().rev() {
        next_strong[index] = following;
        if !matches!(script, Script::Common | Script::Inherited) {
            following = Some(*script);
        }
    }

    let mut preceding = None;
    scripts
        .iter()
        .enumerate()
        .map(|(index, script)| {
            if matches!(script, Script::Common | Script::Inherited) {
                preceding.or(next_strong[index]).unwrap_or(Script::Common)
            } else {
                preceding = Some(*script);
                *script
            }
        })
        .collect()
}

fn special_fragment(
    range: &Range<usize>,
    source: &str,
    grapheme_boundaries: &[Utf8ByteOffset],
    font_size: f32,
) -> Fragment {
    let grapheme_index = grapheme_index_for_byte(grapheme_boundaries, range.start).unwrap_or(0);
    let width = if &source[range.clone()] == "\t" {
        font_size * TAB_WIDTH_EM
    } else {
        0.0
    };
    Fragment {
        glyphs: Vec::new(),
        grapheme_geometry: vec![GraphemeGeometry {
            range: ShapeClusterRange {
                start: Utf8ByteOffset(range.start),
                end: Utf8ByteOffset(range.end),
            },
            grapheme_index,
            x_start: 0.0,
            x_end: width,
            direction: BidiDirection::Ltr,
            visual_index: 0,
        }],
        width,
    }
}

fn shape_face_fragment(
    font_stack: &NativeFontStack,
    source: &str,
    segment: &FontSegment,
    direction: BidiDirection,
    grapheme_boundaries: &[Utf8ByteOffset],
    font_size: f32,
) -> Result<Fragment, ()> {
    let range = &segment.range;
    let face_index = segment.face_index.ok_or(())?;
    let font = font_stack.face_data(face_index).ok_or(())?;
    let face = rustybuzz::Face::from_slice(font.data.as_ref(), font.index).ok_or(())?;
    let mut buffer = rustybuzz::UnicodeBuffer::new();
    buffer.push_str(&source[range.clone()]);
    buffer.set_direction(match direction {
        BidiDirection::Ltr => rustybuzz::Direction::LeftToRight,
        BidiDirection::Rtl => rustybuzz::Direction::RightToLeft,
    });
    buffer.set_script(rustybuzz_script(segment.script));
    let output = rustybuzz::shape(&face, &[], buffer);
    if output.is_empty() {
        return Err(());
    }
    let infos = output.glyph_infos();
    let positions = output.glyph_positions();
    if infos.len() != positions.len() {
        return Err(());
    }
    let mut cluster_starts = infos
        .iter()
        .map(|info| info.cluster as usize)
        .collect::<Vec<_>>();
    cluster_starts.sort_unstable();
    cluster_starts.dedup();
    if cluster_starts
        .iter()
        .any(|start| *start >= source[range.clone()].len())
    {
        return Err(());
    }
    let upem = face.units_per_em();
    if upem <= 0 {
        return Err(());
    }
    let scale = font_size / upem as f32;
    let advances = positions
        .iter()
        .map(|position| position.x_advance as f32 * scale)
        .collect::<Vec<_>>();
    if advances
        .iter()
        .any(|advance| !advance.is_finite() || *advance < 0.0)
    {
        return Err(());
    }
    let width = advances.iter().sum::<f32>();
    let mut cursor = 0.0;
    let mut glyphs = Vec::with_capacity(infos.len());
    for ((info, position), advance) in infos.iter().zip(positions).zip(advances.iter().copied()) {
        let local_cluster_start = info.cluster as usize;
        let next_cluster = cluster_starts
            .iter()
            .copied()
            .find(|candidate| *candidate > local_cluster_start)
            .unwrap_or(source[range.clone()].len());
        let cluster_start = range.start + local_cluster_start;
        let cluster_end = range.start + next_cluster;
        if !source.is_char_boundary(cluster_start)
            || !source.is_char_boundary(cluster_end)
            || !is_grapheme_boundary(grapheme_boundaries, cluster_start)
            || !is_grapheme_boundary(grapheme_boundaries, cluster_end)
        {
            return Err(());
        }
        let x = if direction == BidiDirection::Rtl {
            cursor += advance;
            width - cursor
        } else {
            let x = cursor;
            cursor += advance;
            x
        };
        glyphs.push(GlyphPlacement {
            face_index,
            glyph_id: info.glyph_id,
            cluster: ShapeClusterRange {
                start: Utf8ByteOffset(cluster_start),
                end: Utf8ByteOffset(cluster_end),
            },
            x,
            y_offset: position.y_offset as f32 * scale,
            x_offset: position.x_offset as f32 * scale,
            advance,
            run_index: 0,
        });
    }
    if glyphs.iter().any(|glyph| glyph.glyph_id == 0) {
        return Err(());
    }
    let mut geometry = Vec::new();
    for cluster_start in cluster_starts.iter().copied() {
        let local_end = cluster_starts
            .iter()
            .copied()
            .find(|candidate| *candidate > cluster_start)
            .unwrap_or(source[range.clone()].len());
        let cluster_range = ShapeClusterRange {
            start: Utf8ByteOffset(range.start + cluster_start),
            end: Utf8ByteOffset(range.start + local_end),
        };
        let mut x_start = f32::INFINITY;
        let mut x_end = f32::NEG_INFINITY;
        for glyph in &glyphs {
            if glyph.cluster == cluster_range {
                x_start = x_start.min(glyph.x);
                x_end = x_end.max(glyph.x + glyph.advance);
            }
        }
        if !x_start.is_finite() || !x_end.is_finite() {
            return Err(());
        }
        let cluster_graphemes = grapheme_ranges(
            grapheme_boundaries,
            cluster_range.start.0..cluster_range.end.0,
        );
        if cluster_graphemes.is_empty() {
            return Err(());
        }
        let cluster_width = (x_end - x_start).max(0.0);
        let part = cluster_width / cluster_graphemes.len() as f32;
        for (index, grapheme_range) in cluster_graphemes.into_iter().enumerate() {
            let (start, end) = if direction == BidiDirection::Rtl {
                (
                    x_end - part * (index + 1) as f32,
                    x_end - part * index as f32,
                )
            } else {
                (
                    x_start + part * index as f32,
                    x_start + part * (index + 1) as f32,
                )
            };
            let grapheme_index =
                grapheme_index_for_byte(grapheme_boundaries, grapheme_range.start).ok_or(())?;
            geometry.push(GraphemeGeometry {
                range: ShapeClusterRange {
                    start: Utf8ByteOffset(grapheme_range.start),
                    end: Utf8ByteOffset(grapheme_range.end),
                },
                grapheme_index,
                x_start: start,
                x_end: end,
                direction,
                visual_index: 0,
            });
        }
    }
    Ok(Fragment {
        glyphs,
        grapheme_geometry: geometry,
        width,
    })
}

fn rustybuzz_script(script: Script) -> rustybuzz::Script {
    let tag_bytes = script.as_iso15924_tag().to_be_bytes();
    let tag = rustybuzz::ttf_parser::Tag::from_bytes(&tag_bytes);
    rustybuzz::Script::from_iso15924_tag(tag).unwrap_or(rustybuzz::script::UNKNOWN)
}

fn missing_fragment(
    font_stack: &mut NativeFontStack,
    range: Range<usize>,
    direction: BidiDirection,
    grapheme_boundaries: &[Utf8ByteOffset],
    font_size: f32,
) -> (Fragment, bool) {
    let grapheme_index = grapheme_index_for_byte(grapheme_boundaries, range.start).unwrap_or(0);
    let (face_index, glyph_id, width) = font_stack
        .fallback_glyph()
        .map(|glyph| {
            (
                Some(glyph.face_index),
                glyph.glyph_id,
                font_stack.glyph_advance(glyph, font_size),
            )
        })
        .unwrap_or((None, 0, font_size * COMPATIBILITY_REPLACEMENT_WIDTH_EM));
    let used_replacement = face_index.is_some();
    let glyphs = face_index
        .map(|face_index| {
            vec![GlyphPlacement {
                face_index,
                glyph_id,
                cluster: ShapeClusterRange {
                    start: Utf8ByteOffset(range.start),
                    end: Utf8ByteOffset(range.end),
                },
                x: 0.0,
                y_offset: 0.0,
                x_offset: 0.0,
                advance: width,
                run_index: 0,
            }]
        })
        .unwrap_or_default();
    let (x_start, x_end) = if direction == BidiDirection::Rtl {
        (width, 0.0)
    } else {
        (0.0, width)
    };
    (
        Fragment {
            glyphs,
            grapheme_geometry: vec![GraphemeGeometry {
                range: ShapeClusterRange {
                    start: Utf8ByteOffset(range.start),
                    end: Utf8ByteOffset(range.end),
                },
                grapheme_index,
                x_start,
                x_end,
                direction,
                visual_index: 0,
            }],
            width,
        },
        used_replacement,
    )
}

fn line_break_records(
    text: &str,
    grapheme_boundaries: &[Utf8ByteOffset],
) -> Result<Vec<LineBreakRecord>, ()> {
    let mut records: Vec<LineBreakRecord> = Vec::new();
    for (byte, opportunity) in linebreaks(text) {
        let grapheme = grapheme_index_for_byte(grapheme_boundaries, byte).ok_or(())?;
        let kind = match opportunity {
            BreakOpportunity::Mandatory => LineBreakKind::Mandatory,
            BreakOpportunity::Allowed => LineBreakKind::Allowed,
        };
        if let Some(previous) = records.last_mut()
            && previous.byte.0 == byte
        {
            if kind == LineBreakKind::Mandatory {
                previous.kind = kind;
            }
            continue;
        }
        records.push(LineBreakRecord {
            grapheme: GraphemeBoundary(grapheme),
            byte: Utf8ByteOffset(byte),
            kind,
        });
    }
    if records
        .last()
        .is_none_or(|record| record.byte.0 != text.len())
    {
        records.push(LineBreakRecord {
            grapheme: GraphemeBoundary(grapheme_boundaries.len().saturating_sub(1)),
            byte: Utf8ByteOffset(text.len()),
            kind: LineBreakKind::Mandatory,
        });
    }
    Ok(records)
}

fn line_break_records_lossy(
    text: &str,
    grapheme_boundaries: &[Utf8ByteOffset],
) -> Vec<LineBreakRecord> {
    line_break_records(text, grapheme_boundaries)
        .unwrap_or_else(|_| hard_break_records(text, grapheme_boundaries))
}

fn hard_break_records(text: &str, grapheme_boundaries: &[Utf8ByteOffset]) -> Vec<LineBreakRecord> {
    let mut records = Vec::new();
    let mut characters = text.char_indices().peekable();
    while let Some((byte, character)) = characters.next() {
        let end = match character {
            '\r' => {
                if characters.peek().is_some_and(|(_, next)| *next == '\n') {
                    match characters.next() {
                        Some((_, next)) => byte + 1 + next.len_utf8(),
                        None => byte + character.len_utf8(),
                    }
                } else {
                    byte + character.len_utf8()
                }
            }
            '\n' => byte + character.len_utf8(),
            _ => continue,
        };
        if let Some(grapheme) = grapheme_index_for_byte(grapheme_boundaries, end) {
            records.push(LineBreakRecord {
                grapheme: GraphemeBoundary(grapheme),
                byte: Utf8ByteOffset(end),
                kind: LineBreakKind::Mandatory,
            });
        }
    }
    if records
        .last()
        .is_none_or(|record| record.byte.0 != text.len())
    {
        records.push(LineBreakRecord {
            grapheme: GraphemeBoundary(grapheme_boundaries.len().saturating_sub(1)),
            byte: Utf8ByteOffset(text.len()),
            kind: LineBreakKind::Mandatory,
        });
    }
    records
}

fn scalar_boundaries(text: &str) -> Vec<Utf8ByteOffset> {
    let mut boundaries = text
        .char_indices()
        .map(|(byte, _)| Utf8ByteOffset(byte))
        .collect::<Vec<_>>();
    boundaries.push(Utf8ByteOffset(text.len()));
    boundaries
}

fn grapheme_boundaries(text: &str) -> Vec<Utf8ByteOffset> {
    let mut boundaries = text
        .grapheme_indices(true)
        .map(|(byte, _)| Utf8ByteOffset(byte))
        .collect::<Vec<_>>();
    boundaries.push(Utf8ByteOffset(text.len()));
    boundaries
}

fn grapheme_ranges(boundaries: &[Utf8ByteOffset], range: Range<usize>) -> Vec<Range<usize>> {
    boundaries
        .windows(2)
        .filter_map(|window| {
            let start = window[0].0;
            let end = window[1].0;
            (start >= range.start && end <= range.end && start < end).then_some(start..end)
        })
        .collect()
}

fn grapheme_index_for_byte(boundaries: &[Utf8ByteOffset], byte: usize) -> Option<usize> {
    boundaries.binary_search(&Utf8ByteOffset(byte)).ok()
}

fn is_grapheme_boundary(boundaries: &[Utf8ByteOffset], byte: usize) -> bool {
    grapheme_index_for_byte(boundaries, byte).is_some()
}

fn first_line_range(text: &str) -> Range<usize> {
    let end = text
        .char_indices()
        .find_map(|(byte, character)| matches!(character, '\r' | '\n').then_some(byte))
        .unwrap_or(text.len());
    0..end
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gui_runtime::native_vello::text_renderer::font::NativeFontStack;

    #[test]
    fn shaped_fixture_uses_rustybuzz_clusters_and_ordered_faces() {
        let mut stack = NativeFontStack::from_test_bytes(&[
            include_bytes!("../../../../tests/fixtures/fonts/primary.ttf"),
            include_bytes!("../../../../tests/fixtures/fonts/secondary.ttf"),
        ]);
        let layout = compute_layout(&mut stack, "AΩ", 20.0).expect("valid size");

        assert_eq!(layout.fallback_glyphs, 0);
        assert_eq!(layout.missing_glyphs, 0);
        assert_eq!(
            layout
                .glyphs
                .iter()
                .map(|glyph| glyph.face_index)
                .collect::<Vec<_>>(),
            vec![0, 1]
        );
        assert_eq!(layout.snapshot.grapheme_boundaries.len(), 3);
        assert!(layout.glyphs[1].x > layout.glyphs[0].x);
        assert!(layout.snapshot.glyphs[1].advance > layout.snapshot.glyphs[0].advance);
    }

    #[test]
    fn mixed_scripts_are_itemized_at_grapheme_boundaries_with_context() {
        let mut stack = NativeFontStack::from_test_bytes(&[
            include_bytes!("../../../../tests/fixtures/fonts/primary.ttf"),
            include_bytes!("../../../../tests/fixtures/fonts/secondary.ttf"),
        ]);
        let source = "\u{0301}A.\u{0915}\u{093f}";
        let boundaries = grapheme_boundaries(source);
        let segments = font_segments(&mut stack, source, 0..source.len(), &boundaries)
            .expect("valid grapheme ranges");

        assert_eq!(
            segments
                .iter()
                .map(|segment| (source[segment.range.clone()].to_owned(), segment.script))
                .collect::<Vec<_>>(),
            vec![
                ("\u{0301}".to_owned(), Script::Latin),
                ("A".to_owned(), Script::Latin),
                (".".to_owned(), Script::Latin),
                ("\u{0915}\u{093f}".to_owned(), Script::Devanagari),
            ]
        );
        assert!(segments.iter().all(|segment| {
            is_grapheme_boundary(&boundaries, segment.range.start)
                && is_grapheme_boundary(&boundaries, segment.range.end)
        }));
    }

    #[test]
    fn isolated_fixture_scripts_shape_with_explicit_rustybuzz_script() {
        let stack = NativeFontStack::from_test_bytes(&[
            include_bytes!("../../../../tests/fixtures/fonts/primary.ttf"),
            include_bytes!("../../../../tests/fixtures/fonts/secondary.ttf"),
        ]);
        assert_eq!(
            rustybuzz_script(Script::Devanagari),
            rustybuzz::script::DEVANAGARI
        );

        let latin_source = "A";
        let latin_segment = FontSegment {
            range: 0..latin_source.len(),
            face_index: Some(0),
            special: false,
            script: Script::Latin,
        };
        let latin = shape_face_fragment(
            &stack,
            latin_source,
            &latin_segment,
            BidiDirection::Ltr,
            &grapheme_boundaries(latin_source),
            20.0,
        )
        .expect("isolated Latin fragment shapes");
        assert_eq!(latin.glyphs.len(), 1);
        assert_eq!(latin.glyphs[0].cluster.end, Utf8ByteOffset(1));

        let greek_source = "Ω";
        let greek_segment = FontSegment {
            range: 0..greek_source.len(),
            face_index: Some(1),
            special: false,
            script: Script::Greek,
        };
        let greek = shape_face_fragment(
            &stack,
            greek_source,
            &greek_segment,
            BidiDirection::Ltr,
            &grapheme_boundaries(greek_source),
            20.0,
        )
        .expect("isolated Greek fragment shapes");
        assert_eq!(greek.glyphs.len(), 1);
        assert_eq!(greek.glyphs[0].cluster.end, Utf8ByteOffset(2));
    }

    #[test]
    fn combining_grapheme_is_one_caret_cell_and_one_replacement() {
        let mut stack = NativeFontStack::from_test_bytes(&[
            include_bytes!("../../../../tests/fixtures/fonts/primary.ttf"),
            include_bytes!("../../../../tests/fixtures/fonts/secondary.ttf"),
        ]);
        let layout = compute_layout(&mut stack, "e\u{301}", 20.0).expect("valid size");
        assert_eq!(layout.fallback_glyphs, 1);
        assert_eq!(
            layout.snapshot.grapheme_boundaries,
            vec![Utf8ByteOffset(0), Utf8ByteOffset(3)]
        );
        assert_eq!(layout.snapshot.caret_geometry.len(), 2);
    }

    #[test]
    fn forced_compatibility_keeps_two_face_glyphs_in_their_selected_runs() {
        let mut stack = NativeFontStack::from_test_bytes(&[
            include_bytes!("../../../../tests/fixtures/fonts/primary.ttf"),
            include_bytes!("../../../../tests/fixtures/fonts/secondary.ttf"),
        ]);
        let paragraph = compute_compatibility_paragraph(&mut stack, Arc::from("AΩ"), 20.0);

        assert_eq!(paragraph.quality.fallback_glyphs, 0);
        assert_eq!(
            paragraph
                .glyphs
                .iter()
                .map(|glyph| glyph.face_index)
                .collect::<Vec<_>>(),
            vec![0, 1]
        );
        assert!(paragraph.glyphs.iter().all(|glyph| {
            paragraph.resolved_font_runs.iter().any(|run| {
                run.face_index == Some(glyph.face_index)
                    && run.range.start <= glyph.cluster.start.0
                    && glyph.cluster.end.0 <= run.range.end
            })
        }));
    }

    #[test]
    fn no_font_keeps_deterministic_geometry_without_drawable_glyphs() {
        let mut stack = NativeFontStack::from_test_bytes(&[]);
        let layout = compute_layout(&mut stack, "Ж", 20.0).expect("valid size");
        assert!(layout.glyphs.is_empty());
        assert_eq!(layout.missing_glyphs, 1);
        assert_eq!(layout.width, 10.0);
        assert!(
            layout
                .snapshot
                .caret_x(0, super::super::CaretAffinity::Downstream)
                < layout.width
        );
    }

    #[test]
    fn mixed_direction_keeps_logical_boundaries_and_visual_geometry() {
        let mut stack = NativeFontStack::from_test_bytes(&[
            include_bytes!("../../../../tests/fixtures/fonts/primary.ttf"),
            include_bytes!("../../../../tests/fixtures/fonts/secondary.ttf"),
        ]);
        let layout = compute_layout(&mut stack, "שלום world", 20.0).expect("valid size");
        assert!(
            layout
                .snapshot
                .bidi_runs
                .iter()
                .any(|run| run.direction == BidiDirection::Rtl)
        );
        assert_eq!(layout.snapshot.caret_geometry.len(), 11);
        let rtl_boundary = layout.snapshot.caret_geometry[1];
        let upstream_x = rtl_boundary.upstream_x.expect("RTL upstream caret");
        let downstream_x = rtl_boundary.downstream_x.expect("RTL downstream caret");
        assert_ne!(upstream_x, downstream_x);
        assert_eq!(
            layout.snapshot.hit_test(upstream_x),
            (1, super::super::CaretAffinity::Upstream)
        );
        assert_eq!(
            layout.snapshot.hit_test(downstream_x),
            (1, super::super::CaretAffinity::Downstream)
        );
    }

    #[test]
    fn line_break_records_keep_crlf_and_terminal_boundaries() {
        let bounds = grapheme_boundaries("a\r\nb");
        let records = line_break_records("a\r\nb", &bounds).expect("provider boundaries");
        assert_eq!(records.last().map(|record| record.byte.0), Some(4));
        assert!(
            records
                .iter()
                .all(|record| is_grapheme_boundary(&bounds, record.byte.0))
        );
    }

    #[test]
    fn hard_break_layout_is_single_line_but_keeps_complete_boundary_metadata() {
        let mut stack = NativeFontStack::from_test_bytes(&[include_bytes!(
            "../../../../tests/fixtures/fonts/primary.ttf"
        )]);
        let layout = compute_layout(&mut stack, "a\r\nb", 20.0).expect("valid size");

        assert_eq!(
            layout.snapshot.grapheme_boundaries.last(),
            Some(&Utf8ByteOffset(4))
        );
        assert_eq!(layout.snapshot.grapheme_geometry.len(), 1);
        assert_eq!(
            layout.snapshot.breaks.last().map(|record| record.byte.0),
            Some(4)
        );
        assert!(layout.snapshot.caret_geometry.iter().all(|caret| {
            caret.upstream_x.is_some_and(f32::is_finite)
                && caret.downstream_x.is_some_and(f32::is_finite)
        }));
    }
}
