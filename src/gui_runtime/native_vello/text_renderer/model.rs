use crate::{
    gui::{
        paint::{TextAlign, TextRun},
        types::{Point, Rgba8},
    },
    runtime::PaintText,
};
use std::sync::Arc;
use vello::peniko::FontData;

#[derive(Clone, Debug)]
pub(in crate::gui_runtime::native_vello) struct SceneTextRun {
    pub(in crate::gui_runtime::native_vello) text: PaintText,
    pub(in crate::gui_runtime::native_vello) position: Point,
    pub(in crate::gui_runtime::native_vello) font_size: f32,
    pub(in crate::gui_runtime::native_vello) color: Rgba8,
    pub(in crate::gui_runtime::native_vello) max_width: Option<f32>,
    pub(in crate::gui_runtime::native_vello) align: TextAlign,
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
pub(in crate::gui_runtime::native_vello) struct GlyphLayout {
    pub(in crate::gui_runtime::native_vello) id: u32,
    pub(in crate::gui_runtime::native_vello) x: f32,
}

#[derive(Clone, Debug)]
pub(in crate::gui_runtime::native_vello) struct TextLayout {
    pub(in crate::gui_runtime::native_vello) width: f32,
    pub(in crate::gui_runtime::native_vello) glyphs: Vec<GlyphLayout>,
    pub(in crate::gui_runtime::native_vello) cursor_stops: Vec<TextCursorStop>,
    pub(in crate::gui_runtime::native_vello) unsupported_shaping_runs: u64,
    pub(in crate::gui_runtime::native_vello) unsupported_shaping_scalars: u64,
    pub(in crate::gui_runtime::native_vello) fallback_glyphs: u64,
    pub(in crate::gui_runtime::native_vello) missing_glyphs: u64,
}

impl TextLayout {
    pub(in crate::gui_runtime::native_vello) fn empty_for(text: &str) -> Self {
        Self {
            width: 0.0,
            glyphs: Vec::new(),
            cursor_stops: vec![TextCursorStop {
                byte_index: text.len(),
                x: 0.0,
            }],
            unsupported_shaping_runs: 0,
            unsupported_shaping_scalars: 0,
            fallback_glyphs: 0,
            missing_glyphs: 0,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(in crate::gui_runtime::native_vello) struct TextCursorStop {
    pub(in crate::gui_runtime::native_vello) byte_index: usize,
    pub(in crate::gui_runtime::native_vello) x: f32,
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub(in crate::gui_runtime::native_vello) struct TextLayoutKey {
    pub(in crate::gui_runtime::native_vello) text: Arc<str>,
    pub(in crate::gui_runtime::native_vello) font_size_bits: u32,
}

#[derive(Clone)]
pub(in crate::gui_runtime::native_vello) struct LoadedFont {
    pub(in crate::gui_runtime::native_vello) font: FontData,
}
