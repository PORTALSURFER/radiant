use super::super::*;
use crate::{
    runtime::{
        GpuSurfaceCapabilities, GpuSurfaceContent, PaintFillRect, PaintGpuSurface, PaintTextAlign,
        PaintTextRun,
    },
    widgets::TextWrap,
};
use std::sync::Arc;

pub(super) fn fill(widget_id: u64) -> PaintPrimitive {
    fill_rect(
        widget_id,
        UiRect::from_min_size(Point::new(0.0, 0.0), Vector2::new(1.0, 1.0)),
        white(),
    )
}

pub(super) fn rect(rect: UiRect, color: Rgba8) -> PaintPrimitive {
    fill_rect(1, rect, color)
}

fn fill_rect(widget_id: u64, rect: UiRect, color: Rgba8) -> PaintPrimitive {
    PaintPrimitive::FillRect(PaintFillRect {
        widget_id,
        rect,
        color,
    })
}

pub(super) fn stroke(widget_id: u64) -> PaintPrimitive {
    PaintPrimitive::StrokeRect(PaintStrokeRect {
        widget_id,
        rect: UiRect::from_min_size(Point::new(0.0, 0.0), Vector2::new(1.0, 1.0)),
        color: white(),
        width: 1.0,
    })
}

pub(super) fn text(label: &str) -> PaintPrimitive {
    PaintPrimitive::Text(PaintTextRun {
        widget_id: 9,
        text: label.into(),
        rect: UiRect::from_min_size(Point::new(4.0, 4.0), Vector2::new(120.0, 18.0)),
        font_size: 12.0,
        baseline: None,
        color: white(),
        align: PaintTextAlign::Left,
        wrap: TextWrap::None,
    })
}

pub(super) fn white() -> Rgba8 {
    Rgba8 {
        r: 255,
        g: 255,
        b: 255,
        a: 255,
    }
}

pub(super) fn translucent_white() -> Rgba8 {
    Rgba8 {
        r: 255,
        g: 255,
        b: 255,
        a: 160,
    }
}

pub(super) fn gpu(widget_id: u64) -> PaintPrimitive {
    PaintPrimitive::GpuSurface(PaintGpuSurface {
        widget_id,
        key: widget_id,
        revision: 0,
        rect: UiRect::from_min_size(Point::new(0.0, 0.0), Vector2::new(1.0, 1.0)),
        content: GpuSurfaceContent::RgbaAtlas {
            atlas: Arc::new(
                crate::gui::types::ImageRgba::new(1, 1, vec![255, 255, 255, 255])
                    .expect("valid one-pixel image"),
            ),
            source_rect: UiRect::from_min_size(Point::new(0.0, 0.0), Vector2::new(1.0, 1.0)),
        },
        capabilities: GpuSurfaceCapabilities::default(),
        overlays: Vec::new(),
    })
}
