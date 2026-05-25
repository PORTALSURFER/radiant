use super::public_prelude_source;
use std::{fs, path::PathBuf};

#[path = "resources_gpu/content.rs"]
mod content;
#[path = "resources_gpu/input_builder.rs"]
mod input_builder;
#[path = "resources_gpu/primitive.rs"]
mod primitive;
#[path = "resources_gpu/resource_completion.rs"]
mod resource_completion;
#[path = "resources_gpu/slot.rs"]
mod slot;
#[path = "resources_gpu/widget.rs"]
mod widget;

fn radiant_source(relative: &str) -> String {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    fs::read_to_string(manifest_dir.join(relative))
        .unwrap_or_else(|err| panic!("{relative} should be readable: {err}"))
}

fn prelude_source() -> String {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    public_prelude_source(&manifest_dir)
}
