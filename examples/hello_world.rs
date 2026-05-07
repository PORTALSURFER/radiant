//! Minimal hello-world app built with Radiant application builders.

use radiant::prelude::*;

fn main() -> radiant::Result {
    radiant::window("Radiant Hello World")
        .size(320, 120)
        .min_size(240, 96)
        .run(text("Hello, world!"))
}
