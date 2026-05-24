//! Documentation and diagnostics guardrails.

use std::{fs, path::PathBuf};

#[path = "diagnostics/custom_shader.rs"]
mod custom_shader;
#[path = "diagnostics/models.rs"]
mod models;
#[path = "diagnostics/quality_gates.rs"]
mod quality_gates;
#[path = "diagnostics/text.rs"]
mod text;
#[path = "diagnostics/timing.rs"]
mod timing;

pub(super) fn read_project_file(relative: &str) -> String {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    fs::read_to_string(manifest_dir.join(relative))
        .unwrap_or_else(|err| panic!("{relative} should be readable: {err}"))
}

pub(super) fn normalized(source: &str) -> String {
    source.split_whitespace().collect::<Vec<_>>().join(" ")
}
