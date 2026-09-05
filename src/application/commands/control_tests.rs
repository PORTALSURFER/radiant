use super::*;
use crate::{
    application::{TextKey, ViewNode, button, column},
    gui::{focus::FocusSurface, shortcuts::ShortcutPlatform},
    layout::Vector2,
    runtime::SurfaceRuntime,
    widgets::{WidgetInput, WidgetKey},
};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

fn id() -> CommandId {
    CommandId::new("save").unwrap()
}
fn registry() -> CommandRegistry {
    CommandRegistry::new([CommandDescriptor::new(id(), TextKey::new("save", "Save"))
        .default_binding(CommandShortcut::new(CommandKey::Character("s".into())).primary())])
    .unwrap()
}
fn scope(enabled: bool) -> CommandScope<u32> {
    CommandScope::new(
        "editor",
        CommandScopeKind::Editor { depth: 0 },
        [CommandBinding::new(id(), 42).enabled(enabled)],
    )
    .unwrap()
}
fn view(
    registry: &CommandRegistry,
    scope: &CommandScope<u32>,
    show: bool,
) -> ViewNode<(u32, CommandSource)> {
    let presentation = registry
        .present(
            std::slice::from_ref(scope),
            &Keymap::new(),
            &id(),
            &Default::default(),
            ShortcutPlatform::Mac,
        )
        .unwrap();
    let mut children = Vec::new();
    if show {
        children.push(
            button("Editor")
                .message((0, CommandSource::Application))
                .id(101)
                .command_scope(scope.clone()),
        );
    }
    children.push(presentation.toolbar_button().id(102));
    children.push(
        button("Unrelated")
            .message((0, CommandSource::Application))
            .id(103),
    );
    column(children).id(100)
}

#[test]
fn command_control_preserves_editor_context_and_maps_pointer_keyboard_and_shortcut_once() {
    let registry = registry();
    let projection = registry.clone();
    let calls = Rc::new(RefCell::new(Vec::new()));
    let observed = Rc::clone(&calls);
    let mut runtime = SurfaceRuntime::new(
        crate::app(scope(true))
            .view(move |state: &CommandScope<u32>| view(&projection, state, true))
            .command_registry(
                registry,
                CommandDispatcher::new(|invocation: CommandInvocation<u32>| {
                    (*invocation.context(), invocation.source())
                }),
            )
            .update(move |_, message| observed.borrow_mut().push(message))
            .into_bridge(),
        Vector2::new(400.0, 200.0),
    );
    assert!(runtime.focus_widget(101));
    let point = runtime.layout().rects[&102].center();
    runtime.dispatch_input_at(point, WidgetInput::primary_press(point));
    runtime.dispatch_input_at(point, WidgetInput::primary_release(point));
    assert_eq!(runtime.focused_widget(), Some(102));
    assert_eq!(*calls.borrow(), [(42, CommandSource::Toolbar)]);
    runtime.dispatch_focused_input(WidgetInput::key_press(WidgetKey::Enter));
    assert_eq!(
        *calls.borrow(),
        [(42, CommandSource::Toolbar), (42, CommandSource::Toolbar)]
    );
    let mut input = CommandInput::logical(CommandKey::Character("s".into()), ShortcutPlatform::Mac);
    input.modifiers.meta = true;
    let (status, outcome) =
        runtime.dispatch_command_request(CommandRequest::Input(&input), FocusSurface::None);
    assert_eq!(status, CommandDispatchStatus::Mapped);
    assert_eq!(outcome.messages_dispatched, 1);
    assert_eq!(calls.borrow()[2], (42, CommandSource::Shortcut));
    assert!(runtime.focus_widget(103));
    assert!(runtime.focus_widget(102));
    runtime.dispatch_focused_input(WidgetInput::key_press(WidgetKey::Enter));
    assert_eq!(calls.borrow().len(), 3);
}

#[test]
fn command_control_does_not_restore_a_removed_editor_context_when_identity_reappears() {
    let registry = registry();
    let projection = registry.clone();
    let show = Rc::new(Cell::new(true));
    let shown = Rc::clone(&show);
    let calls = Rc::new(Cell::new(0));
    let observed = Rc::clone(&calls);
    let mut runtime = SurfaceRuntime::new(
        crate::app(scope(true))
            .view(move |state: &CommandScope<u32>| view(&projection, state, shown.get()))
            .command_registry(
                registry,
                CommandDispatcher::new(|invocation: CommandInvocation<u32>| {
                    (*invocation.context(), invocation.source())
                }),
            )
            .update(move |_, _| observed.set(observed.get() + 1))
            .into_bridge(),
        Vector2::new(400.0, 200.0),
    );
    assert!(runtime.focus_widget(101));
    assert!(runtime.focus_widget(102));
    show.set(false);
    runtime.refresh();
    show.set(true);
    runtime.refresh();
    runtime.dispatch_focused_input(WidgetInput::key_press(WidgetKey::Enter));
    assert_eq!(calls.get(), 0);
    assert!(runtime.focus_widget(101));
    assert!(runtime.focus_widget(102));
    runtime.dispatch_focused_input(WidgetInput::key_press(WidgetKey::Enter));
    assert_eq!(calls.get(), 1);
}

