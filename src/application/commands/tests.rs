use super::*;
use crate::{
    application::{ApplicationEnvironment, LocaleId, TextCatalog, TextKey},
    gui::shortcuts::ShortcutPlatform,
    runtime::ResolvedEnvironment,
};
use std::{cell::Cell, rc::Rc, sync::Arc};

fn id(value: &str) -> CommandId {
    CommandId::new(value).unwrap()
}
fn shortcut(value: &str) -> CommandShortcut {
    CommandShortcut::new(CommandKey::Character(value.into()))
}
fn registry_fixture() -> CommandRegistry {
    CommandRegistry::new([
        CommandDescriptor::new(
            id("edit.delete"),
            TextKey::new("delete", "Delete Selection"),
        )
        .default_binding(shortcut("x")),
        CommandDescriptor::new(id("edit.other"), TextKey::new("other", "Other"))
            .default_binding(shortcut("y")),
    ])
    .unwrap()
}
fn scope(name: &str, kind: CommandScopeKind, enabled: bool) -> CommandScope<String> {
    CommandScope::new(
        name,
        kind,
        [CommandBinding::new(id("edit.delete"), name.to_owned()).enabled(enabled)],
    )
    .unwrap()
}
fn press(value: &str) -> CommandInput {
    CommandInput::logical(CommandKey::Character(value.into()), ShortcutPlatform::Mac)
}
fn invoked_context(resolution: CommandResolution<String>) -> String {
    match resolution {
        CommandResolution::Invoked(invocation) => invocation.context().clone(),
        _ => panic!("expected one invocation"),
    }
}

#[test]
fn documented_precedence_is_independent_of_input_order_and_disabled_scopes_decline() {
    let registry = registry_fixture();
    let kinds = [
        CommandScopeKind::Application,
        CommandScopeKind::Window,
        CommandScopeKind::Selection,
        CommandScopeKind::Editor { depth: 1 },
        CommandScopeKind::Editor { depth: 8 },
        CommandScopeKind::Overlay { order: 2 },
        CommandScopeKind::Modal { order: 1 },
        CommandScopeKind::Modal { order: 4 },
    ];
    for active in 0..kinds.len() {
        let scopes: Vec<_> = kinds
            .iter()
            .enumerate()
            .map(|(index, kind)| scope(&format!("scope-{index}"), *kind, index <= active))
            .collect();
        assert_eq!(
            invoked_context(registry.resolve(&scopes, &Keymap::new(), &press("x"))),
            format!("scope-{active}")
        );
    }
}

#[test]
fn same_precedence_enabled_conflict_has_no_ordered_winner() {
    let registry = CommandRegistry::new([
        CommandDescriptor::new(id("a"), TextKey::new("a", "A")).default_binding(shortcut("x")),
        CommandDescriptor::new(id("b"), TextKey::new("b", "B")).default_binding(shortcut("x")),
    ])
    .unwrap();
    for reverse in [false, true] {
        let mut bindings = vec![
            CommandBinding::new(id("a"), ()),
            CommandBinding::new(id("b"), ()),
        ];
        if reverse {
            bindings.reverse();
        }
        let scopes = [
            CommandScope::new("editor", CommandScopeKind::Editor { depth: 1 }, bindings).unwrap(),
        ];
        let CommandResolution::Conflict(conflict) =
            registry.resolve(&scopes, &Keymap::new(), &press("x"))
        else {
            panic!("conflict required")
        };
        assert_eq!(conflict.commands, vec![id("a"), id("b")]);
    }
    let scopes = [
        scope("one", CommandScopeKind::Window, true),
        scope("two", CommandScopeKind::Window, true),
    ];
    assert!(matches!(
        registry_fixture().resolve(&scopes, &Keymap::new(), &press("x")),
        CommandResolution::Conflict(_)
    ));
}

#[test]
fn logical_characters_and_physical_positions_remain_distinct_and_cross_kind_overlap_conflicts() {
    let registry = CommandRegistry::new([
        CommandDescriptor::new(id("logical"), TextKey::new("logical", "Logical"))
            .default_binding(shortcut("x")),
        CommandDescriptor::new(id("physical"), TextKey::new("physical", "Physical"))
            .default_binding(CommandShortcut::new(CommandKey::Physical("KeyZ".into()))),
    ])
    .unwrap();
    let scopes = [CommandScope::new(
        "editor",
        CommandScopeKind::Window,
        [
            CommandBinding::new(id("logical"), "logical".to_owned()),
            CommandBinding::new(id("physical"), "physical".to_owned()),
        ],
    )
    .unwrap()];
    let mut input = press("z");
    input.physical = Some("KeyZ".into());
    assert_eq!(
        invoked_context(registry.resolve(&scopes, &Keymap::new(), &input)),
        "physical"
    );
    input.logical = Some(CommandKey::Character("x".into()));
    assert!(matches!(
        registry.resolve(&scopes, &Keymap::new(), &input),
        CommandResolution::Conflict(_)
    ));
    input.physical = Some("KeyX".into());
    assert_eq!(
        invoked_context(registry.resolve(&scopes, &Keymap::new(), &input)),
        "logical"
    );
}

