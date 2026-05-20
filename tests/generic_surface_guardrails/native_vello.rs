//! Native Vello and renderer-boundary structural guardrails.

use std::{fs, path::PathBuf};

use super::{relative_path, rust_sources_under};

#[path = "native_vello/gpu_surface.rs"]
mod gpu_surface;
#[path = "native_vello/public_api_svg.rs"]
mod public_api_svg;
#[path = "native_vello/runtime_input.rs"]
mod runtime_input;
#[path = "native_vello/scene_cache.rs"]
mod scene_cache;
#[path = "native_vello/surface_backend.rs"]
mod surface_backend;
#[path = "native_vello/text_and_drag.rs"]
mod text_and_drag;
