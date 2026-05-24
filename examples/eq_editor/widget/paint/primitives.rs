use radiant::prelude::*;

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
