//! Headless semantic-command dispatch through the ordinary application reducer.
//!
//! Run with `cargo run --example contextual_commands`. Scopes come from the committed
//! view; the headless host explicitly submits shortcut and presentation requests.
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
    let presentation_registry = registry.clone();
    let mut runtime = SurfaceRuntime::new(
        radiant::app(42u64)
            .view(|document: &u64| {
                button("Document")
                    .message((*document, CommandSource::Application))
                    .id(100)
                    .commands([CommandBinding::new(save_id(), *document)])
            })
            .command_registry(
                registry,
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
    assert!(runtime.focus_widget(100));
    let mut input = CommandInput::logical(CommandKey::Character("s".into()), ShortcutPlatform::Mac);
    input.modifiers.meta = true;
    let (status, outcome) =
        runtime.dispatch_command_request(CommandRequest::Input(&input), FocusSurface::None);
    assert_eq!(status, CommandDispatchStatus::Mapped);
    assert_eq!(outcome.messages_dispatched, 1);
    // Query after dispatch: a refreshed view has new captured context.
    let target = presentation_registry
        .target(
            &runtime
                .command_scopes::<u64>()
                .expect("typed active scopes"),
            &save_id(),
        )
        .expect("active command");
    let (status, outcome) = runtime.dispatch_command_request(
        CommandRequest::Target(&target, CommandSource::Menu),
        FocusSurface::None,
    );
    assert_eq!(status, CommandDispatchStatus::Mapped);
    assert_eq!(outcome.messages_dispatched, 1);
}

#[cfg(test)]
mod tests {
    #[test]
    fn example_routes_shortcut_and_menu_through_reducer() {
        super::main();
    }
}
