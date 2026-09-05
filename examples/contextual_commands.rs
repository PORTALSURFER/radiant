//! Headless semantic-command dispatch through the ordinary application reducer.
//!
//! Run with `cargo run --example contextual_commands`. Native keyboard and declarative
//! view-scope adapters are separate from this explicit host-boundary example.
use radiant::prelude::*;
use radiant::{
    application::*,
    gui::{focus::FocusSurface, shortcuts::ShortcutPlatform},
    layout::Vector2,
    runtime::SurfaceRuntime,
};

fn save_id() -> CommandId {
    CommandId::new("document.save").expect("static command identifier")
}

fn main() {
    let registry = CommandRegistry::new([CommandDescriptor::new(
        save_id(),
        TextKey::new("command.save", "Save"),
    )
    .description(TextKey::new(
        "command.save.description",
        "Save the active document",
    ))
    .default_binding(CommandShortcut::new(CommandKey::Character("s".into())).primary())])
    .expect("unique static registrations");
    let snapshot = CommandSnapshot {
        keymap: Keymap::new(),
        scopes: vec![
            CommandScope::new(
                "document-42",
                CommandScopeKind::Window,
                [CommandBinding::new(save_id(), 42u64)],
            )
            .expect("unique scope bindings"),
        ],
    };
    let target = registry
        .target(&snapshot.scopes, &save_id())
        .expect("active command");
    let mut runtime = SurfaceRuntime::new(
        radiant::app(snapshot)
            .view(|_: &CommandSnapshot<u64>| text("Contextual commands"))
            .commands(
                registry,
                |state, _| state.clone(),
                CommandDispatcher::new(|invocation: CommandInvocation<u64>| {
                    (*invocation.context(), invocation.source())
                }),
            )
            .update(|_, (document, source)| {
                println!("Reducer received save for document {document} from {source:?}")
            })
            .into_bridge(),
        Vector2::new(320.0, 180.0),
    );
    let mut input = CommandInput::logical(CommandKey::Character("s".into()), ShortcutPlatform::Mac);
    input.modifiers.meta = true;
    for request in [
        CommandRequest::Input(&input),
        CommandRequest::Target(&target, CommandSource::Menu),
    ] {
        let (status, outcome) = runtime.dispatch_command_request(request, FocusSurface::None);
        assert_eq!(status, CommandDispatchStatus::Mapped);
        assert_eq!(outcome.messages_dispatched, 1);
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn example_routes_shortcut_and_menu_through_reducer() {
        super::main();
    }
}
