//! Static rows and columns showing padding, spacing, and fill behavior.

use radiant::prelude::*;

fn main() -> radiant::Result {
    radiant::window("Radiant Rows and Columns")
        .size(460, 220)
        .min_size(340, 160)
        .run(
            column([
                text("Rows and columns").size(180.0, 28.0),
                row([
                    text("Left").size(90.0, 32.0),
                    text("Fills width").fill_width().height(32.0),
                    button("Action").message(()).primary(),
                ])
                .fill_width()
                .spacing(8.0),
                row([
                    button("One").message(()).subtle(),
                    button("Two").message(()),
                    button("Three").message(()).danger(),
                ])
                .spacing(8.0),
            ])
            .padding(16.0)
            .spacing(12.0),
        )
}