#[test]
fn primary_modifier_uses_platform_bits_and_extra_modifiers_do_not_match() {
    let registry =
        CommandRegistry::new([
            CommandDescriptor::new(id("save"), TextKey::new("save", "Save"))
                .default_binding(shortcut("s").primary()),
        ])
        .unwrap();
    let scopes = [CommandScope::new(
        "window",
        CommandScopeKind::Window,
        [CommandBinding::new(id("save"), ())],
    )
    .unwrap()];
    for platform in [
        ShortcutPlatform::Mac,
        ShortcutPlatform::Windows,
        ShortcutPlatform::Other,
    ] {
        let mut input = press("s");
        input.platform = platform;
        input.modifiers.meta = platform == ShortcutPlatform::Mac;
        input.modifiers.control = platform != ShortcutPlatform::Mac;
        assert!(matches!(
            registry.resolve(&scopes, &Keymap::new(), &input),
            CommandResolution::Invoked(_)
        ));
        input.modifiers.alt = true;
        assert!(matches!(
            registry.resolve(&scopes, &Keymap::new(), &input),
            CommandResolution::Unhandled
        ));
    }
}

#[test]
fn text_composition_repeat_and_reservations_terminally_preempt_the_single_mapper() {
    let calls = Rc::new(Cell::new(0));
    let observed = Rc::clone(&calls);
    let dispatcher = CommandDispatcher::new(move |_: CommandInvocation<String>| {
        observed.set(observed.get() + 1);
        7
    });
    let registry = registry_fixture();
    let scopes = [scope("window", CommandScopeKind::Window, true)];
    for reason in [
        CommandSuppression::TextEditing,
        CommandSuppression::Composition,
        CommandSuppression::Repeat,
        CommandSuppression::PlatformReserved,
    ] {
        let mut input = press("x");
        match reason {
            CommandSuppression::TextEditing => input.text_consumed = true,
            CommandSuppression::Composition => input.composing = true,
            CommandSuppression::Repeat => input.repeat = true,
            CommandSuppression::PlatformReserved => input.platform_reserved = true,
            _ => unreachable!(),
        }
        let result = dispatcher.input(&registry, &scopes, &Keymap::new(), &input);
        assert_eq!(result.status, CommandDispatchStatus::Suppressed(reason));
        assert!(result.message.is_none());
        assert!(!result.allows_fallback());
    }
    assert_eq!(calls.get(), 0);
    let result = dispatcher.input(&registry, &scopes, &Keymap::new(), &press("x"));
    assert_eq!(result.message, Some(7));
    assert_eq!(calls.get(), 1);
    assert!(
        dispatcher
            .input(&registry, &scopes, &Keymap::new(), &press("q"))
            .allows_fallback()
    );
}

#[test]
fn stale_targets_cannot_invoke_replaced_scope_registry_or_nearer_context() {
    let registry = registry_fixture();
    let original = scope("window", CommandScopeKind::Window, true);
    let scopes = [original.clone()];
    let target = registry.target(&scopes, &id("edit.delete")).unwrap();
    assert!(matches!(
        registry
            .clone()
            .resolve_target(&scopes, &target, CommandSource::Menu),
        CommandResolution::Invoked(_)
    ));
    let replacement = [scope("window", CommandScopeKind::Window, true)];
    assert!(matches!(
        registry.resolve_target(&replacement, &target, CommandSource::Menu),
        CommandResolution::Stale
    ));
    assert!(matches!(
        registry_fixture().resolve_target(&scopes, &target, CommandSource::Menu),
        CommandResolution::Stale
    ));
    let nearer = [
        original,
        scope("editor", CommandScopeKind::Editor { depth: 2 }, true),
    ];
    assert!(matches!(
        registry.resolve_target(&nearer, &target, CommandSource::Menu),
        CommandResolution::Stale
    ));
    let disabled = [scope("window", CommandScopeKind::Window, false)];
    let target = registry.target(&disabled, &id("edit.delete")).unwrap();
    assert!(matches!(
        registry.resolve_target(&disabled, &target, CommandSource::Toolbar),
        CommandResolution::Unavailable
    ));
}

