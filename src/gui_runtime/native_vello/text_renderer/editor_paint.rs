//! Native replay of the shared editor paragraph geometry.

use super::{NativeEditorParagraph, NativeTextRenderer, color_from_rgba};
use crate::{
    gui::{
        text_layout::{
            editor::resolve_editor_scroll,
            paragraph::{CaretAffinity, ParagraphCaret},
        },
        types::{Point, Rect},
    },
    runtime::PaintTextEditor,
};
use vello::glyph::Glyph;
use vello::{Scene, kurbo::Affine, peniko::Fill};

impl NativeTextRenderer {
    /// Replay the prepared native glyph payload at placements supplied by shared geometry.
    pub(in crate::gui_runtime::native_vello) fn encode_editor(
        &mut self,
        scene: &mut Scene,
        input: &PaintTextEditor,
    ) -> bool {
        let Some(paragraph) = self.editor_paragraph_for_request(&input.request) else {
            return false;
        };
        let geometry = paragraph.geometry();
        let scroll = resolve_editor_scroll(
            geometry,
            &input.request,
            input.selection,
            input.scroll,
            input.focused,
        );
        let clip = input.request.rect;
        scene.push_clip_layer(Fill::NonZero, Affine::IDENTITY, &super::to_kurbo_rect(clip));
        for (line_index, line) in geometry.lines().iter().enumerate() {
            let y = clip.min.y + line.y - scroll.y;
            if y + geometry.line_height() < clip.min.y || y > clip.max.y {
                continue;
            }
            for placement in geometry.placements(line_index).unwrap_or_default() {
                let Some(payload) = paragraph.payload_for(placement.source_cluster) else {
                    continue;
                };
                let Some(face) = self.font_stack.face(
                    payload
                        .glyphs()
                        .first()
                        .map_or(usize::MAX, |glyph| glyph.face_index),
                ) else {
                    continue;
                };
                let glyphs: Vec<Glyph> = payload
                    .glyphs()
                    .iter()
                    .map(|glyph| Glyph {
                        id: glyph.glyph_id,
                        x: clip.min.x + placement.rect.min.x + glyph.x + glyph.x_offset - scroll.x,
                        y: y + input.request.font_size + glyph.y_offset,
                    })
                    .collect();
                if !glyphs.is_empty() {
                    scene
                        .draw_glyphs(face)
                        .font_size(input.request.font_size)
                        .brush(color_from_rgba(input.color))
                        .draw(Fill::NonZero, glyphs);
                }
            }
        }
        scene.pop_layer();
        true
    }
}

/// Resolve the IME caret rectangle from the same shared editor geometry.
pub(in crate::gui_runtime::native_vello) fn editor_caret_rect(
    input: &PaintTextEditor,
    paragraph: &NativeEditorParagraph,
) -> Option<Rect> {
    let geometry = paragraph.geometry();
    let scroll = resolve_editor_scroll(
        geometry,
        &input.request,
        input.selection,
        input.scroll,
        input.focused,
    );
    let point = geometry.caret(ParagraphCaret {
        byte: input.selection.caret,
        affinity: CaretAffinity::Downstream,
    })?;
    let x = input.request.rect.min.x + point.x - scroll.x;
    let y = input.request.rect.min.y + point.y - scroll.y;
    Some(Rect::from_xy_size(x, y, 1.0, geometry.line_height()).clamp_to(input.request.rect))
}
