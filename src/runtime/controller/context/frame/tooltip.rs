use super::super::super::SurfaceRuntime;
use crate::{
    gui::text_layout::{TextWidthEstimate, estimated_text_width_in_range},
    gui::types::{Point, Rect},
    layout::Vector2,
    runtime::{PaintPrimitive, RuntimeBridge},
    theme::ThemeTokens,
    widgets::WidgetId,
};

const TOOLTIP_OVERLAY_ID: WidgetId = u64::MAX - 2_048;
const TOOLTIP_MARGIN: f32 = 6.0;
const TOOLTIP_GAP: f32 = 8.0;
const TOOLTIP_MIN_WIDTH: f32 = 140.0;
const TOOLTIP_MAX_WIDTH: f32 = 360.0;
const TOOLTIP_FONT_SIZE: f32 = 9.0;
const TOOLTIP_LINE_HEIGHT: f32 = 13.0;
const TOOLTIP_BITMAP_GLYPH_HEIGHT: f32 = 7.0;
const TOOLTIP_BITMAP_GLYPH_ADVANCE: f32 = 6.0;
const TOOLTIP_CHAR_ADVANCE_SAFETY: f32 = 1.0;
const TOOLTIP_HORIZONTAL_PADDING: f32 = 16.0;
const TOOLTIP_VERTICAL_PADDING: f32 = 8.0;

impl<Bridge, Message> SurfaceRuntime<Bridge, Message>
where
    Bridge: RuntimeBridge<Message>,
{
    pub(super) fn append_widget_tooltip_overlay(
        &self,
        theme: &ThemeTokens,
        primitives: &mut Vec<PaintPrimitive>,
    ) {
        if self.interaction.pointer.capture.is_some() {
            return;
        }
        let Some(widget_id) = self.interaction.hover.widget else {
            return;
        };
        let Some(tooltip) = self
            .surface_widget(widget_id)
            .and_then(|widget| widget.tooltip())
            .filter(|tooltip| !tooltip.is_empty())
        else {
            return;
        };
        let Some(anchor) = self.layout.rects.get(&widget_id).copied() else {
            return;
        };
        let layout = tooltip_layout(
            anchor,
            tooltip,
            Vector2::new(self.viewport.width(), self.viewport.height()),
        );
        crate::runtime::paint::push_tooltip_panel(
            primitives,
            TOOLTIP_OVERLAY_ID,
            layout.rect,
            &layout.lines,
            theme,
            TOOLTIP_FONT_SIZE,
            TOOLTIP_LINE_HEIGHT,
        );
    }
}

struct TooltipLayout {
    rect: Rect,
    lines: Vec<String>,
}

fn tooltip_layout(anchor: Rect, tooltip: &str, viewport: Vector2) -> TooltipLayout {
    let max_width = (viewport.x - TOOLTIP_MARGIN * 2.0).clamp(1.0, TOOLTIP_MAX_WIDTH);
    let max_line_chars = tooltip_max_line_chars(max_width);
    let lines = tooltip_lines(tooltip, max_line_chars);
    let rect = tooltip_rect_for_lines(anchor, &lines, max_width, viewport);
    TooltipLayout { rect, lines }
}

fn tooltip_rect_for_lines(
    anchor: Rect,
    lines: &[String],
    max_width: f32,
    viewport: Vector2,
) -> Rect {
    let width = tooltip_width_for_lines(lines).min(max_width);
    let height = tooltip_height(lines.len());
    let x = anchor.min.x.clamp(
        TOOLTIP_MARGIN,
        (viewport.x - width - TOOLTIP_MARGIN).max(TOOLTIP_MARGIN),
    );
    let below_y = anchor.max.y + TOOLTIP_GAP;
    let y = if below_y + height <= viewport.y - TOOLTIP_MARGIN {
        below_y
    } else {
        (anchor.min.y - TOOLTIP_GAP - height).max(TOOLTIP_MARGIN)
    };
    Rect::from_min_size(Point::new(x, y), Vector2::new(width, height))
}

fn tooltip_width_for_lines(lines: &[String]) -> f32 {
    lines
        .iter()
        .map(|line| {
            estimated_text_width_in_range(
                line,
                tooltip_width_estimate(),
                TOOLTIP_MIN_WIDTH,
                TOOLTIP_MAX_WIDTH,
            )
        })
        .fold(TOOLTIP_MIN_WIDTH, f32::max)
}

fn tooltip_height(line_count: usize) -> f32 {
    line_count.max(1) as f32 * TOOLTIP_LINE_HEIGHT + TOOLTIP_VERTICAL_PADDING
}

fn tooltip_max_line_chars(max_width: f32) -> usize {
    ((max_width - TOOLTIP_HORIZONTAL_PADDING).max(1.0) / tooltip_rendered_character_advance())
        .floor()
        .max(12.0) as usize
}

fn tooltip_width_estimate() -> TextWidthEstimate {
    TextWidthEstimate::new(tooltip_character_advance(), TOOLTIP_HORIZONTAL_PADDING)
}

fn tooltip_character_advance() -> f32 {
    tooltip_rendered_character_advance().ceil() + TOOLTIP_CHAR_ADVANCE_SAFETY
}

fn tooltip_rendered_character_advance() -> f32 {
    let scale = (TOOLTIP_FONT_SIZE / TOOLTIP_BITMAP_GLYPH_HEIGHT).clamp(1.0, 3.0);
    TOOLTIP_BITMAP_GLYPH_ADVANCE * scale
}

fn tooltip_lines(tooltip: &str, max_line_chars: usize) -> Vec<String> {
    let mut lines = Vec::new();
    for paragraph in tooltip.lines() {
        push_wrapped_tooltip_paragraph(&mut lines, paragraph.trim(), max_line_chars);
    }
    if lines.is_empty() {
        lines.push(String::new());
    }
    lines
}

fn push_wrapped_tooltip_paragraph(lines: &mut Vec<String>, paragraph: &str, max_chars: usize) {
    if paragraph.is_empty() {
        return;
    }
    let mut current = String::new();
    for word in paragraph.split_whitespace() {
        if current.is_empty() {
            push_tooltip_word(lines, &mut current, word, max_chars);
            continue;
        }
        let next_len = current.chars().count() + 1 + word.chars().count();
        if next_len <= max_chars {
            current.push(' ');
            current.push_str(word);
        } else {
            lines.push(std::mem::take(&mut current));
            push_tooltip_word(lines, &mut current, word, max_chars);
        }
    }
    if !current.is_empty() {
        lines.push(current);
    }
}

fn push_tooltip_word(lines: &mut Vec<String>, current: &mut String, word: &str, max_chars: usize) {
    if word.chars().count() <= max_chars {
        current.push_str(word);
        return;
    }
    let mut chunk = String::new();
    for ch in word.chars() {
        if chunk.chars().count() == max_chars {
            lines.push(std::mem::take(&mut chunk));
        }
        chunk.push(ch);
    }
    *current = chunk;
}

#[cfg(test)]
mod tests;
