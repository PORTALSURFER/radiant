//! Native text rendering for Vello scenes.

use super::NativeTextOptions;
use crate::gui::{
    paint::{TextAlign, TextRun},
    types::{Point, Rgba8},
};
use vello::{Glyph, Scene, peniko::Fill};

mod cache;
mod encoding;
mod font;
mod layout;
mod model;
mod renderability;

use cache::TextLayoutCache;
pub(in crate::gui_runtime::native_vello) use cache::TextLayoutProfileCounters;
pub(super) use encoding::{color_from_rgba, icon_from_rgba, to_kurbo_rect};
use font::NativeFontStack;
pub(in crate::gui_runtime::native_vello) use model::{
    GlyphLayout, SceneTextRun, TextCursorStop, TextLayout, TextLayoutKey,
};
pub(in crate::gui_runtime::native_vello) use renderability::font_size_is_renderable;
use renderability::text_run_is_renderable;
use renderability::text_run_parts_are_renderable;

pub(super) struct NativeTextRenderer {
    font_stack: NativeFontStack,
    layout_cache: TextLayoutCache,
}

impl NativeTextRenderer {
    #[cfg(test)]
    pub(super) fn new() -> Self {
        Self::with_options(&NativeTextOptions::default())
    }

    pub(super) fn with_options(options: &NativeTextOptions) -> Self {
        let font_stack = NativeFontStack::with_options(options);
        if font_stack.is_empty() {
            tracing::warn!(
                "Native vello text renderer found no fallback font; text runs will be skipped"
            );
        }
        Self {
            font_stack,
            layout_cache: TextLayoutCache::new(),
        }
    }

    pub(super) fn draw_text_runs(&mut self, scene: &mut Scene, text_runs: &[TextRun]) {
        self.draw_scene_text_runs(scene, text_runs.iter().map(SceneTextRun::from));
    }

    pub(super) fn draw_scene_text_runs(
        &mut self,
        scene: &mut Scene,
        text_runs: impl IntoIterator<Item = SceneTextRun>,
    ) {
        if self.font_stack.is_empty() {
            return;
        }
        let layout_cache = &mut self.layout_cache;
        for run in text_runs {
            if !text_run_is_renderable(&run) {
                continue;
            }
            draw_text_run_with_font(
                scene,
                &mut self.font_stack,
                layout_cache,
                run.text.as_ref(),
                TextRunParts {
                    position: run.position,
                    font_size: run.font_size,
                    color: run.color,
                    max_width: run.max_width,
                    align: run.align,
                },
            );
        }
    }

    pub(super) fn draw_text_run(&mut self, scene: &mut Scene, text: &str, parts: TextRunParts) {
        if !text_run_parts_are_renderable(text, parts.position, parts.font_size, parts.max_width) {
            return;
        }
        if self.font_stack.is_empty() {
            return;
        }
        draw_text_run_with_font(
            scene,
            &mut self.font_stack,
            &mut self.layout_cache,
            text,
            parts,
        );
    }

    pub(super) fn layout_text(&mut self, text: &str, font_size: f32) -> Option<&TextLayout> {
        if !font_size_is_renderable(font_size) {
            return None;
        }
        self.layout_cache
            .layout_for(&mut self.font_stack, text, font_size)
    }

    pub(super) fn take_layout_profile_counters(&mut self) -> TextLayoutProfileCounters {
        self.layout_cache.take_profile_counters()
    }
}

#[derive(Clone, Copy, Debug)]
pub(super) struct TextRunParts {
    pub(super) position: Point,
    pub(super) font_size: f32,
    pub(super) color: Rgba8,
    pub(super) max_width: Option<f32>,
    pub(super) align: TextAlign,
}

fn draw_text_run_with_font(
    scene: &mut Scene,
    font_stack: &mut NativeFontStack,
    layout_cache: &mut TextLayoutCache,
    text: &str,
    parts: TextRunParts,
) {
    let Some(layout) = layout_cache.layout_for(font_stack, text, parts.font_size) else {
        return;
    };
    let mut origin_x = parts.position.x;
    if let Some(max_width) = parts.max_width {
        let extra = (max_width - layout.width).max(0.0);
        origin_x += match parts.align {
            TextAlign::Left => 0.0,
            TextAlign::Center => extra * 0.5,
            TextAlign::Right => extra,
        };
    }
    let clip_width = parts.max_width.unwrap_or(f32::INFINITY);
    let baseline = parts.position.y + parts.font_size;
    let mut start = 0;
    while start < layout.glyphs.len() {
        let Some((face_index, end)) = visible_face_segment(&layout.glyphs, start, clip_width)
        else {
            break;
        };
        let Some(font_data) = font_stack.face(face_index) else {
            break;
        };
        let glyph_iter = layout.glyphs[start..end].iter().map(|glyph| Glyph {
            id: glyph.id,
            x: origin_x + glyph.x,
            y: baseline,
        });
        scene
            .draw_glyphs(font_data)
            .font_size(parts.font_size)
            .brush(color_from_rgba(parts.color))
            .draw(Fill::NonZero, glyph_iter);
        start = end;
    }
}

fn visible_face_segment(
    glyphs: &[GlyphLayout],
    start: usize,
    clip_width: f32,
) -> Option<(usize, usize)> {
    let first = glyphs.get(start)?;
    if first.x > clip_width {
        return None;
    }
    let face_index = first.face_index;
    let mut end = start + 1;
    while end < glyphs.len() && glyphs[end].face_index == face_index && glyphs[end].x <= clip_width
    {
        end += 1;
    }
    Some((face_index, end))
}

#[cfg(test)]
mod tests {
    use super::GlyphLayout;
    use super::{TextCursorStop, TextLayout, visible_face_segment};

    #[test]
    fn empty_layout_preserves_terminal_cursor_stop() {
        let layout = TextLayout::empty_for("tempo");
        assert_eq!(layout.width, 0.0);
        assert!(layout.glyphs.is_empty());
        assert_eq!(
            layout.cursor_stops,
            vec![TextCursorStop {
                byte_index: 5,
                x: 0.0,
            }]
        );
    }

    #[test]
    fn visible_glyph_segments_follow_face_boundaries_and_clip_width() {
        let glyphs = vec![
            GlyphLayout {
                face_index: 0,
                id: 1,
                x: 0.0,
            },
            GlyphLayout {
                face_index: 1,
                id: 2,
                x: 5.0,
            },
            GlyphLayout {
                face_index: 0,
                id: 3,
                x: 10.0,
            },
        ];
        let mut start = 0;
        let mut segments = Vec::new();
        while let Some((face, end)) = visible_face_segment(&glyphs, start, 9.0) {
            segments.push((face, start..end));
            start = end;
        }

        assert_eq!(segments, vec![(0, 0..1), (1, 1..2)]);
    }
}
