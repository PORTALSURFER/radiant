use super::{prelude_source, radiant_source};

#[test]
fn resource_slots_keep_load_state_and_lifecycle_focused() {
    let slot = radiant_source("src/runtime/resource/slot.rs");
    let state = radiant_source("src/runtime/resource/slot/state.rs");
    let resource = radiant_source("src/runtime/resource.rs");
    let runtime = radiant_source("src/runtime/mod.rs");
    let prelude = prelude_source();

    assert!(
        slot.contains("mod state;")
            && slot.contains("pub use state::ResourceLoadState;")
            && slot.contains("pub struct ResourceSlot<T>")
            && slot.contains("pub fn begin_load")
            && slot.contains("pub fn apply_for"),
        "resource slot lifecycle should stay in slot.rs while delegating load-state model"
    );
    assert!(
        !slot.contains("pub enum ResourceLoadState")
            && state.contains("pub enum ResourceLoadState")
            && state.contains("Idle")
            && state.contains("Loading")
            && state.contains("Ready")
            && state.contains("Failed"),
        "resource load-state enum should live in runtime/resource/slot/state.rs"
    );
    assert!(
        resource.contains("pub use slot::{ResourceLoadState, ResourceSlot};")
            && runtime.contains("ResourceLoadState")
            && runtime.contains("ResourceSlot")
            && prelude.contains("ResourceLoadState")
            && prelude.contains("ResourceSlot"),
        "resource slot and load-state types should remain exported through runtime and prelude"
    );
}
