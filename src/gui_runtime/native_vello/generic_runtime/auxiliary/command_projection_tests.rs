use super::*;
use crate::application::{
    CommandBinding, CommandDescriptor, CommandDispatchStatus, CommandDispatcher, CommandId,
    CommandInput, CommandInvocation, CommandKey, CommandRegistry, CommandRequest, CommandScope,
    CommandScopeKind, CommandService, CommandShortcut, IntoView, Keymap, TextKey, text,
};
use crate::gui::{focus::FocusSurface, shortcuts::ShortcutPlatform};

fn id() -> CommandId {
    CommandId::new("action").unwrap()
}
fn projection(context: i32) -> AuxiliaryWindow<i32> {
    AuxiliaryWindow::new(
        "child",
        NativeRunOptions::default(),
        crate::runtime::test_arc_surface(
            text("Child")
                .id(1)
                .command_scope(
                    CommandScope::new(
                        "window",
                        CommandScopeKind::Window,
                        [CommandBinding::new(id(), context)],
                    )
                    .unwrap(),
                )
                .into_surface(),
        ),
    )
}

#[test]
fn auxiliary_projection_updates_command_service_and_surface_together() {
    let registry =
        CommandRegistry::new([
            CommandDescriptor::new(id(), TextKey::new("action", "Action"))
                .default_binding(CommandShortcut::new(CommandKey::Character("k".into()))),
        ])
        .unwrap();
    let service = |keymap| {
        CommandService::new(
            registry.clone(),
            CommandDispatcher::new(|invocation: CommandInvocation<i32>| *invocation.context()),
            keymap,
        )
    };
    let mut window = AuxiliaryNativeWindow::new(
        projection(42),
        &NativeRunOptions::default(),
        None,
        false,
        false,
    );
    let input = CommandInput::logical(CommandKey::Character("k".into()), ShortcutPlatform::Mac);
    let (status, _) = window
        .runner
        .core
        .runtime
        .dispatch_command_request(CommandRequest::Input(&input), FocusSurface::None);
    assert_eq!(status, CommandDispatchStatus::Unhandled);
    window.update_projection_with_commands(projection(99), Some(service(Keymap::new())));
    let (status, _) = window
        .runner
        .core
        .runtime
        .dispatch_command_request(CommandRequest::Input(&input), FocusSurface::None);
    assert_eq!(status, CommandDispatchStatus::Mapped);
    assert_eq!(
        window.runner.core.runtime.bridge_mut().take_messages(),
        [99]
    );
    window.update_projection_with_commands(
        projection(100),
        Some(service(
            Keymap::new().override_bindings(&id(), Vec::new()).unwrap(),
        )),
    );
    let (status, _) = window
        .runner
        .core
        .runtime
        .dispatch_command_request(CommandRequest::Input(&input), FocusSurface::None);
    assert_eq!(status, CommandDispatchStatus::Unhandled);
    assert!(
        window
            .runner
            .core
            .runtime
            .bridge_mut()
            .take_messages()
            .is_empty()
    );
}
