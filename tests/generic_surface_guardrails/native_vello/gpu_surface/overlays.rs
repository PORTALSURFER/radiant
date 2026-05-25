use std::{fs, path::PathBuf};

#[path = "overlays/custom_shader.rs"]
mod custom_shader;
#[path = "overlays/imports.rs"]
mod imports;
#[path = "overlays/uniforms.rs"]
mod uniforms;

fn gpu_surface_source(relative: &str) -> String {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    fs::read_to_string(manifest_dir.join(relative))
        .unwrap_or_else(|err| panic!("{relative} should be readable: {err}"))
}
