use super::*;
use crate::{
    application::{
        CommandBinding, CommandDescriptor, CommandDispatchStatus, CommandDispatcher, CommandId,
        CommandInput, CommandInvocation, CommandKey, CommandRegistry, CommandRequest, CommandScope,
        CommandScopeKind, CommandShortcut, IntoView, TextKey, text,
    },
    gui::{focus::FocusSurface, shortcuts::ShortcutPlatform},
    layout::Vector2,
    runtime::SurfaceRuntime,
};
use std::{cell::RefCell, rc::Rc};

fn id() -> CommandId {
    CommandId::new("child.action").unwrap()
}
fn registry() -> CommandRegistry {
    CommandRegistry::new([
        CommandDescriptor::new(id(), TextKey::new("child.action", "Action"))
            .default_binding(CommandShortcut::new(CommandKey::Character("k".into()))),
    ])
    .unwrap()
}
fn input() -> CommandInput {
    CommandInput::logical(CommandKey::Character("k".into()), ShortcutPlatform::Mac)
}
fn surface(context: u32) -> Arc<UiSurface<u32>> {
    crate::runtime::test_arc_surface(
        text("Child")
            .id(100)
            .command_scope(
                CommandScope::new(
                    "window",
                    CommandScopeKind::Window,
                    [CommandBinding::new(id(), context)],
                )
                .unwrap(),
            )
            .into_surface(),
    )
}

#[test]
fn child_commands_use_child_scopes_and_forward_once_before_parent_reduction() {
    let calls = Rc::new(RefCell::new(Vec::new()));
    let mapped = Rc::clone(&calls);
    let reduced = Rc::clone(&calls);
    let mut parent = SurfaceRuntime::new(
        crate::app(())
            .view(|_: &()| {
                text("Parent").id(1).command_scope(
                    CommandScope::new(
                        "window",
                        CommandScopeKind::Window,
                        [CommandBinding::new(id(), 42u32)],
                    )
                    .unwrap(),
                )
            })
            .command_registry(
                registry(),
                CommandDispatcher::new(move |invocation: CommandInvocation<u32>| {
                    mapped.borrow_mut().push(("mapper", *invocation.context()));
                    *invocation.context()
                }),
            )
            .update(move |_, message| reduced.borrow_mut().push(("reducer", message)))
            .into_bridge(),
        Vector2::new(320.0, 180.0),
    );
    let mut bridge = AuxiliarySurfaceBridge::new(surface(99), false, false);
    bridge.command_service = parent.command_service();
    let mut child = SurfaceRuntime::new(bridge, Vector2::new(320.0, 180.0));
    let (status, outcome) =
        child.dispatch_command_request(CommandRequest::Input(&input()), FocusSurface::None);
    assert_eq!(status, CommandDispatchStatus::Mapped);
    assert_eq!(outcome.messages_dispatched, 1);
    assert_eq!(*calls.borrow(), [("mapper", 99)]);
    let messages = child.bridge_mut().take_messages();
    assert_eq!(messages, [99]);
    for message in messages {
        parent.dispatch_message(message);
    }
    assert_eq!(*calls.borrow(), [("mapper", 99), ("reducer", 99)]);
    assert!(child.bridge_mut().take_messages().is_empty());
    child.execute_command(Command::exit());
    let (status, _) =
        child.dispatch_command_request(CommandRequest::Input(&input()), FocusSurface::None);
    assert_eq!(status, CommandDispatchStatus::Unavailable);
    assert_eq!(calls.borrow().len(), 2);
    parent.execute_command(Command::exit());
    assert!(parent.command_service().is_none());
}

