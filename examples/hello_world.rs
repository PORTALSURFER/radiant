//! Minimal hello-world app built on the beginner-facing Radiant API.

use radiant::prelude::*;

fn main() -> radiant::Result {
    radiant::window("Radiant Hello World")
        .size(320, 120)
        .min_size(240, 96)
        .run(text("Hello, world!"))
}