#[test]
fn keymap_round_trip_preserves_invalid_and_unavailable_entries_and_uses_only_valid_overrides() {
    let raw = r#"{"version":1,"future":{"keep":true},"entries":[{"command":"edit.delete","bindings":[{"key":{"kind":"character","value":"d"}}]},{"command":"removed.command","bindings":[{"key":{"kind":"physical","value":"KeyQ"}}]},{"command":"bad","bindings":[{"key":{"kind":"future","value":"whatever"}}]},{"unexpected":[1,2,3]}]}"#;
    let keymap = Keymap::from_json(raw).unwrap();
    let registry = registry_fixture();
    let scopes = [scope("window", CommandScopeKind::Window, true)];
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(raw).unwrap(),
        serde_json::from_str::<serde_json::Value>(&keymap.to_json()).unwrap()
    );
    assert!(matches!(
        registry.resolve(&scopes, &keymap, &press("x")),
        CommandResolution::Unhandled
    ));
    assert!(matches!(
        registry.resolve(&scopes, &keymap, &press("d")),
        CommandResolution::Invoked(_)
    ));
    assert!(
        keymap
            .diagnostics(&registry)
            .iter()
            .any(|item| item.problem == KeymapProblem::UnavailableCommand)
    );
    assert!(
        keymap
            .diagnostics(&registry)
            .iter()
            .any(|item| item.problem == KeymapProblem::InvalidBinding)
    );
    let cleared = keymap
        .clone()
        .override_bindings(&id("edit.delete"), vec![])
        .unwrap();
    assert!(matches!(
        registry.resolve(&scopes, &cleared, &press("x")),
        CommandResolution::Unhandled
    ));
    let defaults = cleared.remove_override(&id("edit.delete")).unwrap();
    assert!(matches!(
        registry.resolve(&scopes, &defaults, &press("x")),
        CommandResolution::Invoked(_)
    ));
    assert_eq!(defaults.entries().len(), 3);
}

