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
        "pub struct UpdateContext<Message>",
        "fn business(&mut self) -> BusinessRuntime<'_, Message>",
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
        business.contains("pub struct BusinessRuntime")
            && business.contains("pub fn interactive(self, name: &'static str)")
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
