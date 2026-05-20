use super::*;

#[test]
fn layout_row_helpers_keep_geometry_and_tests_focused() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let root = fs::read_to_string(manifest_dir.join("src/gui/layout_core/row_helpers.rs"))
        .expect("layout row helpers should be readable");
    let tests = fs::read_to_string(manifest_dir.join("src/gui/layout_core/row_helpers/tests.rs"))
        .expect("layout row helper tests should be readable");

    assert!(
        root.contains("mod fitting;")
            && root.contains("mod rects;")
            && root.contains("mod widths;")
            && root.contains("#[path = \"row_helpers/tests.rs\"]")
            && !root.contains("fn fixed_width_row_rects_start_places_items_from_left_edge"),
        "layout row helper root should re-export focused geometry modules while behavior tests stay delegated"
    );
    assert!(
        tests.contains("fn fixed_width_row_rects_start_places_items_from_left_edge")
            && tests.contains("fn visible_suffix_widths_normalizes_negative_dimensions")
            && tests.contains(
                "fn fixed_width_item_extent_for_available_width_fits_items_after_reserved_gaps"
            ),
        "layout row helper behavior coverage should live in row_helpers/tests.rs"
    );
}

#[test]
fn layout_axis_helpers_keep_orientation_and_tests_focused() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let axis = fs::read_to_string(manifest_dir.join("src/gui/layout_core/engine/helpers/axis.rs"))
        .expect("layout axis helper should be readable");
    let tests =
        fs::read_to_string(manifest_dir.join("src/gui/layout_core/engine/helpers/axis/tests.rs"))
            .expect("layout axis helper tests should be readable");

    assert!(
        axis.contains("enum LayoutAxis")
            && axis.contains("fn main_extent")
            && axis.contains("fn overflow_flags")
            && axis.contains("#[path = \"axis/tests.rs\"]")
            && !axis.contains("fn layout_axis_resolves_main_and_cross_extents"),
        "layout axis orientation helpers should live in axis.rs while behavior tests stay delegated"
    );
    assert!(
        tests.contains("fn layout_axis_resolves_main_and_cross_extents")
            && tests.contains("fn layout_axis_reports_overflow_direction"),
        "layout axis helper behavior coverage should live in axis/tests.rs"
    );
}

#[test]
fn layout_engine_root_tests_stay_grouped_by_behavior_concern() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let root = fs::read_to_string(manifest_dir.join("src/gui/layout_core/engine/tests.rs"))
        .expect("layout engine test root should be readable");
    let layout =
        fs::read_to_string(manifest_dir.join("src/gui/layout_core/engine/tests/layout.rs"))
            .expect("layout engine layout tests should be readable");
    let layout_row =
        fs::read_to_string(manifest_dir.join("src/gui/layout_core/engine/tests/layout/row.rs"))
            .expect("layout engine row layout tests should be readable");
    let layout_switch =
        fs::read_to_string(manifest_dir.join("src/gui/layout_core/engine/tests/layout/switch.rs"))
            .expect("layout engine switch layout tests should be readable");
    let layout_flow =
        fs::read_to_string(manifest_dir.join("src/gui/layout_core/engine/tests/layout/flow.rs"))
            .expect("layout engine flow layout tests should be readable");
    let scroll =
        fs::read_to_string(manifest_dir.join("src/gui/layout_core/engine/tests/scroll.rs"))
            .expect("layout engine scroll tests should be readable");
    let diagnostics =
        fs::read_to_string(manifest_dir.join("src/gui/layout_core/engine/tests/diagnostics.rs"))
            .expect("layout engine diagnostic tests should be readable");
    let debug = fs::read_to_string(manifest_dir.join("src/gui/layout_core/engine/tests/debug.rs"))
        .expect("layout engine debug tests should be readable");
    let scratch =
        fs::read_to_string(manifest_dir.join("src/gui/layout_core/engine/tests/scratch.rs"))
            .expect("layout engine scratch test root should be readable");
    let scratch_reuse =
        fs::read_to_string(manifest_dir.join("src/gui/layout_core/engine/tests/scratch/reuse.rs"))
            .expect("layout engine scratch reuse tests should be readable");
    let scratch_pruning = fs::read_to_string(
        manifest_dir.join("src/gui/layout_core/engine/tests/scratch/pruning.rs"),
    )
    .expect("layout engine scratch pruning tests should be readable");
    let scratch_dirty =
        fs::read_to_string(manifest_dir.join("src/gui/layout_core/engine/tests/scratch/dirty.rs"))
            .expect("layout engine scratch dirty tests should be readable");
    let scratch_fixtures = fs::read_to_string(
        manifest_dir.join("src/gui/layout_core/engine/tests/scratch/fixtures.rs"),
    )
    .expect("layout engine scratch fixtures should be readable");

    assert!(
        root.contains("mod layout;")
            && root.contains("mod scroll;")
            && root.contains("mod diagnostics;")
            && root.contains("mod debug;")
            && root.contains("#[path = \"tests/scratch.rs\"]")
            && !root.contains("fn layout_tree_is_deterministic")
            && !root.contains("fn scroll_offset_is_clamped"),
        "layout engine test root should index focused behavior groups instead of owning all core layout cases"
    );
    assert!(
        scratch.contains("mod reuse;")
            && scratch.contains("mod pruning;")
            && scratch.contains("mod dirty;")
            && scratch.contains("mod fixtures;")
            && !scratch.contains("fn layout_engine_reuses_scratch_maps_between_passes"),
        "layout engine scratch tests should index focused scratch behavior groups instead of owning all cases"
    );
    assert!(
        layout.contains("mod row;")
            && layout.contains("mod switch;")
            && layout.contains("mod flow;")
            && !layout.contains("fn fill_children_redistribute_after_constrained_child_clamps")
            && scroll.contains("fn scroll_offset_is_clamped_and_reported")
            && diagnostics.contains("fn contradictory_constraints_emit_diagnostic")
            && debug.contains("fn debug_primitives_are_emitted_when_enabled"),
        "layout engine tests should stay grouped by layout, scroll, diagnostics, and debug concerns"
    );
    assert!(
        layout_row.contains("fn layout_tree_is_deterministic_for_same_input")
            && layout_row.contains("fn fill_children_redistribute_after_constrained_child_clamps")
            && layout_switch.contains("fn switch_layout_selects_breakpoint_child")
            && layout_flow.contains("fn wrap_layout_moves_items_to_next_line")
            && layout_flow.contains("fn grid_layout_places_items_by_row_and_column"),
        "layout engine layout tests should stay grouped by row/fill, switch, and flow container behavior"
    );
    assert!(
        scratch_reuse.contains("fn layout_engine_reuses_scratch_maps_between_passes")
            && scratch_pruning.contains("fn layout_engine_prunes_stale_measure_cache_versions")
            && scratch_dirty.contains(
                "fn dirty_subtree_invalidates_virtual_metrics_cache_for_whole_marked_set"
            )
            && scratch_fixtures.contains("fn fixed_virtualized_root"),
        "layout engine scratch tests should stay grouped by reuse, pruning, dirty-subtree, and fixture concerns"
    );
}