#[test]
fn duplicate_overrides_are_inactive_instead_of_last_entry_winning() {
    let keymap=Keymap::from_json(r#"{"version":1,"entries":[{"command":"edit.delete","bindings":[]},{"command":"edit.delete","bindings":[{"key":{"kind":"character","value":"d"}}]}]}"#).unwrap();
    let registry = registry_fixture();
    let scopes = [scope("window", CommandScopeKind::Window, true)];
    assert!(matches!(
        registry.resolve(&scopes, &keymap, &press("x")),
        CommandResolution::Invoked(_)
    ));
    assert!(matches!(
        registry.resolve(&scopes, &keymap, &press("d")),
        CommandResolution::Unhandled
    ));
    assert_eq!(
        keymap
            .diagnostics(&registry)
            .iter()
            .filter(|item| item.problem == KeymapProblem::DuplicateCommand)
            .count(),
        2
    );
}

#[test]
fn keymap_validation_reports_reserved_required_conflicts_and_deliberate_shadowing_without_applying()
{
    let registry = registry_fixture();
    let keymap = Keymap::new();
    let scopes = [
        CommandScope::new(
            "editor",
            CommandScopeKind::Editor { depth: 1 },
            [
                CommandBinding::new(id("edit.delete"), ()),
                CommandBinding::new(id("edit.other"), ()),
            ],
        )
        .unwrap(),
        CommandScope::new(
            "window",
            CommandScopeKind::Window,
            [CommandBinding::new(id("edit.other"), ())],
        )
        .unwrap(),
    ];
    let reserved = [shortcut("y")];
    let required = [shortcut("y")];
    let report = keymap
        .validate_override(
            &id("edit.delete"),
            &[shortcut("y")],
            KeymapValidation {
                registry: &registry,
                scopes: &scopes,
                platform: ShortcutPlatform::Mac,
                reserved: &reserved,
                text_required: &required,
            },
        )
        .unwrap();
    for kind in [
        KeymapConflictKind::SameScope,
        KeymapConflictKind::Shadowed,
        KeymapConflictKind::PlatformReserved,
        KeymapConflictKind::TextEditing,
    ] {
        assert!(report.iter().any(|item| item.kind == kind));
    }
    assert!(matches!(
        registry.resolve(&scopes, &keymap, &press("x")),
        CommandResolution::Invoked(_)
    ));
    assert!(keymap.entries().is_empty());
}

#[test]
fn one_localized_projection_supplies_label_checked_state_shortcuts_and_a_current_target() {
    let registry = registry_fixture();
    let scopes = [CommandScope::new(
        "window",
        CommandScopeKind::Window,
        [CommandBinding::new(id("edit.delete"), ()).checked(Some(true))],
    )
    .unwrap()];
    let locale = LocaleId::new("fr").unwrap();
    let catalog = TextCatalog::default().insert(
        locale.clone(),
        TextKey::new("delete", "Delete Selection"),
        "Supprimer la sélection",
    );
    let environment = ResolvedEnvironment::from_snapshots(
        Default::default(),
        Arc::new(ApplicationEnvironment::new(locale).with_catalog(Arc::new(catalog))),
    );
    let presentation = registry
        .present(
            &scopes,
            &Keymap::new(),
            &id("edit.delete"),
            &environment,
            ShortcutPlatform::Mac,
        )
        .unwrap();
    assert_eq!(presentation.label, "Supprimer la sélection");
    assert_eq!(presentation.accessibility, presentation.label);
    assert_eq!(presentation.checked, Some(true));
    assert!(presentation.enabled);
    assert_eq!(presentation.shortcuts[0].compact, "X");
    assert!(matches!(
        registry.resolve_target(
            &scopes,
            &presentation.target.unwrap(),
            CommandSource::Palette
        ),
        CommandResolution::Invoked(_)
    ));
}

#[test]
fn ownership_does_not_require_context_clone_or_message_clone_and_metadata_is_thread_shareable() {
    struct Context(Cell<u32>);
    struct Message(u32);
    let binding = CommandBinding::new(id("edit.delete"), Context(Cell::new(9)));
    let scopes = [
        CommandScope::new("window", CommandScopeKind::Window, [binding.clone()])
            .unwrap()
            .clone(),
    ];
    let dispatcher = CommandDispatcher::new(|invocation: CommandInvocation<Context>| {
        Message(invocation.context().0.get())
    });
    assert_eq!(
        dispatcher
            .input(&registry_fixture(), &scopes, &Keymap::new(), &press("x"))
            .message
            .unwrap()
            .0,
        9
    );
    fn send_sync<T: Send + Sync>() {}
    send_sync::<CommandRegistry>();
    send_sync::<Keymap>();
}

#[test]
fn malformed_inputs_duplicate_scopes_and_oversized_documents_fail_closed() {
    let registry = registry_fixture();
    let scopes = [
        scope("same", CommandScopeKind::Window, true),
        scope("same", CommandScopeKind::Application, true),
    ];
    assert!(matches!(
        registry.resolve(&scopes, &Keymap::new(), &press("x")),
        CommandResolution::Suppressed(CommandSuppression::InvalidScopes)
    ));
    let malformed =
        CommandInput::logical(CommandKey::Physical("KeyX".into()), ShortcutPlatform::Mac);
    assert!(matches!(
        registry.resolve(&[], &Keymap::new(), &malformed),
        CommandResolution::<()>::Suppressed(CommandSuppression::MalformedInput)
    ));
    assert!(matches!(
        Keymap::from_json(&" ".repeat(65_537)),
        Err(KeymapError::Capacity)
    ));
    assert!(matches!(
        Keymap::from_json(r#"{"version":2,"entries":[]}"#),
        Err(KeymapError::UnsupportedVersion)
    ));
}

#[test]
fn ambiguous_disabled_presentation_has_no_incidental_checked_state_or_target() {
    let registry = registry_fixture();
    let scopes = [
        scope("a", CommandScopeKind::Window, false),
        scope("b", CommandScopeKind::Window, false),
    ];
    assert!(registry.target(&scopes, &id("edit.delete")).is_none());
    let mut reversed = scopes.to_vec();
    reversed.reverse();
    assert!(registry.target(&reversed, &id("edit.delete")).is_none());
    let validation = Keymap::new().validate_override(
        &id("edit.delete"),
        &[shortcut("x")],
        KeymapValidation {
            registry: &registry,
            scopes: &[scopes[0].clone(), scopes[0].clone()],
            platform: ShortcutPlatform::Mac,
            reserved: &[],
            text_required: &[],
        },
    );
    assert_eq!(validation.unwrap_err(), KeymapError::InvalidScopes);
}

#[test]
fn explicit_repeat_policy_and_registration_validation_are_enforced() {
    let descriptor = CommandDescriptor::new(id("edit.delete"), TextKey::new("delete", "Delete"))
        .default_binding(shortcut("x"))
        .repeats(true);
    let registry = CommandRegistry::new([descriptor.clone()]).unwrap();
    let mut input = press("x");
    input.repeat = true;
    assert!(matches!(
        registry.resolve(
            &[scope("a", CommandScopeKind::Window, true)],
            &Keymap::new(),
            &input
        ),
        CommandResolution::Invoked(_)
    ));
    assert!(matches!(
        CommandRegistry::new([descriptor.clone(), descriptor]),
        Err(CommandRegistrationError::DuplicateId(_))
    ));
    assert!(CommandId::new("bad\nidentity").is_err());
    assert!(matches!(
        CommandRegistry::new([CommandDescriptor::new(
            id("invalid"),
            TextKey::new("invalid", "Invalid")
        )
        .default_binding(CommandShortcut::new(CommandKey::Physical("bad key".into())))]),
        Err(CommandRegistrationError::InvalidBinding(_))
    ));
}
