//! Small external-style custom layout policy using ordinary declarative children.

use radiant::application::layout;
use radiant::layout::{
    Constraints, LayoutPolicy, MeasureChildren, PlaceChildren, Rect, SizeHint, Vector2,
};
use radiant::prelude::*;

struct TwoColumn;

impl LayoutPolicy for TwoColumn {
    fn measure(&self, children: &mut MeasureChildren<'_>, constraints: Constraints) -> SizeHint {
        let first = children.measure(0, constraints).unwrap_or_default();
        let second = children.measure(1, constraints).unwrap_or_default();
        SizeHint::preferred(Vector2::new(first.x + second.x, first.y.max(second.y)))
    }

    fn place(&self, children: &mut PlaceChildren<'_>, bounds: Rect) {
        let width = bounds.width() * 0.5;
        children
            .place(
                0,
                Rect::from_xy_size(bounds.min.x, bounds.min.y, width, bounds.height()),
            )
            .expect("the first child is declared");
        children
            .place(
                1,
                Rect::from_xy_size(bounds.min.x + width, bounds.min.y, width, bounds.height()),
            )
            .expect("the second child is declared");
    }
}

fn main() {
    let _output = layout(TwoColumn, [text::<()>("Left"), text::<()>("Right")])
        .view_layout(Rect::from_size(320.0, 80.0));
}
