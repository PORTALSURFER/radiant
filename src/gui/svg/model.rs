use kurbo::BezPath;

/// Parsed SVG document ready for simple mask-style rasterization.
///
/// This intentionally supports a small icon-oriented subset: view boxes,
/// groups, paths, rectangles, circles, polygons, transforms, style `fill`, and
/// `fill-rule`. Retained Vello SVG painting remains the preferred path for full
/// backend rendering.
#[derive(Clone, Debug)]
pub struct SvgDocument {
    /// The minimum x coordinate in the declared view box.
    pub view_box_min_x: f32,
    /// The minimum y coordinate in the declared view box.
    pub view_box_min_y: f32,
    /// The width of the declared view box.
    pub view_box_width: f32,
    /// The height of the declared view box.
    pub view_box_height: f32,
    /// The transformed filled shapes emitted by the document.
    pub shapes: Vec<SvgShape>,
}

/// One rasterizable filled SVG shape.
#[derive(Clone, Debug)]
pub struct SvgShape {
    pub(super) path: BezPath,
    pub(super) fill_rule: SvgFillRule,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum SvgFillRule {
    NonZero,
    EvenOdd,
}

impl SvgShape {
    pub(super) fn new(path: BezPath, fill_rule: SvgFillRule) -> Self {
        Self { path, fill_rule }
    }
}