#[test]
fn runtime_surface_nodes_use_named_parts_for_public_tree_construction() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let source = fs::read_to_string(manifest_dir.join("src/runtime/surface/node.rs"))
        .expect("runtime surface node module should be readable");
    let builders = fs::read_to_string(manifest_dir.join("src/runtime/surface/builders.rs"))
        .expect("runtime surface builders should be readable");
    let container_builders =
        fs::read_to_string(manifest_dir.join("src/runtime/surface/builders/container.rs"))
            .expect("runtime surface container builders should be readable");
    let leaf_builders =
        fs::read_to_string(manifest_dir.join("src/runtime/surface/builders/leaf.rs"))
            .expect("runtime surface leaf builders should be readable");
    let overlay_builders =
        fs::read_to_string(manifest_dir.join("src/runtime/surface/builders/overlay.rs"))
            .expect("runtime surface overlay builders should be readable");
    let surface = fs::read_to_string(manifest_dir.join("src/runtime/surface.rs"))
        .expect("runtime surface module should be readable");
    let runtime =
        fs::read_to_string(manifest_dir.join("src/runtime/mod.rs")).expect("runtime module");

    for (parts, from_parts, wrapper) in [
        (
            "pub struct SurfaceChildParts<Message>",
            "pub fn from_parts(parts: SurfaceChildParts<Message>) -> Self",
            "Self::from_parts(SurfaceChildParts {",
        ),
        (
            "pub struct SurfaceContainerParts<Message>",
            "pub fn from_parts(parts: SurfaceContainerParts<Message>) -> Self",
            "Self::from_parts(SurfaceContainerParts {",
        ),
    ] {
        assert!(
            source.contains(parts) && source.contains(from_parts) && source.contains(wrapper),
            "runtime surface nodes should expose named parts and compatibility wrappers for {parts}"
        );
    }
    assert!(
        builders.contains("mod container;")
            && builders.contains("mod leaf;")
            && builders.contains("mod overlay;")
            && !builders
                .contains("pub fn container_from_parts(parts: SurfaceContainerParts<Message>)")
            && !builders.contains("pub fn widget(")
            && !builders.contains("pub fn overlay_panel(")
            && container_builders.contains(
                "pub fn container_from_parts(parts: SurfaceContainerParts<Message>) -> Self"
            )
            && container_builders.contains("pub fn virtual_scroll_area(")
            && container_builders.contains("fn scroll_area_with_virtualization(")
            && leaf_builders.contains("pub fn widget(")
            && leaf_builders.contains("pub fn custom_widget_box(")
            && leaf_builders.contains("pub fn static_widget(")
            && overlay_builders.contains("pub fn overlay_panel(")
            && overlay_builders.contains("pub fn overlay_marker(")
            && surface.contains("SurfaceChildParts")
            && surface.contains("SurfaceContainerParts")
            && runtime.contains("SurfaceChildParts")
            && runtime.contains("SurfaceContainerParts"),
        "runtime surface builders should stay focused while named parts remain publicly available"
    );
}

#[test]
fn runtime_surface_focus_order_keeps_collection_and_tests_focused() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let focus = fs::read_to_string(manifest_dir.join("src/runtime/surface/focus.rs"))
        .expect("runtime surface focus order helper should be readable");
    let tests = fs::read_to_string(manifest_dir.join("src/runtime/surface/focus/tests.rs"))
        .expect("runtime surface focus order tests should be readable");

    assert!(
        focus.contains("pub fn keyboard_focus_order_into")
            && focus.contains("pub fn keyboard_focus_order(&self)")
            && focus.contains("fn append_keyboard_focus_order")
            && focus.contains("#[path = \"focus/tests.rs\"]")
            && !focus.contains("fn keyboard_focus_order_collects_only_keyboard_focusable_widgets"),
        "runtime surface focus ordering should live in surface/focus.rs while behavior tests stay delegated"
    );
    assert!(
        tests.contains("fn keyboard_focus_order_collects_only_keyboard_focusable_widgets")
            && tests.contains("fn keyboard_focus_order_into_reuses_existing_storage"),
        "runtime surface focus behavior coverage should live in surface/focus/tests.rs"
    );
}

#[test]
fn runtime_bridge_app_contract_stays_in_focused_module() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let bridge = fs::read_to_string(manifest_dir.join("src/runtime/bridge.rs"))
        .expect("runtime bridge module should be readable");
    let app = fs::read_to_string(manifest_dir.join("src/runtime/bridge/app.rs"))
        .expect("runtime bridge app contract module should be readable");
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
}

#[test]
fn runtime_animation_activity_keeps_policy_and_tests_focused() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let animation = fs::read_to_string(manifest_dir.join("src/runtime/bridge/animation.rs"))
        .expect("runtime animation activity module should be readable");
    let tests = fs::read_to_string(manifest_dir.join("src/runtime/bridge/animation/tests.rs"))
        .expect("runtime animation activity tests should be readable");
    let bridge = fs::read_to_string(manifest_dir.join("src/runtime/bridge.rs"))
        .expect("runtime bridge module should be readable");

    assert!(
        bridge.contains("mod animation;")
            && bridge.contains("pub use animation::RuntimeAnimationActivity;"),
        "runtime bridge root should re-export animation activity from the focused child module"
    );
    assert!(
        animation.contains("pub struct RuntimeAnimationActivity")
            && animation.contains("pub const fn merge(self, other: Self) -> Self")
            && animation.contains("const fn merge_target_fps(")
            && animation.contains("#[path = \"animation/tests.rs\"]")
            && !animation.contains(
                "fn runtime_animation_activity_keeps_frame_messages_bound_to_paint_frames"
            ),
        "runtime animation policy should live in bridge/animation.rs while behavior tests stay delegated"
    );
    assert!(
        tests.contains("fn runtime_animation_activity_keeps_frame_messages_bound_to_paint_frames")
            && tests.contains("fn runtime_animation_activity_merge_keeps_uncapped_source_uncapped"),
        "runtime animation behavior coverage should live in bridge/animation/tests.rs"
    );
}

