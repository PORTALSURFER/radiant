use super::*;
use crate::gui_runtime::{
    NativeRunOptions,
    native_vello::generic_runtime::{GenericNativeVelloRunner, key_code_from_winit},
};
use crate::{application::*, gui::focus::FocusSurface, runtime::RuntimeBridge};
use std::{cell::RefCell, rc::Rc};
use winit::keyboard::{KeyCode, NamedKey};

#[derive(Clone)]
enum Message {
    Command(u32),
    Text(String),
}
struct State {
    scopes: Vec<CommandScope<u32>>,
    text: String,
}
fn id(value: &str) -> CommandId {
    CommandId::new(value).unwrap()
}
fn runner(
    key: CommandKey,
    repeat: bool,
    observed: Rc<RefCell<Vec<u32>>>,
) -> GenericNativeVelloRunner<impl RuntimeBridge<Message>, Message> {
    let registry = CommandRegistry::new([CommandDescriptor::new(
        id("command"),
        TextKey::new("command", "Command"),
    )
    .default_binding(CommandShortcut::new(key))
    .repeats(repeat)])
    .unwrap();
    runner_with_registry(registry, observed)
}
fn runner_with_registry(
    registry: CommandRegistry,
    observed: Rc<RefCell<Vec<u32>>>,
) -> GenericNativeVelloRunner<impl RuntimeBridge<Message>, Message> {
    let bindings: Vec<_> = registry
        .commands()
        .map(|command| CommandBinding::new(command.id().clone(), 7))
        .collect();
    let state = State {
        text: String::new(),
        scopes: vec![CommandScope::new("window", CommandScopeKind::Window, bindings).unwrap()],
    };
    GenericNativeVelloRunner::new(
        NativeRunOptions::default(),
        crate::app(state)
            .view(|state: &State| text_input(state.text.clone()).message(Message::Text).id(11))
            .commands(
                registry,
                |state, _| CommandSnapshot {
                    keymap: Keymap::new(),
                    scopes: state.scopes.clone(),
                },
                CommandDispatcher::new(|invocation: CommandInvocation<u32>| {
                    Message::Command(*invocation.context())
                }),
            )
            .shortcuts(|_, _, _, _| {
                crate::gui::shortcuts::ShortcutResolution::action(Message::Command(99))
            })
            .update(move |state, message| match message {
                Message::Command(value) => observed.borrow_mut().push(value),
                Message::Text(value) => {
                    observed
                        .borrow_mut()
                        .push(1000 + value.chars().count() as u32);
                    state.text = value;
                }
            })
            .into_bridge(),
        crate::layout::Vector2::new(320.0, 80.0),
    )
}
fn press<B: RuntimeBridge<Message>>(
    runner: &mut GenericNativeVelloRunner<B, Message>,
    logical: Key,
    code: KeyCode,
    repeat: bool,
) {
    let input = command_input(
        &logical,
        PhysicalKey::Code(code),
        runner.input.modifiers,
        repeat,
        runner.core.managed_composition_is_active(),
    );
    let text = match &logical {
        Key::Character(text) => Some(text.as_str()),
        _ => None,
    };
    runner.route_native_key_press_inner(key_code_from_winit(code), &logical, text, None, input);
}
#[test]
fn native_normalization_keeps_logical_layout_text_and_full_positional_codes_distinct() {
    let input = command_input(
        &Key::Character("z".into()),
        PhysicalKey::Code(KeyCode::KeyY),
        ModifiersState::CONTROL | ModifiersState::SUPER,
        true,
        false,
    );
    assert_eq!(input.logical, Some(CommandKey::Character("z".into())));
    assert_eq!(input.physical.as_deref(), Some("KeyY"));
    assert!(input.repeat && input.modifiers.control && input.modifiers.meta);
    let named = command_input(
        &Key::Named(NamedKey::F24),
        PhysicalKey::Code(KeyCode::F24),
        ModifiersState::empty(),
        false,
        false,
    );
    assert_eq!(named.logical, Some(CommandKey::Named("F24".into())));
    assert_eq!(named.physical.as_deref(), Some("F24"));
    let dead = command_input(
        &Key::Dead(Some('´')),
        PhysicalKey::Code(KeyCode::Quote),
        ModifiersState::empty(),
        false,
        false,
    );
    assert!(dead.composing);
    assert!(dead.logical.is_none());
}
#[test]
fn logical_and_physical_commands_reach_the_application_reducer_through_native_routing() {
    for (binding, logical, code) in [
        (CommandKey::Character("z".into()), "z", KeyCode::KeyY),
        (CommandKey::Physical("KeyZ".into()), "y", KeyCode::KeyZ),
        (CommandKey::Physical("F24".into()), "", KeyCode::F24),
    ] {
        let observed = Rc::new(RefCell::new(Vec::new()));
        let mut runner = runner(binding, false, Rc::clone(&observed));
        let logical = if logical.is_empty() {
            Key::Named(NamedKey::F24)
        } else {
            Key::Character(logical.into())
        };
        press(&mut runner, logical, code, false);
        assert_eq!(*observed.borrow(), [7]);
    }
}
#[test]
fn native_repeat_uses_the_registered_command_policy() {
    for allowed in [false, true] {
        let observed = Rc::new(RefCell::new(Vec::new()));
        let mut runner = runner(
            CommandKey::Character("s".into()),
            allowed,
            Rc::clone(&observed),
        );
        press(
            &mut runner,
            Key::Character("s".into()),
            KeyCode::KeyS,
            false,
        );
        press(&mut runner, Key::Character("s".into()), KeyCode::KeyS, true);
        assert_eq!(observed.borrow().len(), if allowed { 2 } else { 1 });
    }
}
#[test]
fn focused_text_typing_preempts_logical_commands_even_without_a_legacy_key_code() {
    let observed = Rc::new(RefCell::new(Vec::new()));
    let mut runner = runner(
        CommandKey::Character("λ".into()),
        false,
        Rc::clone(&observed),
    );
    assert!(runner.core.runtime.focus_widget(11));
    let logical = Key::Character("λ".into());
    let input = command_input(
        &logical,
        PhysicalKey::Code(KeyCode::IntlRo),
        ModifiersState::empty(),
        false,
        false,
    );
    runner.route_native_key_press_inner(None, &logical, Some("λ"), None, input.clone());
    assert_eq!(*observed.borrow(), [1001]);
    let mut composing = input;
    composing.composing = true;
    let (status, outcome) = runner
        .core
        .runtime
        .dispatch_command_request(CommandRequest::Input(&composing), FocusSurface::None);
    assert_eq!(
        status,
        CommandDispatchStatus::Suppressed(CommandSuppression::Composition)
    );
    assert_eq!(outcome.messages_dispatched, 0);
}

