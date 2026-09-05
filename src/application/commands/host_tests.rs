use super::*;
use crate::{
    application::{Layer, TextKey},
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

fn scope(name: &str, kind: CommandScopeKind, context: u32) -> CommandScope<u32> {
    CommandScope::new(name, kind, [CommandBinding::new(id(), context)]).unwrap()
}

#[test]
fn declarative_commands_follow_committed_focus_and_refresh_context() {
    let registry = registry();
    let targets = registry.clone();
    let calls = Rc::new(RefCell::new(Vec::new()));
    let observed = Rc::clone(&calls);
    let mut runtime = SurfaceRuntime::new(
        crate::app(10u32)
            .view(|state: &u32| {
                column([
                    button("First")
                        .message(0)
                        .id(101)
                        .commands([CommandBinding::new(id(), *state)]),
                    button("Second")
                        .message(0)
                        .id(102)
                        .commands([CommandBinding::new(id(), *state + 1)]),
                ])
                .id(100)
                .command_scope(scope("window", CommandScopeKind::Window, 99))
            })
            .command_registry(
                registry,
                CommandDispatcher::new(|invocation: CommandInvocation<u32>| *invocation.context()),
            )
            .update(move |state, message| {
                observed.borrow_mut().push(message);
                *state += 10;
            })
            .into_bridge(),
        Vector2::new(320.0, 180.0),
    );
    assert!(runtime.focus_widget(101));
    let old = targets
        .target(&runtime.command_scopes::<u32>().unwrap(), &id())
        .unwrap();
    let (status, outcome) =
        runtime.dispatch_command_request(CommandRequest::Input(&input()), FocusSurface::None);
    assert_eq!(status, CommandDispatchStatus::Mapped);
    assert_eq!(outcome.messages_dispatched, 1);
    assert_eq!(*calls.borrow(), [10]);
    runtime.refresh();
    assert_eq!(runtime.focused_widget(), Some(101));
    let (status, _) = runtime.dispatch_command_request(
        CommandRequest::Target(&old, CommandSource::Menu),
        FocusSurface::None,
    );
    assert_eq!(status, CommandDispatchStatus::Stale);
    assert!(runtime.focus_widget(102));
    let (status, _) =
        runtime.dispatch_command_request(CommandRequest::Input(&input()), FocusSurface::None);
    assert_eq!(status, CommandDispatchStatus::Mapped);
    assert_eq!(*calls.borrow(), [10, 21]);
}

#[test]
fn declarative_command_layers_use_runtime_order_and_exclude_passive_content() {
    let calls = Rc::new(RefCell::new(Vec::new()));
    let observed = Rc::clone(&calls);
    let mut runtime = SurfaceRuntime::new(
        crate::app(())
            .view(|_: &()| {
                scene(text("base").id(100).command_scope(scope(
                    "window",
                    CommandScopeKind::Window,
                    1,
                )))
                .layer(Layer::modal(text("lower").id(101).command_scope(scope(
                    "lower",
                    CommandScopeKind::Modal { order: 999 },
                    2,
                ))))
                .layer(Layer::modal(text("upper").id(102).command_scope(scope(
                    "upper",
                    CommandScopeKind::Modal { order: 0 },
                    3,
                ))))
                .layer(Layer::tooltip(text("passive").id(103).command_scope(
                    scope("passive", CommandScopeKind::Modal { order: 9999 }, 4),
                )))
                .into_view()
            })
            .command_registry(
                registry(),
                CommandDispatcher::new(|invocation: CommandInvocation<u32>| *invocation.context()),
            )
            .update(move |_, message| observed.borrow_mut().push(message))
            .into_bridge(),
        Vector2::new(320.0, 180.0),
    );
    let (status, _) =
        runtime.dispatch_command_request(CommandRequest::Input(&input()), FocusSurface::None);
    assert_eq!(status, CommandDispatchStatus::Mapped);
    assert_eq!(*calls.borrow(), [3]);
}

#[test]
fn declarative_command_context_mismatch_is_terminal() {
    let mut runtime = SurfaceRuntime::new(
        crate::app(())
            .view(|_: &()| {
                text("base").id(100).command_scope(
                    CommandScope::new(
                        "wrong",
                        CommandScopeKind::Window,
                        [CommandBinding::new(id(), "wrong type")],
                    )
                    .unwrap(),
                )
            })
            .command_registry(
                registry(),
                CommandDispatcher::new(|_: CommandInvocation<u32>| {
                    panic!("invalid context reached mapper")
                }),
            )
            .update(|_, _: ()| panic!("invalid context reached reducer"))
            .into_bridge(),
        Vector2::new(320.0, 180.0),
    );
    let (status, outcome) =
        runtime.dispatch_command_request(CommandRequest::Input(&input()), FocusSurface::None);
    assert_eq!(
        status,
        CommandDispatchStatus::Suppressed(CommandSuppression::ContextMismatch)
    );
    assert_eq!(outcome.messages_dispatched, 0);
}

#[test]
fn declarative_command_invalid_ownership_and_capacity_fail_closed() {
    for (case, expected) in [
        (0, CommandSuppression::InvalidScopes),
        (1, CommandSuppression::InvalidScopes),
        (2, CommandSuppression::Capacity),
    ] {
        let mut runtime = SurfaceRuntime::new(
            crate::app(case)
                .view(|case: &u32| match case {
                    0 => text("base").id(100).command_scope(scope(
                        "misplaced",
                        CommandScopeKind::Modal { order: 0 },
                        1,
                    )),
                    1 => column([
                        text("owner").id(101).command_scope(scope(
                            "window",
                            CommandScopeKind::Window,
                            1,
                        )),
                        text("duplicate scope").id(102).command_scope(scope(
                            "window",
                            CommandScopeKind::Window,
                            2,
                        )),
                    ])
                    .id(100),
                    _ => column((0..65).map(|index| {
                        text("scope").id(100 + index).command_scope(scope(
                            &format!("scope-{index}"),
                            CommandScopeKind::Window,
                            1,
                        ))
                    }))
                    .id(1),
                })
                .command_registry(
                    registry(),
                    CommandDispatcher::new(|_: CommandInvocation<u32>| {
                        panic!("invalid scope reached mapper")
                    }),
                )
                .update(|_, _: ()| panic!("invalid scope reached reducer"))
                .into_bridge(),
            Vector2::new(320.0, 180.0),
        );
        let (status, outcome) =
            runtime.dispatch_command_request(CommandRequest::Input(&input()), FocusSurface::None);
        assert_eq!(
            status,
            CommandDispatchStatus::Suppressed(expected),
            "case {case}"
        );
        assert_eq!(outcome.messages_dispatched, 0);
    }
}

#[test]
fn declarative_commands_nearest_disabled_editor_declines_to_ancestor() {
    let calls = Rc::new(RefCell::new(Vec::new()));
    let observed = Rc::clone(&calls);
    let mut runtime = SurfaceRuntime::new(
        crate::app(())
            .view(|_: &()| {
                column([
                    button("disabled")
                        .message(0)
                        .id(101)
                        .commands([CommandBinding::new(id(), 1).enabled(false)]),
                    button("sibling")
                        .message(0)
                        .id(102)
                        .commands([CommandBinding::new(id(), 2)]),
                ])
                .id(100)
                .commands([CommandBinding::new(id(), 3)])
            })
            .command_registry(
                registry(),
                CommandDispatcher::new(|invocation: CommandInvocation<i32>| *invocation.context()),
            )
            .update(move |_, message| observed.borrow_mut().push(message))
            .into_bridge(),
        Vector2::new(320.0, 180.0),
    );
    assert!(runtime.focus_widget(101));
    let (status, _) =
        runtime.dispatch_command_request(CommandRequest::Input(&input()), FocusSurface::None);
    assert_eq!(status, CommandDispatchStatus::Mapped);
    assert_eq!(*calls.borrow(), [3]);
    assert!(runtime.focus_widget(102));
    let (status, _) =
        runtime.dispatch_command_request(CommandRequest::Input(&input()), FocusSurface::None);
    assert_eq!(status, CommandDispatchStatus::Mapped);
    assert_eq!(*calls.borrow(), [3, 2]);
}

#[test]
fn declarative_prepared_scope_retains_targets_and_reads_keymap_overrides() {
    let registry = registry();
    let unchanged = scope("window", CommandScopeKind::Window, 7);
    let target = registry
        .target(std::slice::from_ref(&unchanged), &id())
        .unwrap();
    let keymap = Keymap::new().override_bindings(&id(), Vec::new()).unwrap();
    let calls = Rc::new(RefCell::new(Vec::new()));
    let observed = Rc::clone(&calls);
    let mut runtime = SurfaceRuntime::new(
        crate::app((unchanged, keymap))
            .view(|state: &(CommandScope<u32>, Keymap)| {
                text("scope").id(100).command_scope(state.0.clone())
            })
            .command_registry(
                registry,
                CommandDispatcher::new(|invocation: CommandInvocation<u32>| *invocation.context()),
            )
            .command_keymap(|state| state.1.clone())
            .update(move |_, message| observed.borrow_mut().push(message))
            .into_bridge(),
        Vector2::new(320.0, 180.0),
    );
    let (status, outcome) =
        runtime.dispatch_command_request(CommandRequest::Input(&input()), FocusSurface::None);
    assert_eq!(status, CommandDispatchStatus::Unhandled);
    assert_eq!(outcome.messages_dispatched, 0);
    for _ in 0..2 {
        runtime.refresh();
        let (status, outcome) = runtime.dispatch_command_request(
            CommandRequest::Target(&target, CommandSource::Toolbar),
            FocusSurface::None,
        );
        assert_eq!(status, CommandDispatchStatus::Mapped);
        assert_eq!(outcome.messages_dispatched, 1);
    }
    assert_eq!(*calls.borrow(), [7, 7]);
}