#[test]
fn declarative_runtime_bridges_use_named_parts_for_host_closures() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let message =
        fs::read_to_string(manifest_dir.join("src/runtime/bridge/declarative/message.rs"))
            .expect("declarative message bridge should be readable");
    let command =
        fs::read_to_string(manifest_dir.join("src/runtime/bridge/declarative/command.rs"))
            .expect("declarative command bridge should be readable");
    let bridge = fs::read_to_string(manifest_dir.join("src/runtime/bridge.rs"))
        .expect("runtime bridge module should be readable");
    let runtime =
        fs::read_to_string(manifest_dir.join("src/runtime/mod.rs")).expect("runtime module");

    for (source, parts, from_parts, wrapper) in [
        (
            message.as_str(),
            "pub struct DeclarativeRuntimeBridgeParts",
            "pub fn from_parts(parts: DeclarativeRuntimeBridgeParts<State, Project, Reduce>) -> Self",
            "Self::from_parts(DeclarativeRuntimeBridgeParts {",
        ),
        (
            message.as_str(),
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

#[test]
fn scroll_commands_use_named_parts_for_reveal_requests() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let command = fs::read_to_string(manifest_dir.join("src/runtime/command.rs"))
        .expect("runtime command module should be readable");
    let command_repaint = fs::read_to_string(manifest_dir.join("src/runtime/command/repaint.rs"))
        .expect("runtime command repaint model should be readable");
    let command_scroll = fs::read_to_string(manifest_dir.join("src/runtime/command/scroll.rs"))
        .expect("runtime command scroll reveal models should be readable");
    let constructors = fs::read_to_string(manifest_dir.join("src/runtime/command/constructors.rs"))
        .expect("runtime command constructors should be readable");
    let scroll_constructors =
        fs::read_to_string(manifest_dir.join("src/runtime/command/constructors/scroll.rs"))
            .expect("runtime command scroll constructors should be readable");
    let update_context =
        fs::read_to_string(manifest_dir.join("src/application/runtime/update_context.rs"))
            .expect("application update context should be readable");
    let update_context_surface =
        fs::read_to_string(manifest_dir.join("src/application/runtime/update_context/surface.rs"))
            .expect("application update context surface helpers should be readable");
    let runtime =
        fs::read_to_string(manifest_dir.join("src/runtime/mod.rs")).expect("runtime module");
    let lib = fs::read_to_string(manifest_dir.join("src/lib.rs"))
        .expect("library module should be readable");

    assert!(
        command.contains("mod repaint;")
            && command.contains("mod scroll;")
            && command.contains("pub use repaint::RepaintScope;")
            && command
                .contains("pub use scroll::{ScrollFixedRowIntoViewParts, ScrollIntoViewParts};")
            && !command.contains("pub enum RepaintScope")
            && !command.contains("pub struct ScrollIntoViewParts")
            && command_repaint.contains("pub enum RepaintScope")
            && command_repaint.contains("pub const fn merge")
            && command_scroll.contains("pub struct ScrollIntoViewParts")
            && command_scroll.contains("pub struct ScrollFixedRowIntoViewParts"),
        "runtime command repaint and scroll reveal models should stay delegated while public exports remain stable"
    );
    assert!(
        constructors.contains("mod scroll;")
            && !constructors.contains(
                "pub const fn scroll_into_view_from_parts(parts: ScrollIntoViewParts) -> Self"
            )
            && scroll_constructors.contains(
                "pub const fn scroll_into_view_from_parts(parts: ScrollIntoViewParts) -> Self"
            )
            && scroll_constructors.contains("pub const fn scroll_fixed_row_into_view_from_parts")
            && scroll_constructors
                .contains("Self::scroll_into_view_from_parts(ScrollIntoViewParts {")
            && scroll_constructors.contains(
                "Self::scroll_fixed_row_into_view_from_parts(ScrollFixedRowIntoViewParts {"
            ),
        "scroll command constructors should stay in their focused module and delegate positional helpers through named request parts"
    );
    assert!(
        update_context.contains("mod surface;")
            && update_context_surface.contains(
                "pub fn scroll_into_view_from_parts(&mut self, parts: ScrollIntoViewParts)"
            )
            && update_context_surface.contains("pub fn scroll_fixed_row_into_view_from_parts")
            && runtime.contains("ScrollIntoViewParts")
            && runtime.contains("ScrollFixedRowIntoViewParts")
            && lib.contains("ScrollIntoViewParts")
            && lib.contains("ScrollFixedRowIntoViewParts"),
        "scroll reveal named request parts should be available through runtime and prelude paths"
    );
}

#[test]
fn update_context_keeps_followup_command_groups_in_focused_modules() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let root = fs::read_to_string(manifest_dir.join("src/application/runtime/update_context.rs"))
        .expect("application update context root should be readable");
    let commands =
        fs::read_to_string(manifest_dir.join("src/application/runtime/update_context/commands.rs"))
            .expect("application update context command helpers should be readable");
    let platform =
        fs::read_to_string(manifest_dir.join("src/application/runtime/update_context/platform.rs"))
            .expect("application update context platform helpers should be readable");
    let tasks =
        fs::read_to_string(manifest_dir.join("src/application/runtime/update_context/tasks.rs"))
            .expect("application update context task helpers should be readable");
    let surface =
        fs::read_to_string(manifest_dir.join("src/application/runtime/update_context/surface.rs"))
            .expect("application update context surface helpers should be readable");

    for required in [
        "mod commands;",
        "mod platform;",
        "mod surface;",
        "mod tasks;",
        "pub struct UpdateContext<Message>",
        "fn into_command(self) -> Command<Message>",
    ] {
        assert!(
            root.contains(required),
            "update context root should own the queue and delegate `{required}`"
        );
    }
    assert!(
        commands.contains("pub fn request_repaint")
            && commands.contains("pub fn request_paint_only")
            && commands.contains("pub fn repaint")
            && commands.contains("pub fn after")
            && commands.contains("pub fn exit"),
        "basic command and repaint helpers should live in update_context/commands.rs"
    );
    assert!(
        platform.contains("pub fn begin_external_drag")
            && platform.contains("pub fn platform_request")
            && platform.contains("pub fn pick_folder")
            && platform.contains("pub fn confirm"),
        "platform and external-drag helpers should live in update_context/platform.rs"
    );
    assert!(
        tasks.contains("pub fn spawn<Output>")
            && tasks.contains("pub fn spawn_cancellable")
            && tasks.contains("pub fn spawn_latest")
            && tasks.contains("pub fn spawn_resource"),
        "runtime task and resource helpers should live in update_context/tasks.rs"
    );
    assert!(
        surface.contains("pub fn focus")
            && surface.contains("pub fn scroll_to")
            && surface.contains("pub fn scroll_into_view_from_parts")
            && surface.contains("pub fn scroll_fixed_row_into_view_from_parts"),
        "focus and scroll helpers should live in update_context/surface.rs"
    );
}

