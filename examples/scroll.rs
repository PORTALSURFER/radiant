//! Simple scroll viewport around static text rows.

use radiant::prelude::*;

fn main() -> radiant::Result {
    radiant::window("Radiant Scroll")
        .size(320, 240)
        .min_size(240, 160)
        .run(
            column([
                text("Scroll area").size(140.0, 28.0),
                scroll_column(1..=50, |number| {
                    text(format!("Scrollable row {number:02}"))
                        .fill_width()
                        .height(24.0)
                })
                .fill_height()
                .spacing(4.0),
            ])
            .padding(16.0)
            .spacing(12.0),
        )
}
