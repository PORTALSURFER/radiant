//! Paint-plan scale sandbox for profiling public Radiant surfaces.

use radiant::prelude::*;
use radiant::{
    layout::{Point, Rect, Vector2, layout_tree},
    runtime::PaintPrimitive,
    theme::ThemeTokens,
};

const ROWS: u64 = 320;

fn main() {
    let report = run_rendering_benchmark(ROWS);
    println!(
        "radiant_rendering_benchmark rows={} layout_rects={} materialized_nodes={} primitives={} fills={} strokes={} text={} clips={}",
        report.rows,
        report.layout_rects,
        report.materialized_nodes,
        report.primitives.total,
        report.primitives.fills,
        report.primitives.strokes,
        report.primitives.text,
        report.primitives.clips,
    );
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct BenchmarkReport {
    rows: u64,
    layout_rects: usize,
    materialized_nodes: usize,
    primitives: PrimitiveCounts,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct PrimitiveCounts {
    total: usize,
    fills: usize,
    strokes: usize,
    text: usize,
    clips: usize,
    images: usize,
    custom_surfaces: usize,
    gpu_surfaces: usize,
}

fn run_rendering_benchmark(rows: u64) -> BenchmarkReport {
    let surface = benchmark_surface(rows).into_surface();
    let layout = layout_tree(
        &surface.layout_node(),
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(960.0, 720.0)),
    );
    let plan = surface.paint_plan(&layout, &ThemeTokens::default());

    BenchmarkReport {
        rows,
        layout_rects: layout.rects.len(),
        materialized_nodes: layout.stats.materialized_nodes,
        primitives: PrimitiveCounts::from_primitives(&plan.primitives),
    }
}

fn benchmark_surface(rows: u64) -> View {
    scroll(
        column((0..rows).map(benchmark_row))
            .id(2)
            .spacing(4.0)
            .padding(8.0)
            .fill_width(),
    )
    .id(1)
    .style(WidgetStyle::default())
    .fill()
}

fn benchmark_row(index: u64) -> View {
    row([
        text(format!("Render item {index:03}"))
            .id(10_000 + index * 10)
            .height(24.0)
            .fill_width(),
        badge(if index.is_multiple_of(3) {
            "hot"
        } else {
            "idle"
        })
        .subtle()
        .message(())
        .id(10_001 + index * 10)
        .size(64.0, 24.0),
        button("Inspect")
            .message(())
            .id(10_002 + index * 10)
            .size(92.0, 30.0),
    ])
    .id(100 + index)
    .style(if index.is_multiple_of(7) {
        WidgetStyle {
            tone: WidgetTone::Accent,
            prominence: WidgetProminence::Subtle,
        }
    } else {
        WidgetStyle::default()
    })
    .padding_x(12.0)
    .padding_y(8.0)
    .spacing(10.0)
    .height(46.0)
    .fill_width()
}

impl PrimitiveCounts {
    fn from_primitives(primitives: &[PaintPrimitive]) -> Self {
        let mut counts = Self {
            total: primitives.len(),
            ..Self::default()
        };
        for primitive in primitives {
            match primitive {
                PaintPrimitive::ClipStart(_) | PaintPrimitive::ClipEnd(_) => counts.clips += 1,
                PaintPrimitive::FillRect(_) | PaintPrimitive::FillPolygon(_) => counts.fills += 1,
                PaintPrimitive::StrokeRect(_)
                | PaintPrimitive::StrokePolygon(_)
                | PaintPrimitive::StrokePolyline(_) => counts.strokes += 1,
                PaintPrimitive::Text(_) | PaintPrimitive::TextInput(_) => counts.text += 1,
                PaintPrimitive::Image(_) => counts.images += 1,
                PaintPrimitive::CustomSurface(_) => counts.custom_surfaces += 1,
                PaintPrimitive::GpuSurface(_) => counts.gpu_surfaces += 1,
                PaintPrimitive::OverlayPanel(_) => {}
            }
        }
        counts
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rendering_benchmark_reports_layout_and_paint_scale() {
        let report = run_rendering_benchmark(96);

        assert_eq!(report.rows, 96);
        assert!(report.layout_rects > 96);
        assert!(report.materialized_nodes > 96);
        assert!(report.primitives.total > 96);
        assert!(report.primitives.fills > 0);
        assert!(report.primitives.text > 0);
        assert_eq!(report.primitives.gpu_surfaces, 0);
    }
}
