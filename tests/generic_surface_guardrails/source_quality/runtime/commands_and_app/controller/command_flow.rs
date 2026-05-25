use super::*;

#[test]
fn controller_commands_keep_outcome_drain_and_dispatch_in_focused_modules() {
    let root = controller_source("src/runtime/controller/commands.rs");
    let outcome = controller_source("src/runtime/controller/commands/outcome.rs");
    let drain = controller_source("src/runtime/controller/commands/drain.rs");
    let dispatch = controller_source("src/runtime/controller/commands/dispatch.rs");
    let external_drag = controller_source("src/runtime/controller/commands/external_drag.rs");
    let scrolling = controller_source("src/runtime/controller/commands/scrolling.rs");
    let scroll_wheel = controller_source("src/runtime/controller/scroll/wheel.rs");
    let tests = controller_source("src/runtime/controller/commands/tests.rs");
    let test_batching = controller_source("src/runtime/controller/commands/tests/batching.rs");
    let test_drain = controller_source("src/runtime/controller/commands/tests/drain.rs");
    let test_external_drag =
        controller_source("src/runtime/controller/commands/tests/external_drag.rs");
    let test_platform = controller_source("src/runtime/controller/commands/tests/platform.rs");
    let test_fixtures = controller_source("src/runtime/controller/commands/tests/fixtures.rs");

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
        root.contains("use super::SurfaceRuntime;")
            && root.contains("use crate::runtime::{Command, RuntimeBridge};")
            && root.contains("#[cfg(test)]")
            && root.contains("gui::types::{Point, Vector2}")
            && root.contains("runtime::UiSurface")
            && !root.starts_with("use super::*;"),
        "runtime controller command root should name production dependencies and keep fixture-only geometry/surface imports under cfg(test)"
    );
    assert!(
        outcome.contains("pub struct CommandOutcome")
            && outcome.contains("fn finish_command_outcome")
            && outcome.contains("use super::SurfaceRuntime;")
            && outcome.contains("use crate::runtime::RuntimeBridge;")
            && !outcome.starts_with("use super::*;")
            && !root.contains("pub struct CommandOutcome"),
        "command pass result and finalization should live in commands/outcome.rs with explicit controller and bridge dependencies"
    );
    assert!(
        drain.contains("pub fn drain_runtime_messages")
            && drain.contains(".drain_bridge_commands")
            && drain.contains(".drain_bridge_messages")
            && !root.contains("pub fn drain_runtime_messages"),
        "runtime work draining should live in commands/drain.rs"
    );
    assert!(
        dispatch.contains("fn execute_command_inner")
            && dispatch.contains("Command::PlatformRequest")
            && dispatch.contains("Command::ScrollFixedRowIntoView")
            && dispatch.contains("use super::{CommandOutcome, SurfaceRuntime};")
            && dispatch.contains("gui::types::Vector2")
            && dispatch
                .contains("runtime::{Command, DragSession, ExternalDragSession, RuntimeBridge}")
            && !dispatch.starts_with("use super::*;")
            && !root.contains("fn execute_command_inner"),
        "command execution branches should live in commands/dispatch.rs"
    );
    assert!(
        external_drag.contains("use super::{CommandOutcome, SurfaceRuntime};")
            && external_drag
                .contains("runtime::{ExternalDragOutcome, ExternalDragSession, RuntimeBridge}")
            && !external_drag.starts_with("use super::*;")
            && scrolling.contains("use super::super::{ScrollUpdate, SurfaceRuntime};")
            && scrolling.contains("gui::types::{Point, Vector2}")
            && scrolling.contains("layout::NodeId")
            && scrolling.contains("runtime::RuntimeBridge")
            && !scrolling.starts_with("use super::super::*;"),
        "external drag and scrolling command helpers should own their drag, scroll, geometry, layout, and bridge dependencies"
    );
    assert!(
        scroll_wheel.contains("use super::super::{CommandOutcome, SurfaceRuntime};")
            && scroll_wheel.contains("gui::types::{Point, Vector2}")
            && scroll_wheel.contains("runtime::{RuntimeBridge, WidgetDispatchResult}")
            && scroll_wheel.contains("widgets::{PointerModifiers, WidgetId, WidgetInput}")
            && !scroll_wheel.starts_with("use super::super::*;")
            && scroll_wheel.contains("fn dispatch_wheel_at_with_refresh")
            && scroll_wheel.contains("fn wheel_widget_at"),
        "runtime controller wheel routing should name command outcome, controller, geometry, bridge, dispatch result, pointer, and widget dependencies without inheriting the controller root"
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

#[test]
fn runtime_surface_public_command_tests_stay_grouped_by_command_concern() {
    let root = controller_source("tests/runtime_surface_public_api/commands.rs");
    let fixtures = controller_source("tests/runtime_surface_public_api/commands/fixtures.rs");
    let repaint = controller_source("tests/runtime_surface_public_api/commands/repaint.rs");
    let runtime_commands =
        controller_source("tests/runtime_surface_public_api/commands/runtime_commands.rs");
    let window = controller_source("tests/runtime_surface_public_api/commands/window.rs");

    assert!(
        root.contains("#[path = \"commands/fixtures.rs\"]")
            && root.contains("#[path = \"commands/repaint.rs\"]")
            && root.contains("#[path = \"commands/runtime_commands.rs\"]")
            && root.contains("#[path = \"commands/window.rs\"]")
            && !root.contains("fn surface_runtime_executes_command_messages_and_repaint_requests")
            && !root.contains("struct RuntimeCommandBridge"),
        "runtime surface public command test root should index fixtures, repaint, runtime command, and window command groups"
    );
    assert!(
        fixtures.contains("enum CommandDemoMessage")
            && fixtures.contains("struct CommandDemoBridge")
            && fixtures.contains("struct RuntimeCommandBridge")
            && fixtures.contains("fn project_demo_surface")
            && fixtures.contains("fn drain_until_messages"),
        "runtime surface public command fixtures should own bridge, surface projection, and drain helpers"
    );
    assert!(
        repaint.contains("fn surface_runtime_treats_mixed_repaint_batches_as_surface_refreshes")
            && repaint
                .contains("fn surface_runtime_executes_command_messages_and_repaint_requests")
            && runtime_commands
                .contains("fn surface_runtime_executes_focus_exit_and_deferred_commands")
            && window
                .contains("fn surface_runtime_reports_dpi_scale_overrides_as_surface_refreshes")
            && window
                .contains("fn surface_runtime_reports_window_size_requests_as_surface_refreshes"),
        "runtime surface public command behavior should stay grouped by repaint, deferred runtime commands, and window commands"
    );
}