#[test]
fn disabled_command_controls_are_visible_but_cannot_focus_or_reduce() {
    let registry = registry();
    let projection = registry.clone();
    let mut runtime = SurfaceRuntime::new(
        crate::app(scope(false))
            .view(move |state: &CommandScope<u32>| view(&projection, state, true))
            .command_registry(
                registry,
                CommandDispatcher::new(|_: CommandInvocation<u32>| {
                    panic!("disabled command reached mapper")
                }),
            )
            .update(|_, _: (u32, CommandSource)| panic!("disabled command reached reducer"))
            .into_bridge(),
        Vector2::new(400.0, 200.0),
    );
    assert!(!runtime.focus_widget(102));
    let point = runtime.layout().rects[&102].center();
    runtime.dispatch_input_at(point, WidgetInput::primary_press(point));
    runtime.dispatch_input_at(point, WidgetInput::primary_release(point));
}

#[test]
fn menu_and_palette_controls_use_one_mapper_with_their_presentation_source() {
    struct Message(CommandSource);
    for source in [CommandSource::Menu, CommandSource::Palette] {
        let registry = registry();
        let projection = registry.clone();
        let state = CommandScope::new(
            "window",
            CommandScopeKind::Window,
            [CommandBinding::new(id(), 42u32)],
        )
        .unwrap();
        let calls = Rc::new(RefCell::new(Vec::new()));
        let observed = Rc::clone(&calls);
        let mut runtime = SurfaceRuntime::new(
            crate::app(state)
                .view(move |state: &CommandScope<u32>| {
                    let presentation = projection
                        .present(
                            std::slice::from_ref(state),
                            &Keymap::new(),
                            &id(),
                            &Default::default(),
                            ShortcutPlatform::Mac,
                        )
                        .unwrap();
                    let control = match source {
                        CommandSource::Menu => presentation.menu_item(),
                        _ => presentation.palette_item(),
                    };
                    control.id(102).command_scope(state.clone())
                })
                .command_registry(
                    registry,
                    CommandDispatcher::new(|invocation: CommandInvocation<u32>| {
                        Message(invocation.source())
                    }),
                )
                .update(move |_, message: Message| observed.borrow_mut().push(message.0))
                .into_bridge(),
            Vector2::new(400.0, 200.0),
        );
        assert!(runtime.focus_widget(102));
        runtime.dispatch_focused_input(WidgetInput::key_press(WidgetKey::Enter));
        assert_eq!(*calls.borrow(), [source]);
    }
}

#[test]
fn command_control_release_cannot_retarget_a_press_after_scope_replacement() {
    let registry = registry();
    let projection = registry.clone();
    let make_scope = |context| {
        CommandScope::new(
            "window",
            CommandScopeKind::Window,
            [CommandBinding::new(id(), context)],
        )
        .unwrap()
    };
    let state = Rc::new(RefCell::new(make_scope(42u32)));
    let changed = Rc::clone(&state);
    let calls = Rc::new(RefCell::new(Vec::new()));
    let observed = Rc::clone(&calls);
    let mut runtime = SurfaceRuntime::new(
        crate::app(state)
            .view(move |state: &Rc<RefCell<CommandScope<u32>>>| {
                let scope = state.borrow().clone();
                projection
                    .present(
                        std::slice::from_ref(&scope),
                        &Keymap::new(),
                        &id(),
                        &Default::default(),
                        ShortcutPlatform::Mac,
                    )
                    .unwrap()
                    .toolbar_button()
                    .id(102)
                    .command_scope(scope)
            })
            .command_registry(
                registry,
                CommandDispatcher::new(|invocation: CommandInvocation<u32>| *invocation.context()),
            )
            .update(move |_, message| observed.borrow_mut().push(message))
            .into_bridge(),
        Vector2::new(400.0, 200.0),
    );
    let point = runtime.layout().rects[&102].center();
    runtime.dispatch_input_at(point, WidgetInput::primary_press(point));
    *changed.borrow_mut() = make_scope(43);
    runtime.refresh();
    runtime.dispatch_input_at(point, WidgetInput::primary_release(point));
    assert!(calls.borrow().is_empty());
    runtime.dispatch_input_at(point, WidgetInput::primary_press(point));
    runtime.dispatch_input_at(point, WidgetInput::primary_release(point));
    assert_eq!(*calls.borrow(), [43]);
}
