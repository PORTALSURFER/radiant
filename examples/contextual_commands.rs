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
    widgets::{WidgetInput, WidgetKey},
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
    let controls_registry = registry.clone();
    let scope = CommandScope::new(
        "document",
        CommandScopeKind::Editor { depth: 0 },
        [CommandBinding::new(save_id(), 42u64)],
    )
    .expect("valid editor scope");
    let calls = std::rc::Rc::new(std::cell::Cell::new(0));
    let observed = std::rc::Rc::clone(&calls);
    let mut runtime = SurfaceRuntime::new(
        radiant::app(scope)
            .view(move |scope: &CommandScope<u64>| {
                let presentation = controls_registry
                    .present(
                        std::slice::from_ref(scope),
                        &Keymap::new(),
                        &save_id(),
                        &Default::default(),
                        ShortcutPlatform::Mac,
                    )
                    .expect("registered command");
                column([
                    button("Document")
                        .message((42, CommandSource::Application))
                        .id(100)
                        .command_scope(scope.clone()),
                    toolbar([presentation.clone().toolbar_button().id(101)]),
                    presentation.clone().menu_item().id(102),
                    presentation.clone().palette_item().id(103),
                    presentation.shortcut_help(),
                ])
            })
            .command_registry(
                registry,
                CommandDispatcher::new(|invocation: CommandInvocation<u64>| {
                    (*invocation.context(), invocation.source())
                }),
            )
            .update(move |_, (document, source)| {
                observed.set(observed.get() + 1);
                println!("Reducer received save for document {document} from {source:?}")
            })
            .into_bridge(),
        Vector2::new(320.0, 400.0),
    );
    assert!(runtime.focus_widget(100));
    let mut input = CommandInput::logical(CommandKey::Character("s".into()), ShortcutPlatform::Mac);
    input.modifiers.meta = true;
    let (status, outcome) =
        runtime.dispatch_command_request(CommandRequest::Input(&input), FocusSurface::None);
    assert_eq!(status, CommandDispatchStatus::Mapped);
    assert_eq!(outcome.messages_dispatched, 1);
    // A native menu adapter reads one current batch, then retains only the activation.
    let native_menu_action = runtime
        .command_presentations(&[save_id()], ShortcutPlatform::Mac)
        .expect("current native presentation")
        .remove(0)
        .activation(CommandSource::Menu)
        .expect("enabled menu item");
    let (status, outcome) =
        runtime.dispatch_command_request(native_menu_action.request(), FocusSurface::None);
    assert_eq!(status, CommandDispatchStatus::Mapped);
    assert_eq!(outcome.messages_dispatched, 1);
    let point = runtime.layout().rects[&101].center();
    runtime.dispatch_input_at(point, WidgetInput::primary_press(point));
    runtime.dispatch_input_at(point, WidgetInput::primary_release(point));
    for control in [102, 103] {
        assert!(runtime.focus_widget(control));
        runtime.dispatch_focused_input(WidgetInput::key_press(WidgetKey::Enter));
    }
    assert_eq!(calls.get(), 5);
}

#[cfg(test)]
mod tests {
    #[test]
    fn example_routes_shortcut_and_menu_through_reducer() {
        super::main();
    }
}
