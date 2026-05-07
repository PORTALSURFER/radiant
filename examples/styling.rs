//! Default control styles and interaction states.

use radiant::prelude::*;

fn main() -> radiant::Result {
    radiant::window("Radiant Styling")
        .size(360, 220)
        .min_size(280, 180)
        .run(
            column([
                text("Button styles").size(160.0, 28.0),
                row([
                    button("Default").message(()),
                    button("Primary").primary().message(()),
                    button("Danger").danger().message(()),
                ])
                .spacing(8.0),
                row([
                    button("Subtle").subtle().message(()),
                    toggle("Toggle", false).message(|_| ()),
                ])
                .spacing(8.0),
            ])
            .padding(16.0)
            .spacing(12.0),
        )
}