#[test]
fn native_conflicts_are_terminal_but_unmatched_keys_reach_legacy_once() {
    let observed = Rc::new(RefCell::new(Vec::new()));
    let registry = CommandRegistry::new([
        CommandDescriptor::new(id("logical"), TextKey::new("logical", "Logical"))
            .default_binding(CommandShortcut::new(CommandKey::Character("z".into()))),
        CommandDescriptor::new(id("physical"), TextKey::new("physical", "Physical"))
            .default_binding(CommandShortcut::new(CommandKey::Physical("KeyY".into()))),
    ])
    .unwrap();
    let mut runner = runner_with_registry(registry, Rc::clone(&observed));
    press(
        &mut runner,
        Key::Character("z".into()),
        KeyCode::KeyY,
        false,
    );
    assert!(observed.borrow().is_empty());
    press(
        &mut runner,
        Key::Character("e".into()),
        KeyCode::KeyE,
        false,
    );
    assert_eq!(*observed.borrow(), [99]);
}

#[test]
fn focused_editor_permits_explicit_command_repeat_but_composition_blocks_it() {
    let observed = Rc::new(RefCell::new(Vec::new()));
    let registry =
        CommandRegistry::new([
            CommandDescriptor::new(id("save"), TextKey::new("save", "Save"))
                .default_binding(CommandShortcut::new(CommandKey::Character("s".into())).primary())
                .repeats(true),
        ])
        .unwrap();
    let mut runner = runner_with_registry(registry, Rc::clone(&observed));
    runner.input.modifiers =
        if crate::gui_runtime::native_vello::generic_runtime::input::native_shortcut_platform()
            == ShortcutPlatform::Mac
        {
            ModifiersState::SUPER
        } else {
            ModifiersState::CONTROL
        };
    assert!(runner.core.runtime.focus_widget(11));
    press(
        &mut runner,
        Key::Character("s".into()),
        KeyCode::KeyS,
        false,
    );
    press(&mut runner, Key::Character("s".into()), KeyCode::KeyS, true);
    assert_eq!(*observed.borrow(), [7, 7]);
    runner.route_native_ime_event(winit::event::Ime::Preedit("あ".into(), Some((0, 3))));
    assert!(runner.core.managed_composition_is_active());
    let before = observed.borrow().len();
    press(
        &mut runner,
        Key::Character("s".into()),
        KeyCode::KeyS,
        false,
    );
    assert_eq!(observed.borrow().len(), before);
}