#[test]
fn application_id_generation_keeps_policy_and_tests_focused() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let ids = fs::read_to_string(manifest_dir.join("src/application/ids.rs"))
        .expect("application id generation module should be readable");
    let tests = fs::read_to_string(manifest_dir.join("src/application/ids/tests.rs"))
        .expect("application id generation tests should be readable");

    assert!(
        ids.contains("pub(in crate::application) struct IdGenerator")
            && ids.contains("enum ReservedIds")
            && ids.contains("fn reserved_id_range(reserved: &[NodeId])")
            && ids.contains("pub(in crate::application) fn scoped_key_id")
            && ids.contains("#[path = \"ids/tests.rs\"]")
            && !ids.contains("fn id_generator_skips_dense_reserved_runs_after_collision"),
        "application id allocation should live in application/ids.rs while behavior tests stay delegated"
    );
    assert!(
        tests.contains("fn id_generator_skips_dense_reserved_runs_after_collision")
            && tests.contains("fn id_generator_keeps_sorted_reserved_ids_for_small_sets")
            && tests.contains("fn id_generator_skips_probing_after_reserved_range_is_exhausted"),
        "application id generation behavior coverage should live in application/ids/tests.rs"
    );
}

#[test]
fn application_task_helpers_keep_cancellation_completion_and_latest_state_focused() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let root = fs::read_to_string(manifest_dir.join("src/application/runtime/task.rs"))
        .expect("application runtime task root should be readable");
    let cancellation =
        fs::read_to_string(manifest_dir.join("src/application/runtime/task/cancellation.rs"))
            .expect("application runtime cancellation token module should be readable");
    let completion =
        fs::read_to_string(manifest_dir.join("src/application/runtime/task/completion.rs"))
            .expect("application runtime task completion module should be readable");
    let latest = fs::read_to_string(manifest_dir.join("src/application/runtime/task/latest.rs"))
        .expect("application runtime latest task module should be readable");
    let keyed_latest =
        fs::read_to_string(manifest_dir.join("src/application/runtime/task/keyed_latest.rs"))
            .expect("application runtime keyed latest task module should be readable");
    let runtime = fs::read_to_string(manifest_dir.join("src/application/runtime.rs"))
        .expect("application runtime module should be readable");

    for required in [
        "mod cancellation;",
        "mod completion;",
        "mod keyed_latest;",
        "mod latest;",
        "pub use cancellation::CancellationToken;",
        "pub use completion::{KeyedTaskCompletion, TaskCompletion, TaskTicket};",
        "pub use keyed_latest::KeyedLatestTasks;",
        "pub use latest::LatestTask;",
    ] {
        assert!(
            root.contains(required),
            "application runtime task root should delegate `{required}`"
        );
    }
    assert!(
        !root.contains("pub struct CancellationToken")
            && !root.contains("pub struct TaskCompletion")
            && !root.contains("pub struct LatestTask")
            && !root.contains("pub struct KeyedLatestTasks"),
        "application runtime task root should re-export task helpers without owning implementation"
    );
    assert!(
        cancellation.contains("pub struct CancellationToken")
            && cancellation.contains("pub fn cancel(&self)")
            && cancellation.contains("pub fn is_cancelled(&self) -> bool")
            && cancellation.contains("#[path = \"cancellation/tests.rs\"]")
            && !cancellation.contains("fn cancellation_token_is_shared_across_clones"),
        "task cancellation token should live in application/runtime/task/cancellation.rs while behavior tests stay delegated"
    );
    assert!(
        completion.contains("pub struct TaskTicket")
            && completion.contains("pub struct TaskCompletion<Output>")
            && completion.contains("pub struct KeyedTaskCompletion<Key, Output>"),
        "task tickets and completion DTOs should live in application/runtime/task/completion.rs"
    );
    assert!(
        latest.contains("pub struct LatestTask")
            && latest.contains("pub fn begin(&mut self) -> TaskTicket")
            && latest.contains("pub fn finish(&mut self, ticket: TaskTicket) -> bool")
            && latest.contains("#[path = \"latest/tests.rs\"]")
            && !latest.contains("fn latest_task_rejects_stale_tickets_after_newer_begin"),
        "single-resource latest task state should live in application/runtime/task/latest.rs while behavior tests stay delegated"
    );
    assert!(
        keyed_latest.contains("pub struct KeyedLatestTasks<Key>")
            && keyed_latest.contains("pub fn begin(&mut self, key: Key) -> TaskTicket")
            && keyed_latest.contains("pub fn remove(&mut self, key: &Key) -> Option<LatestTask>")
            && keyed_latest.contains("#[path = \"keyed_latest/tests.rs\"]")
            && !keyed_latest.contains("fn keyed_latest_tasks_reject_stale_tickets_per_key"),
        "keyed latest task registry should live in application/runtime/task/keyed_latest.rs while behavior tests stay delegated"
    );
    assert!(
        runtime.contains("CancellationToken")
            && runtime.contains("KeyedLatestTasks")
            && runtime.contains("TaskCompletion")
            && runtime.contains("TaskTicket"),
        "application runtime facade should keep task helpers available through the public runtime path"
    );
}

#[test]
fn application_runtime_timer_lane_keeps_worker_policy_and_tests_focused() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let timer = fs::read_to_string(manifest_dir.join("src/application/runtime/timer.rs"))
        .expect("application runtime timer root should be readable");
    let lane = fs::read_to_string(manifest_dir.join("src/application/runtime/timer/lane.rs"))
        .expect("application runtime timer lane should be readable");
    let tests =
        fs::read_to_string(manifest_dir.join("src/application/runtime/timer/lane/tests.rs"))
            .expect("application runtime timer lane tests should be readable");

    assert!(
        timer.contains("mod lane;")
            && timer.contains("mod queue;")
            && timer.contains("mod worker;")
            && timer.contains("pub(super) use lane::TimerLane;"),
        "application runtime timer root should delegate lane, queue, and worker responsibilities"
    );
    assert!(
        lane.contains("pub(in crate::application::runtime) struct TimerLane<Message>")
            && lane.contains("pub(in crate::application::runtime) fn schedule(")
            && lane.contains("pub(in crate::application::runtime) fn schedule_interval(")
            && lane.contains("#[path = \"lane/tests.rs\"]")
            && !lane.contains("fn timer_lane_rejects_work_when_worker_is_unavailable"),
        "timer lane worker policy should live in timer/lane.rs while behavior tests stay delegated"
    );
    assert!(
        tests.contains("fn timer_lane_rejects_work_when_worker_is_unavailable"),
        "timer lane behavior coverage should live in timer/lane/tests.rs"
    );
}

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

