//! Product-neutral static split-pane geometry inspection.

use radiant::gui::types::{Point, Rect, Vector2};
use radiant::layout::{LayoutOutput, SplitPaneAxis, layout_tree};
use radiant::prelude as ui;
use radiant::prelude::IntoView;
use radiant::runtime::UiSurface;

fn main() {
    let output = static_split_layout();
    println!("static split rectangles: {}", output.rects.len());
    for (node_id, rect) in output.rects {
        println!("- node {node_id}: {rect:?}");
    }
}

fn static_split_layout() -> LayoutOutput {
    let surface: UiSurface<()> =
        ui::split_pane(ui::text("First pane").id(2), ui::text("Second pane").id(3))
            .axis(SplitPaneAxis::Horizontal)
            .initial_ratio(0.35)
            .min_first(32.0)
            .min_second(64.0)
            .divider_extent(8.0)
            .into_surface();

    layout_tree(
        &surface.layout_node(),
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(480.0, 180.0)),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn static_split_example_reports_two_panes_and_a_root() {
        let output = static_split_layout();

        assert_eq!(output.rects.len(), 3);
        assert_eq!(output.rects[&2].width(), 165.0);
        assert_eq!(output.rects[&3].min.x, 173.0);
        assert_eq!(output.rects[&3].width(), 307.0);
    }
}
