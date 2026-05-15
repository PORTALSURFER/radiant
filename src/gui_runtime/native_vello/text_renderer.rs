//! Native text rendering for Vello scenes.

use super::NativeTextOptions;
use crate::{
    gui::paint::{TextAlign, TextRun},
    runtime::PaintText,
};
use std::sync::Arc;
use vello::{
    Glyph, Scene,
    peniko::{Fill, FontData},
};

mod cache;
mod encoding;
mod font;
mod layout;

use cache::TextLayoutCache;
#[cfg(test)]
use cache::TextLayoutProfileCounters;
pub(super) use encoding::{color_from_rgba, icon_from_rgba, to_kurbo_rect};

#[derive(Clone, Debug)]
pub(super) struct SceneTextRun {
    pub(super) text: PaintText,
    pub(super) position: crate::gui::types::Point,
    pub(super) font_size: f32,
    pub(super) color: crate::gui::types::Rgba8,
    pub(super) max_width: Option<f32>,
    pub(super) align: TextAlign,
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
        }
    }
}

#[derive(Clone, Debug)]
pub(super) struct GlyphLayout {
    id: u32,
    x: f32,
}

#[derive(Clone, Debug)]
pub(super) struct TextLayout {
    pub(super) width: f32,
    pub(super) glyphs: Vec<GlyphLayout>,
    pub(super) cursor_stops: Vec<TextCursorStop>,
}

#[derive(Clone, Debug, PartialEq)]
pub(super) struct TextCursorStop {
    pub(super) byte_index: usize,
    pub(super) x: f32,
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub(super) struct TextLayoutKey {
    text: Arc<str>,
    font_size_bits: u32,
}

#[derive(Clone)]
pub(super) struct LoadedFont {
    font: FontData,
}

pub(super) struct NativeTextRenderer {
    loaded_font: Option<LoadedFont>,
    layout_cache: TextLayoutCache,
}

impl NativeTextRenderer {
    #[cfg(test)]
    pub(super) fn new() -> Self {
        Self::with_options(&NativeTextOptions::default())
    }

    pub(super) fn with_options(options: &NativeTextOptions) -> Self {
        let loaded_font = font::load_native_font(options).map(|font| LoadedFont { font });
        if loaded_font.is_none() {
            tracing::warn!(
                "Native vello text renderer found no fallback font; text runs will be skipped"
            );
        }
        Self {
            loaded_font,
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
        let Some(loaded_font) = self.loaded_font.as_ref() else {
            return;
        };
        let font_data = &loaded_font.font;
        for run in text_runs {
            if !text_run_is_renderable(&run) {
                continue;
            }
            let Some(layout) =
                self.layout_cache
                    .layout_for(font_data, run.text.as_ref(), run.font_size)
            else {
                continue;
            };
            let mut origin_x = run.position.x;
            if let Some(max_width) = run.max_width {
                let extra = (max_width - layout.width).max(0.0);
                origin_x += match run.align {
                    TextAlign::Left => 0.0,
                    TextAlign::Center => extra * 0.5,
                    TextAlign::Right => extra,
                };
            }
            let clip_width = run.max_width.unwrap_or(f32::INFINITY);
            let baseline = run.position.y + run.font_size;
            let glyph_iter = layout
                .glyphs
                .iter()
                .take_while(|glyph| glyph.x <= clip_width)
                .map(|glyph| Glyph {
                    id: glyph.id,
                    x: origin_x + glyph.x,
                    y: baseline,
                });
            scene
                .draw_glyphs(font_data)
                .font_size(run.font_size)
                .brush(color_from_rgba(run.color))
                .draw(Fill::NonZero, glyph_iter);
        }
    }

    pub(super) fn layout_text(&mut self, text: &str, font_size: f32) -> Option<&TextLayout> {
        if !font_size_is_renderable(font_size) {
            return None;
        }
        let font = &self.loaded_font.as_ref()?.font;
        self.layout_cache.layout_for(font, text, font_size)
    }

    #[cfg(test)]
    pub(super) fn take_layout_profile_counters(&mut self) -> TextLayoutProfileCounters {
        self.layout_cache.take_profile_counters()
    }
}

fn text_run_is_renderable(run: &SceneTextRun) -> bool {
    !run.text.is_empty()
        && font_size_is_renderable(run.font_size)
        && run.position.x.is_finite()
        && run.position.y.is_finite()
        && run
            .max_width
            .is_none_or(|max_width| max_width.is_finite() && max_width > 0.0)
}

pub(in crate::gui_runtime::native_vello) fn font_size_is_renderable(font_size: f32) -> bool {
    font_size.is_finite() && font_size > 0.0
}

impl TextLayout {
    pub(super) fn empty_for(text: &str) -> Self {
        Self {
            width: 0.0,
            glyphs: Vec::new(),
            cursor_stops: vec![TextCursorStop {
                byte_index: text.len(),
                x: 0.0,
            }],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn text_run_renderability_rejects_non_finite_geometry() {
        let mut run = SceneTextRun {
            text: "tempo".into(),
            position: crate::gui::types::Point::new(4.0, 8.0),
            font_size: 12.0,
            color: crate::gui::types::Rgba8 {
                r: 255,
                g: 255,
                b: 255,
                a: 255,
            },
            max_width: Some(80.0),
            align: TextAlign::Left,
        };

        assert!(text_run_is_renderable(&run));

        run.font_size = f32::NAN;
        assert!(!text_run_is_renderable(&run));

        run.font_size = 12.0;
        run.position.x = f32::INFINITY;
        assert!(!text_run_is_renderable(&run));

        run.position.x = 4.0;
        run.max_width = Some(0.0);
        assert!(!text_run_is_renderable(&run));

        run.max_width = Some(f32::NAN);
        assert!(!text_run_is_renderable(&run));
    }
}