#[test]
fn text_input_state_keeps_models_selection_navigation_and_editing_focused() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let model = fs::read_to_string(manifest_dir.join("src/widgets/primitives/text_input/model.rs"))
        .expect("text input model root should be readable");
    let selection = fs::read_to_string(
        manifest_dir.join("src/widgets/primitives/text_input/model/selection.rs"),
    )
    .expect("text input selection model should be readable");
    let navigation = fs::read_to_string(
        manifest_dir.join("src/widgets/primitives/text_input/model/navigation.rs"),
    )
    .expect("text input navigation model should be readable");
    let editing =
        fs::read_to_string(manifest_dir.join("src/widgets/primitives/text_input/model/editing.rs"))
            .expect("text input editing model should be readable");
    let editing_command = fs::read_to_string(
        manifest_dir.join("src/widgets/primitives/text_input/model/editing/command.rs"),
    )
    .expect("text input edit command model should be readable");
    let editing_mutation = fs::read_to_string(
        manifest_dir.join("src/widgets/primitives/text_input/model/editing/mutation.rs"),
    )
    .expect("text input edit mutation model should be readable");
    let tests = fs::read_to_string(manifest_dir.join("src/widgets/primitives/text_input/tests.rs"))
        .expect("text input behavior test root should be readable");
    let widget_tests =
        fs::read_to_string(manifest_dir.join("src/widgets/primitives/text_input/tests/widget.rs"))
            .expect("text input widget interaction tests should be readable");
    let state_tests =
        fs::read_to_string(manifest_dir.join("src/widgets/primitives/text_input/tests/state.rs"))
            .expect("text input state behavior tests should be readable");

    for required in ["mod editing;", "mod navigation;", "mod selection;"] {
        assert!(
            model.contains(required),
            "text input model root should delegate `{required}`"
        );
    }
    assert!(
        model.contains("pub struct TextInputProps")
            && model.contains("pub struct TextInputState")
            && model.contains("pub struct TextInputEditResult")
            && model.contains("pub fn from_value")
            && !model.contains("TextEditCommand")
            && !model.contains("WidgetKey"),
        "text input model root should keep public state definitions separate from command handling"
    );
    assert!(
        selection.contains("pub fn selected_text")
            && selection.contains("pub fn selection_range")
            && selection.contains("pub fn has_selection"),
        "text input selection queries should live in model/selection.rs"
    );
    assert!(
        navigation.contains("pub fn set_caret")
            && navigation.contains("fn move_left")
            && navigation.contains("fn move_right"),
        "text input caret movement should live in model/navigation.rs"
    );
    assert!(
        editing.contains("mod command;")
            && editing.contains("mod mutation;")
            && !editing.contains("pub fn apply_edit_command")
            && !editing.contains("pub fn insert_text"),
        "text input editing root should delegate command dispatch and mutation mechanics"
    );
    assert!(
        editing_command.contains("pub fn apply_edit_command")
            && editing_command.contains("pub fn apply_key")
            && editing_command.contains("TextEditCommand")
            && editing_command.contains("WidgetKey")
            && !editing_mutation.contains("TextEditCommand")
            && !editing_mutation.contains("WidgetKey"),
        "text input edit command handling should live in model/editing/command.rs"
    );
    assert!(
        editing_mutation.contains("pub fn insert_text")
            && editing_mutation.contains("pub fn replace_selection")
            && editing_mutation.contains("pub(crate) fn delete_selected_text")
            && editing_mutation.contains("byte_index_for_char")
            && !editing_command.contains("byte_index_for_char"),
        "text input mutation mechanics should live in model/editing/mutation.rs"
    );
    assert!(
        tests.contains("mod widget;")
            && tests.contains("mod state;")
            && !tests.contains("fn text_input_editing_emits_changed_and_submitted_messages")
            && !tests.contains("fn text_input_state_applies_backend_neutral_editing_commands"),
        "text input behavior test root should index focused widget and state groups instead of owning all cases"
    );
    assert!(
        widget_tests.contains("fn text_input_editing_emits_changed_and_submitted_messages")
            && widget_tests
                .contains("fn text_input_pointer_drag_extends_selection_including_caret_character")
            && state_tests.contains("fn text_input_state_applies_backend_neutral_editing_commands")
            && state_tests.contains("fn text_input_state_can_clear_or_delete_active_selection"),
        "text input behavior tests should stay grouped by widget interaction and state editing concerns"
    );
}

#[test]
fn retained_invalidation_primitives_stay_in_focused_modules() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let root = fs::read_to_string(manifest_dir.join("src/gui/invalidation.rs"))
        .expect("invalidation root should be readable");
    let tests = fs::read_to_string(manifest_dir.join("src/gui/invalidation/tests.rs"))
        .expect("invalidation behavior tests should be readable");
    let mask = fs::read_to_string(manifest_dir.join("src/gui/invalidation/mask.rs"))
        .expect("invalidation mask module should be readable");
    let retained_mask =
        fs::read_to_string(manifest_dir.join("src/gui/invalidation/retained_mask.rs"))
            .expect("retained mask module should be readable");
    let segment = fs::read_to_string(manifest_dir.join("src/gui/invalidation/segment.rs"))
        .expect("retained segment module should be readable");

    for required in [
        "mod mask;",
        "mod retained_mask;",
        "mod segment;",
        "#[path = \"invalidation/tests.rs\"]",
        "pub use mask::InvalidationMask;",
        "pub use retained_mask::RetainedSegmentMask;",
    ] {
        assert!(
            root.contains(required),
            "invalidation root should delegate `{required}`"
        );
    }
    assert!(
        root.contains("RetainedSegmentPlan")
            && root.contains("RetainedSegmentRevisions")
            && !root.contains("pub struct InvalidationMask")
            && !root.contains("pub struct RetainedSegmentMask")
            && !root.contains("pub struct RetainedSegmentPlan")
            && !root.contains("fn invalidation_mask_clips_to_valid_bits"),
        "invalidation root should re-export public primitives and delegate behavior tests without owning implementations"
    );
    assert!(
        tests.contains("fn invalidation_mask_clips_to_valid_bits")
            && tests.contains("fn retained_segment_plan_names_groups_and_bumps_revisions"),
        "invalidation behavior coverage should live in gui/invalidation/tests.rs"
    );
    assert!(
        mask.contains("pub struct InvalidationMask")
            && mask.contains("pub const fn from_bits")
            && mask.contains("pub fn insert"),
        "raw invalidation bit operations should live in invalidation/mask.rs"
    );
    assert!(
        retained_mask.contains("pub struct RetainedSegmentMask")
            && retained_mask.contains("pub const fn requires_static_rebuild")
            && retained_mask.contains("pub const fn requires_overlay_rebuild"),
        "typed retained segment masks should live in invalidation/retained_mask.rs"
    );
    assert!(
        segment.contains("pub struct RetainedSegmentPlan")
            && segment.contains("pub struct RetainedSegmentRevisions")
            && segment.contains("pub enum RetainedSegmentKind")
            && segment.contains("pub fn bump_revisions"),
        "retained segment metadata, plans, and revisions should live in invalidation/segment.rs"
    );
}