#[test]
fn child_service_replacement_applies_keymaps_and_rejects_old_registry_targets() {
    use crate::application::{CommandService, CommandSource, Keymap};
    let registry = registry();
    let mut bridge = AuxiliarySurfaceBridge::new(surface(99), false, false);
    bridge.command_service = Some(CommandService::new(
        registry.clone(),
        CommandDispatcher::new(|invocation: CommandInvocation<u32>| *invocation.context()),
        Keymap::new(),
    ));
    let mut child = SurfaceRuntime::new(bridge, Vector2::new(320.0, 180.0));
    let rows = child
        .command_presentations(&[id()], ShortcutPlatform::Mac)
        .unwrap();
    assert_eq!(rows[0].label, "Action");
    assert!(rows[0].enabled);
    let target = registry
        .target(&child.command_scopes::<u32>().unwrap(), &id())
        .unwrap();
    child.bridge_mut().command_service = Some(CommandService::new(
        registry.clone(),
        CommandDispatcher::new(|invocation: CommandInvocation<u32>| *invocation.context()),
        Keymap::new().override_bindings(&id(), Vec::new()).unwrap(),
    ));
    let (status, _) =
        child.dispatch_command_request(CommandRequest::Input(&input()), FocusSurface::None);
    assert_eq!(status, CommandDispatchStatus::Unhandled);
    let replacement = CommandRegistry::new(registry.commands().cloned()).unwrap();
    child.bridge_mut().command_service = Some(CommandService::new(
        replacement,
        CommandDispatcher::new(|_: CommandInvocation<u32>| panic!("stale target reached mapper")),
        Keymap::new(),
    ));
    let (status, _) = child.dispatch_command_request(
        CommandRequest::Target(&target, CommandSource::Menu),
        FocusSurface::None,
    );
    assert_eq!(status, CommandDispatchStatus::Stale);
    assert!(child.bridge_mut().take_messages().is_empty());
}

#[test]
fn application_scopes_cross_windows_with_local_precedence_and_stale_target_fencing() {
    use crate::application::CommandSource;
    fn global(value: u32) -> CommandScope<u32> {
        CommandScope::new(
            "global",
            CommandScopeKind::Application,
            [CommandBinding::new(id(), value)],
        )
        .unwrap()
    }
    let mut parent = SurfaceRuntime::new(
        crate::app(global(42))
            .view(|scope: &CommandScope<u32>| text("Parent").id(1).command_scope(scope.clone()))
            .command_registry(
                registry(),
                CommandDispatcher::new(|invocation: CommandInvocation<u32>| *invocation.context()),
            )
            .update(|scope, value| *scope = global(value))
            .into_bridge(),
        Vector2::new(320.0, 180.0),
    );
    // The parent's own query must not combine its application scope twice.
    assert!(
        parent
            .command_presentations(&[id()], ShortcutPlatform::Mac)
            .unwrap()[0]
            .enabled
    );
    let plain = crate::runtime::test_arc_surface(text::<u32>("Child").id(100).into_surface());
    let mut bridge = AuxiliarySurfaceBridge::new(plain, false, false);
    bridge.command_service = parent.command_service();
    let mut child = SurfaceRuntime::new(bridge, Vector2::new(320.0, 180.0));
    let activation = child
        .command_presentations(&[id()], ShortcutPlatform::Mac)
        .unwrap()[0]
        .activation(CommandSource::Menu)
        .unwrap();
    let (status, _) = child.dispatch_command_request(activation.request(), FocusSurface::None);
    assert_eq!(status, CommandDispatchStatus::Mapped);
    assert_eq!(child.bridge_mut().take_messages(), [42]);
    parent.dispatch_message(43);
    child.bridge_mut().command_service = parent.command_service();
    let (status, _) = child.dispatch_command_request(activation.request(), FocusSurface::None);
    assert_eq!(status, CommandDispatchStatus::Stale);
    assert!(child.bridge_mut().take_messages().is_empty());
    let (status, _) =
        child.dispatch_command_request(CommandRequest::Input(&input()), FocusSurface::None);
    assert_eq!(status, CommandDispatchStatus::Mapped);
    assert_eq!(child.bridge_mut().take_messages(), [43]);
    // A grandchild receives the same inherited scope exactly once.
    let mut grandchild_bridge = AuxiliarySurfaceBridge::new(
        crate::runtime::test_arc_surface(text::<u32>("Grandchild").id(1).into_surface()),
        false,
        false,
    );
    grandchild_bridge.command_service = child.command_service();
    let mut grandchild = SurfaceRuntime::new(grandchild_bridge, Vector2::new(320.0, 180.0));
    let (status, _) =
        grandchild.dispatch_command_request(CommandRequest::Input(&input()), FocusSurface::None);
    assert_eq!(status, CommandDispatchStatus::Mapped);
    assert_eq!(grandchild.bridge_mut().take_messages(), [43]);
    // Child window context overrides the lower-priority application fallback.
    let mut local_bridge = AuxiliarySurfaceBridge::new(surface(99), false, false);
    local_bridge.command_service = parent.command_service();
    let mut local = SurfaceRuntime::new(local_bridge, Vector2::new(320.0, 180.0));
    let (status, _) =
        local.dispatch_command_request(CommandRequest::Input(&input()), FocusSurface::None);
    assert_eq!(status, CommandDispatchStatus::Mapped);
    assert_eq!(local.bridge_mut().take_messages(), [99]);
}
