use super::*;

#[test]
fn controller_commands_keep_outcome_drain_and_dispatch_in_focused_modules() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let root = fs::read_to_string(manifest_dir.join("src/runtime/controller/commands.rs"))
        .expect("runtime controller command root should be readable");
    let outcome =
        fs::read_to_string(manifest_dir.join("src/runtime/controller/commands/outcome.rs"))
            .expect("runtime command outcome module should be readable");
    let drain = fs::read_to_string(manifest_dir.join("src/runtime/controller/commands/drain.rs"))
        .expect("runtime command drain module should be readable");
    let dispatch =
        fs::read_to_string(manifest_dir.join("src/runtime/controller/commands/dispatch.rs"))
            .expect("runtime command dispatch module should be readable");
    let tests = fs::read_to_string(manifest_dir.join("src/runtime/controller/commands/tests.rs"))
        .expect("runtime command test root should be readable");
    let test_batching =
        fs::read_to_string(manifest_dir.join("src/runtime/controller/commands/tests/batching.rs"))
            .expect("runtime command batching tests should be readable");
    let test_drain =
        fs::read_to_string(manifest_dir.join("src/runtime/controller/commands/tests/drain.rs"))
            .expect("runtime command drain tests should be readable");
    let test_external_drag = fs::read_to_string(
        manifest_dir.join("src/runtime/controller/commands/tests/external_drag.rs"),
    )
    .expect("runtime command external drag tests should be readable");
    let test_platform =
        fs::read_to_string(manifest_dir.join("src/runtime/controller/commands/tests/platform.rs"))
            .expect("runtime command platform tests should be readable");
    let test_fixtures =
        fs::read_to_string(manifest_dir.join("src/runtime/controller/commands/tests/fixtures.rs"))
            .expect("runtime command test fixtures should be readable");

    for required in [
        "mod dispatch;",
        "mod drain;",
        "mod outcome;",
        "pub use outcome::CommandOutcome;",
    ] {
        assert!(
            root.contains(required),
            "runtime controller command root should delegate `{required}`"
        );
    }
    assert!(
        outcome.contains("pub struct CommandOutcome")
            && outcome.contains("fn finish_command_outcome")
            && !root.contains("pub struct CommandOutcome"),
        "command pass result and finalization should live in commands/outcome.rs"
    );
    assert!(
        drain.contains("pub fn drain_runtime_messages")
            && drain.contains("take_runtime_command_batch_into")
            && !root.contains("pub fn drain_runtime_messages"),
        "runtime work draining should live in commands/drain.rs"
    );
    assert!(
        dispatch.contains("fn execute_command_inner")
            && dispatch.contains("Command::PlatformRequest")
            && dispatch.contains("Command::ScrollFixedRowIntoView")
            && !root.contains("fn execute_command_inner"),
        "command execution branches should live in commands/dispatch.rs"
    );
    assert!(
        tests.contains("mod batching;")
            && tests.contains("mod drain;")
            && tests.contains("mod external_drag;")
            && tests.contains("mod platform;")
            && tests.contains("mod fixtures;")
            && !tests.contains("fn runtime_command_batch_preserves_order_and_keeps_remainder"),
        "runtime controller command test root should index focused behavior groups instead of owning all cases"
    );
    assert!(
        test_batching.contains("fn runtime_command_batch_preserves_order_and_keeps_remainder")
            && test_drain
                .contains("fn runtime_command_drains_are_bounded_and_request_followup_wakeup")
            && test_external_drag
                .contains("fn external_drag_command_arms_and_clears_native_session")
            && test_platform.contains("fn platform_request_dispatches_through_bridge_completion")
            && test_fixtures.contains("struct QueuedCommandBridge"),
        "runtime controller command tests should stay grouped by batching, drain, external drag, platform, and fixtures concerns"
    );
}