#[test]
fn retained_cache_support_keeps_fingerprint_storage_and_tests_focused() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let fingerprint = fs::read_to_string(manifest_dir.join("src/gui/fingerprint.rs"))
        .expect("stable fingerprint source should be readable");
    let fingerprint_tests = fs::read_to_string(manifest_dir.join("src/gui/fingerprint/tests.rs"))
        .expect("stable fingerprint tests should be readable");
    let retained = fs::read_to_string(manifest_dir.join("src/gui/retained.rs"))
        .expect("retained storage source should be readable");
    let retained_tests = fs::read_to_string(manifest_dir.join("src/gui/retained/tests.rs"))
        .expect("retained storage tests should be readable");

    assert!(
        fingerprint.contains("pub struct StableFingerprint")
            && fingerprint.contains("pub fn mix_rgba8")
            && fingerprint.contains("#[path = \"fingerprint/tests.rs\"]")
            && !fingerprint.contains("fn fingerprints_are_stable_for_identical_inputs"),
        "stable fingerprint mixing should live in gui/fingerprint.rs while behavior tests stay delegated"
    );
    assert!(
        fingerprint_tests.contains("fn fingerprints_are_stable_for_identical_inputs")
            && fingerprint_tests.contains("fn color_channels_affect_fingerprint"),
        "fingerprint behavior coverage should live in gui/fingerprint/tests.rs"
    );
    assert!(
        retained.contains("pub struct RetainedVec")
            && retained.contains("pub fn make_mut")
            && retained.contains("#[path = \"retained/tests.rs\"]")
            && !retained.contains("fn retained_vec_clones_share_storage_until_mutation"),
        "retained vector storage should live in gui/retained.rs while behavior tests stay delegated"
    );
    assert!(
        retained_tests.contains("fn retained_vec_clones_share_storage_until_mutation"),
        "retained storage behavior coverage should live in gui/retained/tests.rs"
    );
}

#[test]
fn text_input_primitive_keeps_surface_builders_focused() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let root = fs::read_to_string(manifest_dir.join("src/widgets/primitives/text_input.rs"))
        .expect("text-input primitive root should be readable");
    let builders =
        fs::read_to_string(manifest_dir.join("src/widgets/primitives/text_input/builders.rs"))
            .expect("text-input primitive builders should be readable");

    assert!(
        root.contains("mod builders;")
            && root.contains("pub struct TextInputWidget")
            && root.contains("impl Widget for TextInputWidget")
            && !root.contains("impl<Message> SurfaceNode<Message>")
            && !root.contains("impl<Message> WidgetMessageMapper<Message>"),
        "text-input primitive root should own widget behavior and delegate runtime builders"
    );
    assert!(
        builders.contains("impl<Message> SurfaceNode<Message>")
            && builders.contains("pub fn text_input(")
            && builders.contains("pub fn text_input_mapped(")
            && builders.contains("impl<Message> WidgetMessageMapper<Message>"),
        "text-input runtime builder helpers should live in text_input/builders.rs"
    );
}

#[test]
fn input_key_identity_and_keypress_state_stay_focused() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let input = fs::read_to_string(manifest_dir.join("src/gui/input.rs"))
        .expect("input module should be readable");
    let key = fs::read_to_string(manifest_dir.join("src/gui/input/key.rs"))
        .expect("input key module should be readable");
    let press = fs::read_to_string(manifest_dir.join("src/gui/input/key/press.rs"))
        .expect("input keypress module should be readable");
    let press_tests = fs::read_to_string(manifest_dir.join("src/gui/input/key/press/tests.rs"))
        .expect("input keypress behavior tests should be readable");
    let pointer = fs::read_to_string(manifest_dir.join("src/gui/input/pointer.rs"))
        .expect("input pointer module should be readable");
    let pointer_tests = fs::read_to_string(manifest_dir.join("src/gui/input/pointer/tests.rs"))
        .expect("input pointer behavior tests should be readable");

    assert!(
        input.contains("pub use key::{KeyCode, KeyPress};")
            && input.contains("pub use pointer::logical_point_to_u16_coords;")
            && key.contains("mod press;")
            && key.contains("pub use press::KeyPress;"),
        "input facade should preserve key and pointer exports through focused child modules"
    );
    assert!(
        key.contains("pub enum KeyCode")
            && !key.contains("pub struct KeyPress")
            && press.contains("pub struct KeyPress")
            && press.contains("pub const fn with_command")
            && press.contains("#[path = \"press/tests.rs\"]")
            && !press.contains("fn keypress_constructors_preserve_modifier_state"),
        "key identity should stay in key.rs while modifier-bearing keypress state lives in key/press.rs with behavior tests delegated"
    );
    assert!(
        press_tests.contains("fn keypress_constructors_preserve_modifier_state"),
        "keypress behavior coverage should live in input/key/press/tests.rs"
    );
    assert!(
        pointer.contains("pub fn logical_point_to_u16_coords")
            && pointer.contains("#[path = \"pointer/tests.rs\"]")
            && !pointer.contains("fn logical_point_to_u16_coords_clamps_and_rounds"),
        "pointer coordinate conversion should live in input/pointer.rs with behavior tests delegated"
    );
    assert!(
        pointer_tests.contains("fn logical_point_to_u16_coords_clamps_and_rounds"),
        "pointer behavior coverage should live in input/pointer/tests.rs"
    );
}

#[test]
fn shortcut_primitives_stay_in_resolution_gesture_and_layer_modules() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let root = fs::read_to_string(manifest_dir.join("src/gui/shortcuts.rs"))
        .expect("shortcut root should be readable");
    let tests = fs::read_to_string(manifest_dir.join("src/gui/shortcuts/tests.rs"))
        .expect("shortcut behavior tests should be readable");
    let resolution = fs::read_to_string(manifest_dir.join("src/gui/shortcuts/resolution.rs"))
        .expect("shortcut resolution module should be readable");
    let gesture = fs::read_to_string(manifest_dir.join("src/gui/shortcuts/gesture.rs"))
        .expect("shortcut gesture module should be readable");
    let layer = fs::read_to_string(manifest_dir.join("src/gui/shortcuts/layer.rs"))
        .expect("shortcut layer module should be readable");

    for required in [
        "mod gesture;",
        "mod layer;",
        "mod resolution;",
        "#[path = \"shortcuts/tests.rs\"]",
        "pub use gesture::{ShortcutGesture, ShortcutModifier};",
        "pub use layer::{ShortcutBinding, ShortcutLayer};",
        "pub use resolution::ShortcutResolution;",
    ] {
        assert!(
            root.contains(required),
            "shortcut root should delegate `{required}`"
        );
    }
    assert!(
        !root.contains("pub struct ShortcutResolution")
            && !root.contains("pub struct ShortcutLayer")
            && !root.contains("pub struct ShortcutGesture")
            && !root.contains("fn shortcut_layer_resolves_actions_and_modal_misses"),
        "shortcut root should re-export public primitives and delegate behavior tests instead of owning implementations"
    );
    assert!(
        tests.contains("fn shortcut_resolution_unhandled_has_no_action_or_chord")
            && tests.contains("fn shortcut_layer_resolves_actions_and_modal_misses"),
        "shortcut behavior coverage should live in gui/shortcuts/tests.rs"
    );
    assert!(
        resolution.contains("pub struct ShortcutResolution")
            && resolution.contains("pub fn unhandled")
            && resolution.contains("pub fn pending_chord"),
        "shortcut result constructors should live in shortcuts/resolution.rs"
    );
    assert!(
        gesture.contains("pub enum ShortcutModifier")
            && gesture.contains("pub struct ShortcutGesture")
            && gesture.contains("impl From<KeyPress> for ShortcutGesture"),
        "shortcut modifier and key matching should live in shortcuts/gesture.rs"
    );
    assert!(
        layer.contains("pub struct ShortcutBinding")
            && layer.contains("pub struct ShortcutLayer")
            && layer.contains("pub fn resolve_or_else"),
        "shortcut binding collections and modal resolution should live in shortcuts/layer.rs"
    );
}

