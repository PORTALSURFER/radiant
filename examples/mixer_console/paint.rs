use radiant::prelude::*;
use radiant::runtime::{PaintFillRect, PaintStrokeRect};

pub(crate) fn meter_color(db: f32) -> Rgba8 {
    if db > -3.0 {
        rgba(255, 82, 52, 255)
    } else if db > -10.0 {
        rgba(255, 190, 72, 255)
    } else {
        rgba(60, 214, 154, 255)
    }
}

pub(crate) fn group_color(group: usize, theme: &ThemeTokens) -> Rgba8 {
    match group % 4 {
        0 => theme.highlight_cyan,
        1 => theme.highlight_blue,
        2 => theme.accent_warning,
        _ => theme.highlight_orange,
    }
}

pub(crate) fn send_color(send: usize, theme: &ThemeTokens) -> Rgba8 {
    match send % 3 {
        0 => theme.highlight_cyan,
        1 => theme.highlight_blue,
        _ => theme.highlight_orange,
    }
}

pub(crate) fn blend_color(a: Rgba8, b: Rgba8, t: f32) -> Rgba8 {
    let t = t.clamp(0.0, 1.0);
    rgba(
        (a.r as f32 + (b.r as f32 - a.r as f32) * t).round() as u8,
        (a.g as f32 + (b.g as f32 - a.g as f32) * t).round() as u8,
        (a.b as f32 + (b.b as f32 - a.b as f32) * t).round() as u8,
        255,
    )
}

pub(crate) fn rgba(r: u8, g: u8, b: u8, a: u8) -> Rgba8 {
    Rgba8 { r, g, b, a }
}

pub(crate) fn translucent(mut color: Rgba8, alpha: u8) -> Rgba8 {
    color.a = alpha;
    color
}

pub(crate) fn push_rect(
    primitives: &mut Vec<PaintPrimitive>,
    widget_id: u64,
    rect: Rect,
    color: Rgba8,
) {
    primitives.push(PaintPrimitive::FillRect(PaintFillRect {
        widget_id,
        rect,
        color,
    }));
}

pub(crate) fn push_stroke(
    primitives: &mut Vec<PaintPrimitive>,
    widget_id: u64,
    rect: Rect,
    color: Rgba8,
    width: f32,
) {
    primitives.push(PaintPrimitive::StrokeRect(PaintStrokeRect {
        widget_id,
        rect,
        color,
        width,
    }));
}

pub(crate) fn push_text(
    primitives: &mut Vec<PaintPrimitive>,
    widget_id: u64,
    text: impl Into<String>,
    rect: Rect,
    color: Rgba8,
    align: PaintTextAlign,
) {
    primitives.push(PaintPrimitive::Text(PaintTextRun {
        widget_id,
        text: text.into().into(),
        rect,
        font_size: 12.0,
        baseline: Some(16.0),
        color,
        align,
        wrap: TextWrap::None,
    }));
}