#[test]
fn required_text_policy_distinguishes_editing_from_application_shortcuts() {
    use crate::gui::input::KeyCode as Legacy;
    assert!(required_text_key(
        Some(Legacy::V),
        Some("v"),
        ModifiersState::SUPER
    ));
    assert!(required_text_key(
        Some(Legacy::ArrowLeft),
        None,
        ModifiersState::ALT
    ));
    assert!(required_text_key(None, Some("λ"), ModifiersState::empty()));
    assert!(!required_text_key(
        None,
        Some("\u{1b}"),
        ModifiersState::empty()
    ));
    assert!(!required_text_key(
        Some(Legacy::S),
        Some("s"),
        ModifiersState::SUPER
    ));
}

#[test]
fn logical_editing_without_physical_identity_preempts_a_registered_command() {
    let observed = Rc::new(RefCell::new(Vec::new()));
    let mut runner = runner(
        CommandKey::Named("Backspace".into()),
        false,
        Rc::clone(&observed),
    );
    assert!(runner.core.runtime.focus_widget(11));
    let logical = Key::Character("λ".into());
    let mut input = CommandInput::logical(CommandKey::Character("λ".into()), ShortcutPlatform::Mac);
    runner.route_native_key_press_inner(None, &logical, None, None, input.clone());
    assert_eq!(*observed.borrow(), [1001]);
    input.logical = Some(CommandKey::Named("Backspace".into()));
    runner.route_native_key_press_inner(None, &Key::Named(NamedKey::Backspace), None, None, input);
    assert_eq!(*observed.borrow(), [1001, 1000]);
}

#[test]
fn layout_remapped_editing_shortcuts_stay_with_the_text_owner() {
    let observed = Rc::new(RefCell::new(Vec::new()));
    let registry = CommandRegistry::new([CommandDescriptor::new(
        id("global-cut"),
        TextKey::new("cut", "Cut"),
    )
    .default_binding(CommandShortcut::new(CommandKey::Character("x".into())).primary())])
    .unwrap();
    let mut runner = runner_with_registry(registry, Rc::clone(&observed));
    runner.input.clipboard = None;
    assert!(runner.core.runtime.focus_widget(11));
    press(
        &mut runner,
        Key::Character("λ".into()),
        KeyCode::IntlRo,
        false,
    );
    runner.input.modifiers =
        if crate::gui_runtime::native_vello::generic_runtime::input::native_shortcut_platform()
            == ShortcutPlatform::Mac
        {
            ModifiersState::SUPER
        } else {
            ModifiersState::CONTROL
        };
    press(
        &mut runner,
        Key::Character("a".into()),
        KeyCode::KeyQ,
        false,
    );
    press(
        &mut runner,
        Key::Character("x".into()),
        KeyCode::KeyY,
        false,
    );
    assert_eq!(*observed.borrow(), [1001, 1000]);
}