#[test]
fn repaint_signaling_keeps_coalescing_and_callback_tests_focused() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let source = fs::read_to_string(manifest_dir.join("src/gui/repaint.rs"))
        .expect("repaint signaling source should be readable");
    let tests = fs::read_to_string(manifest_dir.join("src/gui/repaint/tests.rs"))
        .expect("repaint signaling behavior tests should be readable");

    assert!(
        source.contains("pub trait RepaintSignal")
            && source.contains("pub fn try_mark_repaint_pending")
            && source.contains("pub struct CoalescingRepaintSignal")
            && source.contains("pub struct SharedRepaintSignal")
            && source.contains("#[path = \"repaint/tests.rs\"]")
            && !source.contains("fn shared_repaint_signal_forwards_request_to_active_callback"),
        "repaint signaling primitives should live in gui/repaint.rs while behavior tests stay delegated"
    );
    assert!(
        tests.contains("fn shared_repaint_signal_forwards_request_to_active_callback")
            && tests.contains("fn coalescing_repaint_signal_clears_pending_when_queue_fails"),
        "repaint behavior coverage should live in gui/repaint/tests.rs"
    );
}

#[test]
fn canvas_gesture_primitives_stay_in_event_pointer_and_state_modules() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let root = fs::read_to_string(manifest_dir.join("src/widgets/interaction/canvas_gesture.rs"))
        .expect("canvas gesture root should be readable");
    let event =
        fs::read_to_string(manifest_dir.join("src/widgets/interaction/canvas_gesture/event.rs"))
            .expect("canvas gesture event module should be readable");
    let pointer =
        fs::read_to_string(manifest_dir.join("src/widgets/interaction/canvas_gesture/pointer.rs"))
            .expect("canvas gesture pointer module should be readable");
    let state =
        fs::read_to_string(manifest_dir.join("src/widgets/interaction/canvas_gesture/state.rs"))
            .expect("canvas gesture state module should be readable");
    let active_press = fs::read_to_string(
        manifest_dir.join("src/widgets/interaction/canvas_gesture/state/active_press.rs"),
    )
    .expect("canvas gesture active press module should be readable");
    let state_tests = fs::read_to_string(
        manifest_dir.join("src/widgets/interaction/canvas_gesture/state/tests.rs"),
    )
    .expect("canvas gesture state tests should be readable");

    for required in [
        "mod event;",
        "mod pointer;",
        "mod state;",
        "pub use event::CanvasGestureEvent;",
        "pub use pointer::CanvasPointer;",
        "pub use state::CanvasGestureState;",
    ] {
        assert!(
            root.contains(required),
            "canvas gesture root should delegate `{required}`"
        );
    }
    assert!(
        !root.contains("pub enum CanvasGestureEvent")
            && !root.contains("pub struct CanvasPointer")
            && !root.contains("pub struct CanvasGestureState"),
        "canvas gesture root should re-export public primitives instead of owning their implementations"
    );
    assert!(
        event.contains("pub enum CanvasGestureEvent")
            && event.contains("Hover(CanvasPointer)")
            && event.contains("FocusChanged(bool)"),
        "canvas gesture event variants should live in canvas_gesture/event.rs"
    );
    assert!(
        pointer.contains("pub struct CanvasPointer")
            && pointer.contains("fn canvas_pointer")
            && pointer.contains("fn point_delta"),
        "canvas pointer projection and delta helpers should live in canvas_gesture/pointer.rs"
    );
    assert!(
        state.contains("mod active_press;")
            && state.contains("#[cfg(test)]")
            && state.contains("mod tests;")
            && state.contains("pub struct CanvasGestureState")
            && state.contains("pub fn handle_input"),
        "canvas retained state and input resolution should live in canvas_gesture/state.rs"
    );
    assert!(
        !state.contains("struct ActiveCanvasPress")
            && active_press.contains("struct ActiveCanvasPress")
            && active_press.contains("origin: CanvasPointer")
            && active_press.contains("button: PointerButton")
            && active_press.contains("modifiers: PointerModifiers"),
        "canvas retained press metadata should live in canvas_gesture/state/active_press.rs"
    );
    assert!(
        !state.contains("fn canvas_gesture_state_tracks_press_drag_and_release")
            && state_tests
                .contains("fn canvas_gesture_state_projects_local_and_normalized_positions")
            && state_tests.contains("fn canvas_gesture_state_tracks_press_drag_and_release")
            && state_tests.contains("fn canvas_gesture_state_clears_drag_on_focus_loss"),
        "canvas gesture state regression tests should live in canvas_gesture/state/tests.rs"
    );
}

#[test]
fn resource_completions_use_named_parts_for_request_results() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let source = fs::read_to_string(manifest_dir.join("src/runtime/resource/load.rs"))
        .expect("resource load module should be readable");
    let resource = fs::read_to_string(manifest_dir.join("src/runtime/resource.rs"))
        .expect("runtime resource module should be readable");
    let runtime =
        fs::read_to_string(manifest_dir.join("src/runtime/mod.rs")).expect("runtime module");
    let lib = fs::read_to_string(manifest_dir.join("src/lib.rs"))
        .expect("library module should be readable");
    let update_context_tasks =
        fs::read_to_string(manifest_dir.join("src/application/runtime/update_context/tasks.rs"))
            .expect("application update context task helpers should be readable");

    assert!(
        source.contains("pub struct ResourceCompletionParts")
            && source.contains("pub fn from_parts(parts: ResourceCompletionParts<T>) -> Self")
            && source.contains("Self::from_parts(ResourceCompletionParts { request, load })"),
        "resource completions should expose named parts and keep the compatibility constructor"
    );
    assert!(
        source.contains("ResourceCompletion::from_parts(ResourceCompletionParts {")
            && update_context_tasks.contains(
                "ResourceCompletion::from_parts(ResourceCompletionParts { request, load })"
            ),
        "resource completion mapping and spawn helpers should use the named-parts construction path"
    );
    assert!(
        resource.contains("ResourceCompletionParts")
            && runtime.contains("ResourceCompletionParts")
            && lib.contains("ResourceCompletionParts"),
        "resource completion parts should remain publicly exported through runtime and prelude"
    );
}

