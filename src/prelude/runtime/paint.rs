//! Backend-neutral paint primitives used by common custom widgets.

pub use crate::runtime::{
    PaintClipEnd, PaintClipStart, PaintFillRect, PaintFillRectBatch, PaintFillRule, PaintImage,
    PaintPath, PaintPathCommand, PaintPrimitive, PaintRectList, PaintStrokeRect,
    PaintStrokeRectBatch, PaintSvg, PaintSvgDocument, PaintTextAlign, PaintTextMetrics,
    PaintTextRun, PaintTransform, TransientOverlayContext,
};
