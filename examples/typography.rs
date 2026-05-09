//! Text wrapping, truncation, baseline, and sizing policies.

use radiant::prelude::*;

fn main() -> radiant::Result {
    radiant::window("Radiant Typography")
        .size(640, 360)
        .min_size(420, 260)
        .run(
            column([
                text("Typography").size(220.0, 32.0).baseline(22.0),
                text(
                    "Wrapped text keeps a fixed row height while Radiant clips, measures, and paints words inside the assigned rectangle.",
                )
                .wrap()
                .fill_width()
                .height(72.0)
                .baseline(18.0),
                text(
                    "Truncated single-line text demonstrates overflow behavior without changing layout height.",
                )
                .truncate()
                .fill_width()
                .height(28.0)
                .baseline(19.0),
                row([
                    text("Label").size(92.0, 28.0).baseline(19.0),
                    text("Value aligned on the same baseline")
                        .fill_width()
                        .height(28.0)
                        .baseline(19.0),
                ])
                .fill_width()
                .spacing(8.0),
            ])
            .padding(18.0)
            .spacing(12.0),
        )
}
