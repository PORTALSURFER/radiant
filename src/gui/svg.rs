//! Vello SVG-backed parsing for retained vector icons.
//!
//! The public surface is [`SvgIcon`]. Parser internals stay private so Radiant
//! can widen SVG support without exposing document or element models.

mod document;

use vello::kurbo::{Affine, Vec2};

use crate::gui::types::{Rect, Rgba8};
use crate::runtime::{PaintFillPath, PaintFillRule, PaintPrimitive};
use crate::widgets::WidgetId;
use document::{SvgDocument, SvgFillRule};

/// Retained vector SVG icon parsed into backend-neutral paint paths.
#[derive(Clone, Debug)]
pub struct SvgIcon {
    document: SvgDocument,
}

impl SvgIcon {
    /// Parse an SVG icon from embedded source text.
    pub fn from_svg(svg: &str) -> Option<Self> {
        Some(Self {
            document: SvgDocument::parse(svg)?,
        })
    }

    /// Append this icon as filled vector paint primitives inside `rect`.
    pub fn append_fill_paint(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        widget_id: WidgetId,
        rect: Rect,
        color: Rgba8,
    ) {
        let scale_x = rect.width() / self.document.width.max(f32::EPSILON);
        let scale_y = rect.height() / self.document.height.max(f32::EPSILON);
        let transform = Affine::translate(Vec2::new(rect.min.x as f64, rect.min.y as f64))
            * Affine::scale_non_uniform(scale_x as f64, scale_y as f64);
        for shape in &self.document.shapes {
            primitives.push(PaintPrimitive::FillPath(PaintFillPath {
                widget_id,
                path: std::sync::Arc::clone(&shape.path),
                transform,
                fill_rule: shape.fill_rule.into(),
                color,
            }));
        }
    }
}

impl From<SvgFillRule> for PaintFillRule {
    fn from(value: SvgFillRule) -> Self {
        match value {
            SvgFillRule::NonZero => Self::NonZero,
            SvgFillRule::EvenOdd => Self::EvenOdd,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vello::kurbo::Shape;

    #[test]
    fn svg_icon_appends_scaled_fill_path_primitives() {
        let svg = r#"
            <svg viewBox="0 0 4 4" xmlns="http://www.w3.org/2000/svg">
              <rect x="0" y="0" width="2" height="4" />
            </svg>
        "#;
        let icon = SvgIcon::from_svg(svg).expect("icon should parse");
        let mut primitives = Vec::new();

        icon.append_fill_paint(
            &mut primitives,
            9,
            Rect::from_min_size(
                crate::gui::types::Point::new(10.0, 20.0),
                crate::gui::types::Vector2::new(8.0, 8.0),
            ),
            Rgba8 {
                r: 10,
                g: 20,
                b: 30,
                a: 255,
            },
        );
        icon.append_fill_paint(
            &mut primitives,
            9,
            Rect::from_min_size(
                crate::gui::types::Point::new(10.0, 20.0),
                crate::gui::types::Vector2::new(8.0, 8.0),
            ),
            Rgba8 {
                r: 10,
                g: 20,
                b: 30,
                a: 255,
            },
        );

        let [
            PaintPrimitive::FillPath(fill),
            PaintPrimitive::FillPath(second_fill),
        ] = primitives.as_slice()
        else {
            panic!("svg icon should append filled paths");
        };
        assert_eq!(fill.widget_id, 9);
        assert_eq!(fill.color.r, 10);
        assert!(std::sync::Arc::ptr_eq(&fill.path, &second_fill.path));
        let transformed_path = fill.transform * (*fill.path).clone();
        assert_eq!(
            transformed_path.bounding_box(),
            vello::kurbo::Rect::new(10.0, 20.0, 14.0, 28.0)
        );
    }
}
