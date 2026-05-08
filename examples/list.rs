//! Scrollable list with stable row heights.

use radiant::prelude::*;

fn main() -> radiant::Result {
    radiant::window("Radiant List")
        .size(360, 260)
        .min_size(280, 180)
        .run(
            column([
                text("Numbered rows").size(160.0, 28.0),
                list(1..=24, |number| {
                    list_row(
                        number,
                        [
                            text(format!("Row {number:02}")).fill_width(),
                            button("Open").message(()).subtle(),
                        ],
                    )
                })
                .fill_height(),
            ])
            .padding(16.0)
            .spacing(12.0),
        )
}