#[test]
fn resource_slots_keep_load_state_and_lifecycle_focused() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let slot = fs::read_to_string(manifest_dir.join("src/runtime/resource/slot.rs"))
        .expect("resource slot module should be readable");
    let state = fs::read_to_string(manifest_dir.join("src/runtime/resource/slot/state.rs"))
        .expect("resource slot state module should be readable");
    let resource = fs::read_to_string(manifest_dir.join("src/runtime/resource.rs"))
        .expect("runtime resource module should be readable");
    let runtime =
        fs::read_to_string(manifest_dir.join("src/runtime/mod.rs")).expect("runtime module");
    let lib = fs::read_to_string(manifest_dir.join("src/lib.rs"))
        .expect("library module should be readable");

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
            && lib.contains("ResourceLoadState")
            && lib.contains("ResourceSlot"),
        "resource slot and load-state types should remain exported through runtime and prelude"
    );
}

#[test]
fn gpu_surface_widget_uses_named_parts_for_retained_resource_identity() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let source_path = manifest_dir.join("src/widgets/primitives/gpu_surface.rs");
    let source = fs::read_to_string(&source_path)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", source_path.display()));
    let widgets = fs::read_to_string(manifest_dir.join("src/widgets/mod.rs"))
        .expect("widgets module should be readable");
    let application_builder =
        fs::read_to_string(manifest_dir.join("src/application/builders/leaf.rs"))
            .expect("application leaf builders should be readable");

    assert!(
        source.contains("pub struct GpuSurfaceParts")
            && source.contains("pub fn from_parts(parts: GpuSurfaceParts) -> Self"),
        "retained GPU surfaces should expose named parts for resource identity, revision, and content"
    );
    assert!(
        source.contains("Self::from_parts(GpuSurfaceParts {")
            && widgets.contains("GpuSurfaceParts")
            && application_builder.contains("pub fn gpu_surface_from_parts"),
        "GPU surface compatibility constructors, public exports, and application builders should keep the named-parts path available"
    );
}

#[test]
fn gpu_surface_primitive_keeps_surface_builders_focused() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let root = fs::read_to_string(manifest_dir.join("src/widgets/primitives/gpu_surface.rs"))
        .expect("gpu-surface primitive root should be readable");
    let builders =
        fs::read_to_string(manifest_dir.join("src/widgets/primitives/gpu_surface/builders.rs"))
            .expect("gpu-surface primitive builders should be readable");

    assert!(
        root.contains("mod builders;")
            && root.contains("pub struct GpuSurfaceWidget")
            && root.contains("impl Widget for GpuSurfaceWidget")
            && !root.contains("impl<Message> SurfaceNode<Message>"),
        "gpu-surface primitive root should own widget behavior while delegating runtime builders"
    );
    assert!(
        builders.contains("impl<Message> SurfaceNode<Message>")
            && builders.contains("pub fn gpu_surface("),
        "gpu-surface runtime builder helper should live in gpu_surface/builders.rs"
    );
}

#[test]
fn gpu_surface_content_models_stay_focused() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let content = fs::read_to_string(manifest_dir.join("src/runtime/gpu_surface/content.rs"))
        .expect("GPU surface content module should be readable");
    let model = fs::read_to_string(manifest_dir.join("src/runtime/gpu_surface/content/model.rs"))
        .expect("GPU surface content model module should be readable");
    let validation =
        fs::read_to_string(manifest_dir.join("src/runtime/gpu_surface/content/validation.rs"))
            .expect("GPU surface content validation module should be readable");
    let tests = fs::read_to_string(manifest_dir.join("src/runtime/gpu_surface/content/tests.rs"))
        .expect("GPU surface content test root should be readable");
    let atlas_tests =
        fs::read_to_string(manifest_dir.join("src/runtime/gpu_surface/content/tests/atlas.rs"))
            .expect("GPU surface atlas tests should be readable");
    let signal_tests = fs::read_to_string(
        manifest_dir.join("src/runtime/gpu_surface/content/tests/signal_shape.rs"),
    )
    .expect("GPU surface signal shape tests should be readable");
    let validation_tests = fs::read_to_string(
        manifest_dir.join("src/runtime/gpu_surface/content/tests/validation.rs"),
    )
    .expect("GPU surface content validation tests should be readable");
    let gpu_surface = fs::read_to_string(manifest_dir.join("src/runtime/gpu_surface.rs"))
        .expect("GPU surface runtime facade should be readable");

    assert!(
        content.contains("mod model;")
            && content.contains("pub use model::{GpuSignalGainPreview, GpuSignalRenderShape};")
            && content.contains("pub enum GpuSurfaceContent")
            && !content.contains("pub struct GpuSignalGainPreview")
            && !content.contains("pub struct GpuSignalRenderShape"),
        "GPU surface content root should expose the retained content enum while re-exporting focused signal content models"
    );
    assert!(
        model.contains("pub struct GpuSignalGainPreview")
            && model.contains("pub fade_in_length: f32")
            && model.contains("pub struct GpuSignalRenderShape")
            && model.contains("pub sample_count: usize"),
        "GPU signal gain-preview and render-shape DTOs should live in content/model.rs"
    );
    assert!(
        validation.contains("validate_signal_gain_preview")
            && validation.contains("validate_signal_render_shape"),
        "GPU surface content validation should stay in the validation module"
    );
    assert!(
        tests.contains("mod atlas;")
            && tests.contains("mod signal_shape;")
            && tests.contains("mod validation;")
            && !tests.contains("fn rgba_atlas_source_rect_must_be_inside_atlas")
            && !tests.contains("fn signal_render_shape_rejects_invalid_payload_contracts"),
        "GPU surface content test root should index focused content behavior groups instead of owning all cases"
    );
    assert!(
        atlas_tests.contains("fn rgba_atlas_source_rect_must_be_inside_atlas")
            && signal_tests.contains("fn signal_render_shape_uses_effective_available_frame_count")
            && validation_tests
                .contains("fn gpu_surface_content_validation_rejects_non_finite_gain_preview"),
        "GPU surface content behavior tests should stay grouped by atlas, signal shape, and validation concerns"
    );
    assert!(
        gpu_surface.contains("GpuSignalGainPreview")
            && gpu_surface.contains("GpuSignalRenderShape")
            && gpu_surface.contains("GpuSurfaceContent")
            && gpu_surface.contains("GpuSurfaceContentError"),
        "GPU surface content models and diagnostics should remain available through the runtime facade"
    );
}
