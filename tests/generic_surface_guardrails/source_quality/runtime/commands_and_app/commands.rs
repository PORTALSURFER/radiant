use super::*;

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
    let prelude = public_prelude_source(&manifest_dir);

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
            && prelude.contains("ScrollIntoViewParts")
            && prelude.contains("ScrollFixedRowIntoViewParts"),
        "scroll reveal named request parts should be available through runtime and prelude paths"
    );
}

#[test]
fn ui_update_context_keeps_followup_command_groups_in_focused_modules() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let root = fs::read_to_string(manifest_dir.join("src/application/runtime/update_context.rs"))
        .expect("application update context root should be readable");
    let commands =
        fs::read_to_string(manifest_dir.join("src/application/runtime/update_context/commands.rs"))
            .expect("application update context command helpers should be readable");
    let platform =
        fs::read_to_string(manifest_dir.join("src/application/runtime/update_context/platform.rs"))
            .expect("application update context platform helpers should be readable");
    let business = fs::read_to_string(
        manifest_dir.join("src/application/runtime/update_context/business/mod.rs"),
    )
    .expect("application update context business root should be readable");
    let business_request = fs::read_to_string(
        manifest_dir.join("src/application/runtime/update_context/business/request.rs"),
    )
    .expect("application update context business request helpers should be readable");
    let business_latest = fs::read_to_string(
        manifest_dir.join("src/application/runtime/update_context/business/latest.rs"),
    )
    .expect("application update context business latest helpers should be readable");
    let business_keyed_latest = fs::read_to_string(
        manifest_dir.join("src/application/runtime/update_context/business/keyed_latest.rs"),
    )
    .expect("application update context business keyed latest helpers should be readable");
    let business_resource = fs::read_to_string(
        manifest_dir.join("src/application/runtime/update_context/business/resource.rs"),
    )
    .expect("application update context business resource helpers should be readable");
    let surface =
        fs::read_to_string(manifest_dir.join("src/application/runtime/update_context/surface.rs"))
            .expect("application update context surface helpers should be readable");

    for required in [
        "mod commands;",
        "mod business;",
        "mod platform;",
        "mod surface;",
        "pub struct UiUpdateContext<Message>",
        "pub(in crate::application) fn queue_command(&mut self, command: Command<Message>)",
        "fn business(&mut self) -> BusinessRuntime<'_, Message>",
        "fn into_command(self) -> Command<Message>",
    ] {
        assert!(
            root.contains(required),
            "UI update context root should own the queue and delegate `{required}`"
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
            && platform.contains("pub fn reveal_path")
            && platform.contains("pub fn copy_file_paths")
            && platform.contains("pub fn read_file_paths")
            && platform.contains("pub fn confirm"),
        "platform and external-drag helpers should live in update_context/platform.rs"
    );
    assert!(
        business.contains("pub struct BusinessRuntime")
            && business.contains("pub fn interactive(self, name: &'static str)")
            && business.contains("pub fn priority(")
            && business.contains("pub fn background(self, name: &'static str)")
            && business.contains("pub fn idle(self, name: &'static str)")
            && business_request.contains("pub fn latest(self, latest: &mut LatestTask)")
            && business_request.contains("pub fn latest_for<Key>")
            && business_request.contains("pub fn resource<Output>")
            && business_request.contains("pub fn cancellable(self)")
            && business_request.contains("pub fn run<Output>")
            && business_latest.contains("pub struct BusinessLatestRequest")
            && business_keyed_latest.contains("pub struct BusinessKeyedLatestRequest")
            && business_resource.contains("pub struct BusinessResourceRequest"),
        "business runtime helpers should stay split by request concern under update_context/business"
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
fn app_facing_runtime_surface_hides_command_escape_hatches() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let prelude_commands = fs::read_to_string(manifest_dir.join("src/prelude/runtime/commands.rs"))
        .expect("runtime command prelude should be readable");
    let prelude_runtime =
        fs::read_to_string(manifest_dir.join("src/prelude/application/runtime.rs"))
            .expect("application runtime prelude should be readable");
    let update_context =
        fs::read_to_string(manifest_dir.join("src/application/runtime/update_context.rs"))
            .expect("application update context root should be readable");
    let update_context_commands =
        fs::read_to_string(manifest_dir.join("src/application/runtime/update_context/commands.rs"))
            .expect("application update context command helpers should be readable");
    let app_with_view =
        fs::read_to_string(manifest_dir.join("src/application/launch/stateful/with_view.rs"))
            .expect("stateful app with-view builder should be readable");

    assert!(
        !prelude_commands.contains("Command"),
        "normal app prelude must not re-export runtime Command; apps should use typed UiUpdateContext helpers and context.business()"
    );
    assert!(
        prelude_runtime.contains("UiUpdateContext")
            && !prelude_runtime.contains(", UpdateContext")
            && !prelude_runtime.contains("{\n    UpdateContext")
            && !prelude_runtime.contains("BusinessWorkContext"),
        "normal app prelude should expose UiUpdateContext, not the old context name or worker-only BusinessWorkContext"
    );
    assert!(
        update_context.contains(
            "pub(in crate::application) fn queue_command(&mut self, command: Command<Message>)"
        ) && !update_context.contains("pub fn command(")
            && !update_context_commands.contains("pub fn command("),
        "UiUpdateContext may queue commands internally but must not expose arbitrary command injection to app handlers"
    );
    assert!(
        !app_with_view.contains("pub fn update_command")
            && !app_with_view.contains("pub fn update_with")
            && !app_with_view.contains("pub fn reducer")
            && !app_with_view.contains("context.command("),
        "stateful app builders should not offer compatibility handler aliases or command-returning app handlers on the normal app-facing path"
    );
}

#[test]
fn business_work_context_is_worker_only_capability() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let work_context = fs::read_to_string(
        manifest_dir.join("src/application/runtime/update_context/business/work_context.rs"),
    )
    .expect("business work context should be readable");
    let request = fs::read_to_string(
        manifest_dir.join("src/application/runtime/update_context/business/request.rs"),
    )
    .expect("business request should be readable");

    assert!(
        work_context.contains("pub struct BusinessWorkContext")
            && work_context.contains("pub(super) fn new(")
            && !work_context.contains("Default"),
        "BusinessWorkContext should be public as a worker closure argument but not app-constructible"
    );
    assert!(
        request.contains("move || work(BusinessWorkContext::new(worker_token))"),
        "business runtime should create BusinessWorkContext only inside worker command execution"
    );
}
