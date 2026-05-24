use super::*;

#[test]
fn runtime_bridge_app_contract_stays_in_focused_module() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let bridge = fs::read_to_string(manifest_dir.join("src/runtime/bridge.rs"))
        .expect("runtime bridge module should be readable");
    let app = fs::read_to_string(manifest_dir.join("src/runtime/bridge/app.rs"))
        .expect("runtime bridge app contract module should be readable");
    let auxiliary = fs::read_to_string(manifest_dir.join("src/runtime/bridge/auxiliary.rs"))
        .expect("runtime bridge auxiliary window model should be readable");
    let contract = fs::read_to_string(manifest_dir.join("src/runtime/bridge/contract.rs"))
        .expect("runtime bridge contract module should be readable");
    let runtime =
        fs::read_to_string(manifest_dir.join("src/runtime/mod.rs")).expect("runtime module");

    assert!(
        bridge.contains("mod app;")
            && bridge.contains("pub use app::App;")
            && runtime.contains("App,"),
        "runtime bridge root should publicly re-export the focused App contract"
    );
    assert!(
        app.contains("pub trait App<Message>: RuntimeBridge<Message>")
            && app.contains("impl<Bridge, Message> App<Message> for Bridge where Bridge: RuntimeBridge<Message> {}")
            && !contract.contains("pub trait App<Message>"),
        "the public App marker contract should stay in runtime/bridge/app.rs"
    );
    assert!(
        bridge.contains("mod auxiliary;")
            && bridge.contains("pub use auxiliary::AuxiliaryWindow;")
            && auxiliary.contains("pub struct AuxiliaryWindow<Message>")
            && auxiliary.contains("pub fn new(")
            && auxiliary.contains("pub fn on_close(mut self, message: Message) -> Self")
            && !contract.contains("pub struct AuxiliaryWindow<Message>"),
        "the public auxiliary-window projection model should stay out of the RuntimeBridge trait contract"
    );
}

#[test]
fn runtime_bridge_contract_documents_adapter_hook_groups() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let contract = fs::read_to_string(manifest_dir.join("src/runtime/bridge/contract.rs"))
        .expect("runtime bridge contract module should be readable");
    let docs =
        fs::read_to_string(manifest_dir.join("docs/API.md")).expect("API docs should be readable");
    let normalized_docs = docs.split_whitespace().collect::<Vec<_>>().join(" ");

    for group in [
        "Surface projection.",
        "State updates and input policy.",
        "Runtime scheduling and host work.",
        "Platform services.",
        "Runtime-owned queues.",
        "Animation policy.",
        "Retained and transient rendering hooks.",
        "Diagnostics and lifecycle.",
    ] {
        assert!(
            contract.contains(group),
            "RuntimeBridge should document adapter hook group `{group}`"
        );
    }
    assert!(
        normalized_docs.contains("`RuntimeBridge` remains the single explicit adapter trait")
            && normalized_docs.contains("surface projection, state updates and input policy, runtime scheduling, platform services, runtime-owned queues, animation policy, retained/transient rendering, diagnostics, and lifecycle")
            && normalized_docs.contains("custom bridges should override only the groups they own"),
        "API docs should explain RuntimeBridge hook groups without presenting them as a second app framework"
    );
}
