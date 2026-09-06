use super::*;
use crate::{
    application::{TextKey, text},
    gui::{focus::FocusSurface, shortcuts::ShortcutPlatform},
    layout::Vector2,
    runtime::{Command, SurfaceRuntime},
};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

fn id(name: &str) -> CommandId {
    CommandId::new(name).unwrap()
}
fn registry() -> CommandRegistry {
    CommandRegistry::new([
        CommandDescriptor::new(id("save"), TextKey::new("save", "Save"))
            .default_binding(CommandShortcut::new(CommandKey::Character("s".into())).primary()),
        CommandDescriptor::new(id("close"), TextKey::new("close", "Close")),
    ])
    .unwrap()
}
fn scope(context: u32) -> CommandScope<u32> {
    CommandScope::new(
        "window",
        CommandScopeKind::Window,
        [
            CommandBinding::new(id("save"), context).checked(Some(true)),
            CommandBinding::new(id("close"), context).enabled(false),
        ],
    )
    .unwrap()
}

#[test]
fn native_presentations_share_one_snapshot_and_activations_revalidate_before_mapping() {
    let keymap_reads = Rc::new(Cell::new(0));
    let read = Rc::clone(&keymap_reads);
    let calls = Rc::new(RefCell::new(Vec::new()));
    let mapped = Rc::clone(&calls);
    let reduced = Rc::clone(&calls);
    let mut runtime = SurfaceRuntime::new(
        crate::app(scope(42))
            .view(|state: &CommandScope<u32>| text("Commands").id(1).command_scope(state.clone()))
            .command_registry(
                registry(),
                CommandDispatcher::new(move |invocation: CommandInvocation<u32>| {
                    mapped.borrow_mut().push(("mapper", *invocation.context()));
                    *invocation.context()
                }),
            )
            .command_keymap(move |_| {
                read.set(read.get() + 1);
                Keymap::new()
            })
            .update(move |state, context| {
                reduced.borrow_mut().push(("reducer", context));
                *state = scope(context + 1);
            })
            .into_bridge(),
        Vector2::new(320.0, 180.0),
    );
    let rows = runtime
        .command_presentations(
            &[id("save"), id("close"), id("save")],
            ShortcutPlatform::Mac,
        )
        .unwrap();
    assert_eq!(keymap_reads.get(), 1);
    assert!(calls.borrow().is_empty());
    assert_eq!(
        rows.iter().map(|row| row.id.as_str()).collect::<Vec<_>>(),
        ["save", "close", "save"]
    );
    assert_eq!(rows[0].label, "Save");
    assert_eq!(rows[0].checked, Some(true));
    assert!(rows[0].enabled);
    assert!(!rows[1].enabled);
    assert!(rows[1].activation(CommandSource::Menu).is_none());
    let activation = rows[0].activation(CommandSource::Menu).unwrap();
    let (status, outcome) =
        runtime.dispatch_command_request(activation.request(), FocusSurface::None);
    assert_eq!(status, CommandDispatchStatus::Mapped);
    assert_eq!(outcome.messages_dispatched, 1);
    assert_eq!(*calls.borrow(), [("mapper", 42), ("reducer", 42)]);
    let (status, outcome) =
        runtime.dispatch_command_request(activation.request(), FocusSurface::None);
    assert_eq!(status, CommandDispatchStatus::Stale);
    assert_eq!(outcome.messages_dispatched, 0);
    assert_eq!(calls.borrow().len(), 2);
    let fresh = runtime
        .command_presentations(&[id("save")], ShortcutPlatform::Windows)
        .unwrap();
    assert_ne!(rows[0].shortcuts[0].compact, fresh[0].shortcuts[0].compact);
    assert_eq!(calls.borrow().len(), 2);
    let (status, _) = runtime.dispatch_command_request(
        fresh[0].activation(CommandSource::Menu).unwrap().request(),
        FocusSurface::None,
    );
    assert_eq!(status, CommandDispatchStatus::Mapped);
    assert_eq!(calls.borrow()[2], ("mapper", 43));
}

#[test]
fn native_presentation_errors_reject_the_batch_without_invoking_mapper() {
    let keymap_reads = Rc::new(Cell::new(0));
    let read = Rc::clone(&keymap_reads);
    let mut runtime = SurfaceRuntime::new(
        crate::app(scope(42))
            .view(|state: &CommandScope<u32>| text("Commands").id(1).command_scope(state.clone()))
            .command_registry(
                registry(),
                CommandDispatcher::new(|_: CommandInvocation<u32>| panic!("query invoked mapper")),
            )
            .command_keymap(move |_| {
                read.set(read.get() + 1);
                Keymap::new()
            })
            .update(|_, _: ()| panic!("query invoked reducer"))
            .into_bridge(),
        Vector2::new(320.0, 180.0),
    );
    assert_eq!(
        runtime
            .command_presentations(&vec![id("save"); 257], ShortcutPlatform::Mac)
            .unwrap_err(),
        CommandPresentationError::Capacity
    );
    assert_eq!(keymap_reads.get(), 0);
    assert_eq!(
        runtime
            .command_presentations(&[id("save"), id("unknown")], ShortcutPlatform::Mac)
            .unwrap_err(),
        CommandPresentationError::UnknownCommand(id("unknown"))
    );
    assert_eq!(keymap_reads.get(), 1);
    runtime.execute_command(Command::exit());
    assert_eq!(
        runtime
            .command_presentations(&[id("save")], ShortcutPlatform::Mac)
            .unwrap_err(),
        CommandPresentationError::Unavailable
    );
    assert_eq!(keymap_reads.get(), 1);
}

