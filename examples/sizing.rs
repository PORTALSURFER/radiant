//! Sizing helpers for fixed, preferred, and fill layouts.

use radiant::prelude::*;

fn main() -> radiant::Result {
    radiant::window("Radiant Sizing")
        .size(480, 220)
        .min_size(360, 160)
        .run(
            column([
                text("Sizing").size(120.0, 28.0),
                row([
                    text("Fixed").size(80.0, 32.0),
                    text("Fill width").fill_width().height(32.0),
                    button("Fixed").message(()).size(90.0, 32.0),
                ])
                .fill_width(),
                row([
                    text_input("Preferred")
                        .message(|_| ())
                        .min_size(140.0, 32.0)
                        .preferred_size(240.0, 32.0),
                    text("Fixed height").fill_width().height(32.0),
                ])
                .fill_width(),
            ])
            .padding(16.0)
            .spacing(12.0),
        )
}
