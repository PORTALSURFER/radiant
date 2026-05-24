//! Native Vello surface and frame-boundary guardrails.

use std::{fs, path::PathBuf};

#[path = "surface_backend/diagnostics_upload.rs"]
mod diagnostics_upload;
#[path = "surface_backend/frame_pipeline.rs"]
mod frame_pipeline;
#[path = "surface_backend/surface_lifecycle.rs"]
mod surface_lifecycle;
#[path = "surface_backend/window_platform.rs"]
mod window_platform;

pub(super) fn read_runtime_source(relative: &str) -> String {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    fs::read_to_string(manifest_dir.join(relative))
        .unwrap_or_else(|err| panic!("{relative} should be readable: {err}"))
}
