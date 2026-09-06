use super::super::super::SurfaceRuntime;
use crate::{
    gui::text_layout::{TextWidthEstimate, estimated_text_width_in_range},
    gui::types::{Point, Rect},
    layout::Vector2,
    runtime::{PaintPrimitive, RuntimeBridge},
    theme::ThemeTokens,
    widgets::{DeclaredTextMetrics, TextScaleParticipation, WidgetId, WidgetSizing},
};
use std::time::Duration;

const TOOLTIP_OVERLAY_ID: WidgetId = u64::MAX - 2_048;
const TOOLTIP_HOVER_DELAY: Duration = Duration::from_millis(500);
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
    pub(in crate::runtime::controller) fn reset_tooltip_hover_intent(&mut self) {
        let revealed = self.interaction.tooltip.revealed;
        self.interaction.tooltip = Default::default();
        if revealed {
            self.repaint_requested = true;
        }
    }

    pub(in crate::runtime::controller) fn arm_tooltip_hover_intent(
        &mut self,
        widget_id: Option<WidgetId>,
    ) {
        self.reset_tooltip_hover_intent();
        let Some(widget_id) = widget_id else {
            return;
        };
        if (self.gesture_owns_pointer_capture() || self.interaction.pointer.capture.is_some())
            || self
                .surface_widget(widget_id)
                .and_then(|widget| widget.tooltip())
                .is_none_or(|tooltip| tooltip.is_empty())
        {
            return;
        }
        self.interaction.tooltip.target = Some(widget_id);
        self.interaction.tooltip.deadline =
            self.timed_repaint_now().checked_add(TOOLTIP_HOVER_DELAY);
    }

    pub(in crate::runtime::controller) fn rearm_tooltip_hover_intent(&mut self) {
        let hover = self.interaction.hover.widget;
        self.arm_tooltip_hover_intent(hover);
    }

    pub(super) fn append_widget_tooltip_overlay(
        &self,
        theme: &ThemeTokens,
        environment: &crate::runtime::ResolvedEnvironment,
        primitives: &mut Vec<PaintPrimitive>,
    ) {
        let Some(widget_id) = self
            .interaction
            .tooltip
            .target
            .filter(|_| self.interaction.tooltip.revealed)
            .filter(|widget_id| self.interaction.hover.widget == Some(*widget_id))
        else {
            return;
        };
        if self.gesture_owns_pointer_capture() || self.interaction.pointer.capture.is_some() {
            return;
        }
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
            environment,
        );
        crate::runtime::paint::push_tooltip_panel_with_environment(
            primitives,
            TOOLTIP_OVERLAY_ID,
            layout.rect,
            &layout.lines,
            theme,
            TOOLTIP_FONT_SIZE,
            TOOLTIP_LINE_HEIGHT,
            environment,
        );
    }
}

struct TooltipLayout {
    rect: Rect,
    lines: Vec<String>,
}

fn tooltip_layout(
    anchor: Rect,
    tooltip: &str,
    viewport: Vector2,
    environment: &crate::runtime::ResolvedEnvironment,
) -> TooltipLayout {
    let metrics = tooltip_metrics(environment);
    let max_width = (viewport.x - TOOLTIP_MARGIN * 2.0).clamp(1.0, TOOLTIP_MAX_WIDTH);
    let max_line_chars = tooltip_max_line_chars(max_width, metrics.font_size, metrics.insets.x);
    let lines = tooltip_lines(tooltip, max_line_chars);
    let rect = tooltip_rect_for_lines(anchor, &lines, max_width, viewport, metrics);
    TooltipLayout { rect, lines }
}

fn tooltip_metrics(
    environment: &crate::runtime::ResolvedEnvironment,
) -> crate::widgets::ResolvedTextMetrics {
    DeclaredTextMetrics::new(
        WidgetSizing::fixed(Vector2::new(0.0, 0.0)),
        TOOLTIP_FONT_SIZE,
        Vector2::new(TOOLTIP_HORIZONTAL_PADDING, TOOLTIP_VERTICAL_PADDING),
    )
    .resolve(environment, TextScaleParticipation::Scaled)
}

fn tooltip_rect_for_lines(
    anchor: Rect,
    lines: &[String],
    max_width: f32,
    viewport: Vector2,
    metrics: crate::widgets::ResolvedTextMetrics,
) -> Rect {
    let width = tooltip_width_for_lines(lines, metrics).min(max_width);
    let height = tooltip_height(lines.len(), metrics);
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

fn tooltip_width_for_lines(lines: &[String], metrics: crate::widgets::ResolvedTextMetrics) -> f32 {
    lines
        .iter()
        .map(|line| {
            estimated_text_width_in_range(
                line,
                tooltip_width_estimate(metrics),
                TOOLTIP_MIN_WIDTH,
                TOOLTIP_MAX_WIDTH,
            )
        })
        .fold(TOOLTIP_MIN_WIDTH, f32::max)
}

fn tooltip_height(line_count: usize, metrics: crate::widgets::ResolvedTextMetrics) -> f32 {
    let scale = metrics.font_size / TOOLTIP_FONT_SIZE;
    line_count.max(1) as f32 * TOOLTIP_LINE_HEIGHT * scale + metrics.insets.y
}

fn tooltip_max_line_chars(max_width: f32, font_size: f32, horizontal_padding: f32) -> usize {
    ((max_width - horizontal_padding).max(1.0) / tooltip_rendered_character_advance(font_size))
        .floor()
        .max(12.0) as usize
}

fn tooltip_width_estimate(metrics: crate::widgets::ResolvedTextMetrics) -> TextWidthEstimate {
    TextWidthEstimate::new(
        tooltip_character_advance(metrics.font_size),
        metrics.insets.x,
    )
}

fn tooltip_character_advance(font_size: f32) -> f32 {
    tooltip_rendered_character_advance(font_size).ceil() + TOOLTIP_CHAR_ADVANCE_SAFETY
}

fn tooltip_rendered_character_advance(font_size: f32) -> f32 {
    let scale = (font_size / TOOLTIP_BITMAP_GLYPH_HEIGHT).clamp(1.0, 3.0);
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
