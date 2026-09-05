use super::*;
use crate::{
    application::TextKey,
    gui::{focus::FocusSurface, shortcuts::ShortcutPlatform},
    layout::Vector2,
    prelude::*,
    runtime::{Command, SurfaceRuntime},
};
use std::{cell::RefCell, rc::Rc};

fn id() -> CommandId {
    CommandId::new("document.save").unwrap()
}
fn registry() -> CommandRegistry {
    CommandRegistry::new([CommandDescriptor::new(id(), TextKey::new("save", "Save"))
        .default_binding(CommandShortcut::new(CommandKey::Character("s".into())).primary())])
    .unwrap()
}
fn snapshot(context: u32) -> CommandSnapshot<u32> {
    CommandSnapshot {
        keymap: Keymap::new(),
        scopes: vec![
            CommandScope::new(
                "document",
                CommandScopeKind::Window,
                [CommandBinding::new(id(), context)],
            )
            .unwrap(),
        ],
    }
}
fn input() -> CommandInput {
    let mut input = CommandInput::logical(CommandKey::Character("s".into()), ShortcutPlatform::Mac);
    input.modifiers.meta = true;
    input
}
#[test]
fn application_command_input_and_targets_share_the_normal_reducer_and_revalidate_state() {
    let registry = registry();
    let initial = snapshot(42);
    let old_target = registry.target(&initial.scopes, &id()).unwrap();
    let calls = Rc::new(RefCell::new(Vec::new()));
    let mapped = Rc::clone(&calls);
    let reduced = Rc::clone(&calls);
    let mut runtime = SurfaceRuntime::new(
        crate::app(initial)
            .view(|_: &CommandSnapshot<u32>| text("Commands").id(100))
            .commands(
                registry,
                |state, _| state.clone(),
                CommandDispatcher::new(move |invocation: CommandInvocation<u32>| {
                    mapped.borrow_mut().push((
                        "mapper",
                        *invocation.context(),
                        invocation.source(),
                    ));
                    *invocation.context()
                }),
            )
            .update(move |state, context| {
                reduced
                    .borrow_mut()
                    .push(("reducer", context, CommandSource::Application));
                *state = snapshot(context + 1);
            })
            .into_bridge(),
        Vector2::new(320.0, 180.0),
    );
    let (status, outcome) =
        runtime.dispatch_command_request(CommandRequest::Input(&input()), FocusSurface::None);
    assert_eq!(status, CommandDispatchStatus::Mapped);
    assert_eq!(outcome.messages_dispatched, 1);
    assert_eq!(
        calls.borrow().as_slice(),
        &[
            ("mapper", 42, CommandSource::Shortcut),
            ("reducer", 42, CommandSource::Application)
        ]
    );
    let (status, outcome) = runtime.dispatch_command_request(
        CommandRequest::Target(&old_target, CommandSource::Menu),
        FocusSurface::None,
    );
    assert_eq!(status, CommandDispatchStatus::Stale);
    assert_eq!(outcome.messages_dispatched, 0);
    assert_eq!(calls.borrow().len(), 2);
    runtime.dispatch_command_request(CommandRequest::Input(&input()), FocusSurface::None);
    assert_eq!(calls.borrow()[2].1, 43);
    runtime.execute_command(Command::exit());
    let (status, outcome) =
        runtime.dispatch_command_request(CommandRequest::Input(&input()), FocusSurface::None);
    assert_eq!(status, CommandDispatchStatus::Unavailable);
    assert_eq!(outcome.messages_dispatched, 0);
    assert_eq!(calls.borrow().len(), 4);
}

#[test]
fn presentation_target_maps_once_and_composing_input_never_reaches_reducer() {
    let registry = registry();
    let state = snapshot(7);
    let target = registry.target(&state.scopes, &id()).unwrap();
    let calls = Rc::new(RefCell::new(Vec::new()));
    let observed = Rc::clone(&calls);
    let mut runtime = SurfaceRuntime::new(
        crate::app(state)
            .view(|_: &CommandSnapshot<u32>| text("Commands"))
            .commands(
                registry,
                |state, _| state.clone(),
                CommandDispatcher::new(|invocation: CommandInvocation<u32>| {
                    (*invocation.context(), invocation.source())
                }),
            )
            .update(move |_, message| observed.borrow_mut().push(message))
            .into_bridge(),
        Vector2::new(320.0, 180.0),
    );
    let (status, outcome) = runtime.dispatch_command_request(
        CommandRequest::Target(&target, CommandSource::Toolbar),
        FocusSurface::None,
    );
    assert_eq!(status, CommandDispatchStatus::Mapped);
    assert_eq!(outcome.messages_dispatched, 1);
    assert_eq!(*calls.borrow(), [(7, CommandSource::Toolbar)]);
    let mut composing = input();
    composing.composing = true;
    let (status, outcome) =
        runtime.dispatch_command_request(CommandRequest::Input(&composing), FocusSurface::None);
    assert_eq!(
        status,
        CommandDispatchStatus::Suppressed(CommandSuppression::Composition)
    );
    assert_eq!(outcome.messages_dispatched, 0);
    assert_eq!(calls.borrow().len(), 1);
}
