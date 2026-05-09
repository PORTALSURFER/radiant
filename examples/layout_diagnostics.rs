//! Layout diagnostics and debug primitive collection.

use radiant::gui::types::{Point, Rect, Vector2};
use radiant::layout::{LayoutDebugOptions, LayoutOutput, LayoutState, layout_tree_with_state};
use radiant::prelude::*;
use radiant::runtime::UiSurface;

fn main() {
    let output = diagnostic_layout();

    println!("layout diagnostics: {}", output.diagnostics.len());
    for diagnostic in &output.diagnostics {
        println!(
            "- node {} {:?}: {}",
            diagnostic.node_id, diagnostic.code, diagnostic.message
        );
    }
    println!("debug primitives: {}", output.debug_primitives.len());
}

fn diagnostic_layout() -> LayoutOutput {
    let surface: UiSurface<()> = column([
        text("Layout Diagnostics").height(28.0).fill_width(),
        scroll(
            column((0..10).map(|index| {
                text(format!("Diagnostic row {index:02}"))
                    .height(28.0)
                    .fill_width()
            }))
            .fill_width()
            .spacing(4.0),
        )
        .id(10)
        .fill_height(),
    ])
    .id(1)
    .padding(10.0)
    .spacing(8.0)
    .into_surface();

    let mut state = LayoutState::default();
    state.scroll_offsets.insert(10, Vector2::new(0.0, 1_000.0));

    layout_tree_with_state(
        &surface.layout_node(),
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(280.0, 120.0)),
        &state,
        LayoutDebugOptions::all_enabled(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use radiant::layout::{DebugPrimitiveKind, LayoutDiagnosticCode};

    #[test]
    fn layout_diagnostics_example_collects_diagnostics_and_debug_primitives() {
        let output = diagnostic_layout();

        assert!(output.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == LayoutDiagnosticCode::InvalidScrollOffsetClamped
        }));
        assert!(
            output
                .debug_primitives
                .iter()
                .any(|primitive| { primitive.kind == DebugPrimitiveKind::NodeBounds })
        );
        assert!(
            output
                .debug_primitives
                .iter()
                .any(|primitive| { primitive.kind == DebugPrimitiveKind::ViewportBounds })
        );
    }
}
