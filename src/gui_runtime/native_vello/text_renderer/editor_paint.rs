//! Native replay of the shared editor paragraph geometry.
use super::{NativeEditorParagraph, NativeTextRenderer, color_from_rgba, to_kurbo_rect};
use crate::{
    gui::{
        text_layout::{editor::resolve_editor_scroll, paragraph::ParagraphCaret},
        types::Rect,
    },
    runtime::PaintTextEditor,
};
use vello::{Glyph, Scene, kurbo::Affine, peniko::Fill};
impl NativeTextRenderer {
    pub(in crate::gui_runtime::native_vello) fn encode_editor(
        &mut self,
        scene: &mut Scene,
        input: &PaintTextEditor,
    ) -> bool {
        let Some(paragraph) = self
            .retained_editor_paragraph(&input.request)
            .or_else(|| self.editor_paragraph_for_request(&input.request))
        else {
            return false;
        };
        let geometry = paragraph.geometry();
        let scroll = resolve_editor_scroll(
            geometry,
            &input.request,
            input.selection,
            input.scroll,
            input.reveal_caret,
        );
        let clip = input.request.rect;
        scene.push_clip_layer(Fill::NonZero, Affine::IDENTITY, &to_kurbo_rect(clip));
        let first = geometry
            .lines()
            .partition_point(|line| line.y + geometry.line_height() < scroll.y);
        let end = geometry
            .lines()
            .partition_point(|line| line.y <= scroll.y + clip.height());
        if input.focused && !input.hide_adornments {
            for rect in geometry.selection_rects_in_lines(input.selection.range(), first..end) {
                let rect = Rect::from_xy_size(
                    clip.min.x + rect.min.x - scroll.x,
                    clip.min.y + rect.min.y - scroll.y,
                    rect.width(),
                    rect.height(),
                );
                scene.fill(
                    Fill::NonZero,
                    Affine::IDENTITY,
                    color_from_rgba(input.selection_color),
                    None,
                    &to_kurbo_rect(rect),
                );
            }
        }
        let mut segment = Vec::new();
        let mut face_index = None;
        let flush = |scene: &mut Scene, face_index: Option<usize>, segment: &mut Vec<Glyph>| {
            if let Some(face) = face_index.and_then(|index| self.font_stack.face(index)) {
                scene
                    .draw_glyphs(face)
                    .font_size(input.request.font_size)
                    .brush(color_from_rgba(input.color))
                    .draw(Fill::NonZero, segment.drain(..));
            } else {
                segment.clear();
            }
        };
        for line_index in first..end {
            let line = &geometry.lines()[line_index];
            let y = clip.min.y + line.y - scroll.y;
            for placement in geometry.placements(line_index).unwrap_or_default() {
                let Some(payload) = paragraph.payload_for(placement.source_cluster) else {
                    continue;
                };
                for glyph in payload.glyphs() {
                    let glyph_x = placement.rect.min.x + glyph.x + glyph.x_offset;
                    let right = glyph_x + glyph.advance;
                    if glyph_x.min(right) > scroll.x + clip.width() || glyph_x.max(right) < scroll.x
                    {
                        continue;
                    }
                    if face_index != Some(glyph.face_index) {
                        flush(scene, face_index.take(), &mut segment);
                        face_index = Some(glyph.face_index);
                    }
                    segment.push(Glyph {
                        id: glyph.glyph_id,
                        x: clip.min.x + placement.rect.min.x + glyph.x + glyph.x_offset - scroll.x,
                        y: y + input.request.font_size + glyph.y_offset,
                    });
                    if segment.len() >= 4096 {
                        flush(scene, face_index, &mut segment);
                    }
                }
            }
        }
        flush(scene, face_index, &mut segment);
        if input.focused && !input.hide_adornments && input.selection.range().is_empty() {
            if let Some(rect) = editor_caret_rect(input, &paragraph) {
                scene.fill(
                    Fill::NonZero,
                    Affine::IDENTITY,
                    color_from_rgba(input.caret_color),
                    None,
                    &to_kurbo_rect(rect),
                );
            }
        }
        scene.pop_layer();
        true
    }
}
/// Shared caret and viewport projection used by native IME and paint.
pub(in crate::gui_runtime::native_vello) fn editor_caret_rect(
    input: &PaintTextEditor,
    paragraph: &NativeEditorParagraph,
) -> Option<Rect> {
    if !input.focused || input.hide_adornments {
        return None;
    }
    let geometry = paragraph.geometry();
    let scroll = resolve_editor_scroll(
        geometry,
        &input.request,
        input.selection,
        input.scroll,
        input.reveal_caret,
    );
    let point = geometry.caret(ParagraphCaret {
        byte: input.selection.caret,
        affinity: input.selection.affinity,
    })?;
    let rect = Rect::from_xy_size(
        input.request.rect.min.x + point.x - scroll.x,
        input.request.rect.min.y + point.y - scroll.y,
        1.0,
        geometry.line_height(),
    );
    let clipped = rect.clamp_to(input.request.rect);
    (clipped.is_finite() && clipped.height() > 0.0 && clipped.width() > 0.0).then_some(clipped)
}