#[test]
fn native_presentations_use_the_current_resolved_locale_without_mapping() {
    use crate::application::{ApplicationEnvironment, LocaleId, TextCatalog};
    let mut runtime = SurfaceRuntime::new(
        crate::app((scope(42), false))
            .view(|state: &(CommandScope<u32>, bool)| {
                text("Commands").id(1).command_scope(state.0.clone())
            })
            .application_environment(|state: &(CommandScope<u32>, bool)| {
                let french = LocaleId::new("fr").unwrap();
                let catalog = TextCatalog::default().insert(
                    french.clone(),
                    TextKey::new("save", "Save"),
                    "Enregistrer",
                );
                ApplicationEnvironment::new(if state.1 { french } else { LocaleId::english() })
                    .with_catalog(std::sync::Arc::new(catalog))
            })
            .command_registry(
                registry(),
                CommandDispatcher::new(|_: CommandInvocation<u32>| panic!("presentation mapped")),
            )
            .update(|state, _: ()| state.1 = !state.1)
            .into_bridge(),
        Vector2::new(320.0, 180.0),
    );
    assert_eq!(
        runtime
            .command_presentations(&[id("save")], ShortcutPlatform::Mac)
            .unwrap()[0]
            .label,
        "Save"
    );
    runtime.dispatch_message(());
    let rows = runtime
        .command_presentations(&[id("save")], ShortcutPlatform::Mac)
        .unwrap();
    assert_eq!(rows[0].label, "Enregistrer");
    assert_eq!(rows[0].accessibility, "Enregistrer");
}

#[test]
fn native_presentations_reject_mismatched_and_ambiguous_scope_contexts() {
    use crate::application::column;
    for wrong_type in [true, false] {
        let runtime = SurfaceRuntime::new(
            crate::app(wrong_type)
                .view(|wrong_type: &bool| {
                    if *wrong_type {
                        text("Wrong").id(1).command_scope(
                            CommandScope::new(
                                "window",
                                CommandScopeKind::Window,
                                [CommandBinding::new(id("save"), "wrong")],
                            )
                            .unwrap(),
                        )
                    } else {
                        column([
                            text("First").id(1).command_scope(scope(1)),
                            text("Second").id(2).command_scope(scope(2)),
                        ])
                    }
                })
                .command_registry(
                    registry(),
                    CommandDispatcher::new(|_: CommandInvocation<u32>| {
                        panic!("invalid query mapped")
                    }),
                )
                .update(|_, _: ()| panic!("invalid query reduced"))
                .into_bridge(),
            Vector2::new(320.0, 180.0),
        );
        let expected = if wrong_type {
            CommandSuppression::ContextMismatch
        } else {
            CommandSuppression::InvalidScopes
        };
        assert_eq!(
            runtime
                .command_presentations(&[id("save")], ShortcutPlatform::Mac)
                .unwrap_err(),
            CommandPresentationError::Scopes(expected)
        );
    }
}

#[test]
fn inherited_application_scopes_reject_ambiguity_type_mismatch_and_combined_capacity() {
    fn record(name: &str) -> ResolvedCommandScope {
        ResolvedCommandScope {
            node_id: 1,
            kind: CommandScopeKind::Application,
            attachment: CommandScopeAttachment::explicit(
                CommandScope::new(
                    name,
                    CommandScopeKind::Application,
                    [CommandBinding::new(id("save"), 42u32)],
                )
                .unwrap(),
            ),
        }
    }
    let service: CommandService<()> = CommandService::new(
        registry(),
        CommandDispatcher::new(|_: CommandInvocation<u32>| panic!("invalid scopes reached mapper")),
        Keymap::new(),
    );
    let globals = [record("global")];
    let inherited = service
        .clone()
        .with_application_scopes(CommandScopeProjection::new(&globals, None));
    let duplicate = [record("global")];
    let too_many: Vec<_> = (0..64).map(|i| record(&format!("local-{i}"))).collect();
    let wrong = [ResolvedCommandScope {
        node_id: 2,
        kind: CommandScopeKind::Application,
        attachment: CommandScopeAttachment::explicit(
            CommandScope::new(
                "wrong",
                CommandScopeKind::Application,
                [CommandBinding::new(id("save"), "wrong type")],
            )
            .unwrap(),
        ),
    }];
    let input = CommandInput::logical(CommandKey::Character("s".into()), ShortcutPlatform::Mac);
    for (records, expected) in [
        (duplicate.as_slice(), CommandSuppression::InvalidScopes),
        (too_many.as_slice(), CommandSuppression::Capacity),
        (wrong.as_slice(), CommandSuppression::ContextMismatch),
    ] {
        let projection = CommandScopeProjection::new(records, None);
        let rows = inherited.presentations(
            projection,
            &[id("save")],
            &crate::runtime::ResolvedEnvironment::default(),
            ShortcutPlatform::Mac,
        );
        assert!(
            matches!(rows, Err(CommandPresentationError::Scopes(reason)) if reason == expected)
        );
        let dispatch = inherited.resolve(CommandRequest::Input(&input), projection);
        assert_eq!(dispatch.status, CommandDispatchStatus::Suppressed(expected));
        assert!(dispatch.message.is_none());
    }
    let invalid = service.with_application_scopes(CommandScopeProjection::new(
        &[],
        Some(CommandSuppression::InvalidScopes),
    ));
    assert_eq!(
        invalid
            .resolve(
                CommandRequest::Input(&input),
                CommandScopeProjection::empty()
            )
            .status,
        CommandDispatchStatus::Suppressed(CommandSuppression::InvalidScopes)
    );
}
