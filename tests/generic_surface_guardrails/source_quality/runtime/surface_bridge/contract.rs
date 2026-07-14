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
fn runtime_bridge_contract_stays_minimal_and_routes_optional_host_capabilities() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let contract = fs::read_to_string(manifest_dir.join("src/runtime/bridge/contract.rs"))
        .expect("runtime bridge contract module should be readable");
    let capabilities = fs::read_to_string(manifest_dir.join("src/runtime/bridge/capabilities.rs"))
        .expect("runtime bridge capability facade should be readable");
    let docs =
        fs::read_to_string(manifest_dir.join("docs/API.md")).expect("API docs should be readable");
    let normalized_docs = docs.split_whitespace().collect::<Vec<_>>().join(" ");

    assert!(
        contract.lines().count() <= 80,
        "RuntimeBridge should stay focused on projection, update, and one capability table"
    );
    for core_method in [
        "fn project_surface",
        "fn pull_surface",
        "fn reduce_message",
        "fn update(",
        "fn update_with_runtime",
        "fn host_capabilities",
    ] {
        assert!(
            contract.contains(core_method),
            "RuntimeBridge should retain core method `{core_method}`"
        );
    }
    for optional_hook in [
        "fn paint_transient_overlay",
        "fn observe_frame_diagnostics",
        "fn request_platform_service",
        "fn spawn_message_task",
        "fn on_runtime_exit",
    ] {
        assert!(
            !contract.contains(optional_hook),
            "optional hook `{optional_hook}` belongs in a focused capability trait"
        );
    }
    for capability in [
        "RuntimeInputHost",
        "RuntimeTaskHost",
        "RuntimePlatformHost",
        "RuntimeQueueHost",
        "RuntimeAnimationHost",
        "RuntimeWindowHost",
        "RuntimeRetainedSurfaceHost",
        "RuntimeTransientOverlayHost",
        "RuntimeDiagnosticsHost",
        "RuntimeFrameDiagnosticsHost",
        "RuntimeLifecycleHost",
    ] {
        assert!(
            capabilities.contains(capability),
            "focused host capability `{capability}` should stay exported"
        );
    }
    assert!(
        normalized_docs.contains("`RuntimeBridge` is the minimal projection and update contract")
            && normalized_docs.contains("`RuntimeHostCapabilities` is cached once")
            && normalized_docs.contains("minimal custom host")
            && normalized_docs.contains("advanced capability host"),
        "API docs should explain the minimal bridge and explicit capability model"
    );
}
