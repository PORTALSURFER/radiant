use super::*;

#[path = "controller/command_flow.rs"]
mod command_flow;
#[path = "controller/pointer.rs"]
mod pointer;
#[path = "controller/state_context.rs"]
mod state_context;

fn controller_source(relative: &str) -> String {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    fs::read_to_string(manifest_dir.join(relative))
        .unwrap_or_else(|err| panic!("{relative} should be readable: {err}"))
}
