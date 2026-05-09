//! Fixed-column grid layout for gallery-style views.

use radiant::prelude::*;

fn main() -> radiant::Result {
    radiant::window("Radiant Grid Gallery")
        .size(720, 420)
        .min_size(480, 300)
        .run(
            column([
                text("Grid Gallery").height(30.0).fill_width(),
                scroll(
                    grid_with_gaps((0..18).map(tile), 3, 12.0, 12.0)
                        .padding(12.0)
                        .fill_width(),
                )
                .style(WidgetStyle::default())
                .fill(),
            ])
            .padding(16.0)
            .spacing(12.0),
        )
}

fn tile(index: usize) -> View {
    column([
        text(format!("Tile {:02}", index + 1))
            .height(24.0)
            .fill_width(),
        text("Fixed columns keep dense cards aligned while content remains normal Radiant views.")
            .wrap()
            .fill_width()
            .height(52.0),
    ])
    .style(if index.is_multiple_of(5) {
        WidgetStyle {
            tone: WidgetTone::Accent,
            prominence: WidgetProminence::Subtle,
        }
    } else {
        WidgetStyle::default()
    })
    .padding(10.0)
    .spacing(6.0)
    .fill_width()
    .height(104.0)
}
