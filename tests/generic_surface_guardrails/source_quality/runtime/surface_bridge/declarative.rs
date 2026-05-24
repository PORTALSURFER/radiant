use super::*;

#[test]
fn declarative_runtime_bridges_use_named_parts_for_host_closures() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let message =
        fs::read_to_string(manifest_dir.join("src/runtime/bridge/declarative/message.rs"))
            .expect("declarative message bridge should be readable");
    let message_shared =
        fs::read_to_string(manifest_dir.join("src/runtime/bridge/declarative/message/shared.rs"))
            .expect("declarative shared-surface message bridge should be readable");
    let message_owned =
        fs::read_to_string(manifest_dir.join("src/runtime/bridge/declarative/message/owned.rs"))
            .expect("declarative owned-surface message bridge should be readable");
    let command =
        fs::read_to_string(manifest_dir.join("src/runtime/bridge/declarative/command.rs"))
            .expect("declarative command bridge should be readable");
    let bridge = fs::read_to_string(manifest_dir.join("src/runtime/bridge.rs"))
        .expect("runtime bridge module should be readable");
    let runtime =
        fs::read_to_string(manifest_dir.join("src/runtime/mod.rs")).expect("runtime module");

    for (source, parts, from_parts, wrapper) in [
        (
            message_shared.as_str(),
            "pub struct DeclarativeRuntimeBridgeParts",
            "pub fn from_parts(parts: DeclarativeRuntimeBridgeParts<State, Project, Reduce>) -> Self",
            "Self::from_parts(DeclarativeRuntimeBridgeParts {",
        ),
        (
            message_owned.as_str(),
            "pub struct DeclarativeOwnedRuntimeBridgeParts",
            "pub fn from_parts(parts: DeclarativeOwnedRuntimeBridgeParts<State, Project, Reduce>) -> Self",
            "Self::from_parts(DeclarativeOwnedRuntimeBridgeParts {",
        ),
        (
            command.as_str(),
            "pub struct DeclarativeCommandRuntimeBridgeParts",
            "pub fn from_parts(parts: DeclarativeCommandRuntimeBridgeParts<State, Project, Update>) -> Self",
            "Self::from_parts(DeclarativeCommandRuntimeBridgeParts {",
        ),
        (
            command.as_str(),
            "pub struct DeclarativeOwnedCommandRuntimeBridgeParts",
            "parts: DeclarativeOwnedCommandRuntimeBridgeParts<State, Project, Update>",
            "Self::from_parts(DeclarativeOwnedCommandRuntimeBridgeParts {",
        ),
    ] {
        assert!(
            source.contains(parts) && source.contains(from_parts) && source.contains(wrapper),
            "declarative runtime bridge should expose named parts and compatibility wrappers for {parts}"
        );
    }
    assert!(
        message.contains("mod owned;")
            && message.contains("mod shared;")
            && message.contains("pub use owned::{")
            && message.contains("pub use shared::{")
            && !message.contains("pub struct DeclarativeRuntimeBridge<")
            && !message.contains("pub struct DeclarativeOwnedRuntimeBridge<"),
        "declarative message bridge root should re-export focused shared and owned bridge modules"
    );
    for export in [
        "DeclarativeRuntimeBridgeParts",
        "DeclarativeOwnedRuntimeBridgeParts",
        "DeclarativeCommandRuntimeBridgeParts",
        "DeclarativeOwnedCommandRuntimeBridgeParts",
    ] {
        assert!(
            bridge.contains(export) && runtime.contains(export),
            "runtime bridge named parts type {export} should stay publicly exported"
        );
    }
}
