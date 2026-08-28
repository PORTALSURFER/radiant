use super::super::*;
use crate::{
    gui::{
        focus::FocusSurface,
        input::{InputTimestamp, KeyCode, KeyPress},
        shortcuts::ShortcutResolution,
        types::Rect,
    },
    layout::LayoutOutput,
    runtime::{
        Event, FocusTraversal, PaintPrimitive, RuntimeBridge, RuntimeHostCapabilities,
        RuntimeInputHost, SequentialFocusTraversalDisposition, SurfaceChild, SurfaceNode,
        UiSurface, WidgetMessageMapper,
    },
    theme::ThemeTokens,
    widgets::{
        CanvasMessage, CanvasWidget, KeyboardModifiers, TextEditCommand, TextWidget, Widget,
        WidgetCommon, WidgetId, WidgetInput, WidgetKey, WidgetOutput, WidgetSizing,
    },
};
use std::sync::Arc;
use winit::keyboard::{KeyCode as WinitKeyCode, ModifiersState, PhysicalKey};

#[derive(Clone)]
struct FocusedKeyboardMetadataWidget {
    inner: CanvasWidget,
}

impl FocusedKeyboardMetadataWidget {
    fn new(id: WidgetId) -> Self {
        Self {
            inner: CanvasWidget::new(id, WidgetSizing::fixed(Vector2::new(160.0, 28.0))),
        }
    }
}

impl Widget for FocusedKeyboardMetadataWidget {
    fn common(&self) -> &WidgetCommon {
        Widget::common(&self.inner)
    }

    fn common_mut(&mut self) -> &mut WidgetCommon {
        Widget::common_mut(&mut self.inner)
    }

    fn handle_input(&mut self, bounds: Rect, input: WidgetInput) -> Option<WidgetOutput> {
        CanvasWidget::handle_input(&mut self.inner, bounds, input).map(WidgetOutput::typed)
    }

    fn accepts_text_input(&self) -> bool {
        true
    }

    fn append_paint(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        bounds: Rect,
        layout: &LayoutOutput,
        theme: &ThemeTokens,
    ) {
        Widget::append_paint(&self.inner, primitives, bounds, layout, theme);
    }
}

#[derive(Clone, Debug, PartialEq)]
enum KeyboardTimestampMessage {
    KeyPress {
        modifiers: KeyboardModifiers,
        repeat: bool,
        timestamp: Option<InputTimestamp>,
    },
    KeyRelease {
        key: WidgetKey,
        modifiers: KeyboardModifiers,
        timestamp: Option<InputTimestamp>,
    },
    Character {
        character: char,
        timestamp: Option<InputTimestamp>,
    },
    TextEdit(Option<InputTimestamp>),
    Ignored,
}

#[derive(Default)]
struct KeyboardTimestampBridge {
    messages: Vec<KeyboardTimestampMessage>,
}

impl RuntimeBridge<KeyboardTimestampMessage> for KeyboardTimestampBridge {
    fn project_surface(&mut self) -> Arc<UiSurface<KeyboardTimestampMessage>> {
        crate::runtime::test_arc_surface(UiSurface::new(SurfaceNode::widget(
            FocusedKeyboardMetadataWidget::new(90),
            WidgetMessageMapper::canvas(|message| match message {
                CanvasMessage::Input {
                    input:
                        WidgetInput::KeyPress {
                            modifiers,
                            repeat,
                            timestamp,
                            ..
                        },
                } => KeyboardTimestampMessage::KeyPress {
                    modifiers,
                    repeat,
                    timestamp,
                },
                CanvasMessage::Input {
                    input:
                        WidgetInput::KeyRelease {
                            key,
                            modifiers,
                            timestamp,
                        },
                } => KeyboardTimestampMessage::KeyRelease {
                    key,
                    modifiers,
                    timestamp,
                },
                CanvasMessage::Input {
                    input:
                        WidgetInput::Character {
                            character,
                            timestamp,
                        },
                } => KeyboardTimestampMessage::Character {
                    character,
                    timestamp,
                },
                CanvasMessage::Input {
                    input: WidgetInput::TextEdit { timestamp, .. },
                } => KeyboardTimestampMessage::TextEdit(timestamp),
                CanvasMessage::Input { .. } => KeyboardTimestampMessage::Ignored,
            }),
        )))
    }

    fn reduce_message(&mut self, message: KeyboardTimestampMessage) {
        if !matches!(message, KeyboardTimestampMessage::Ignored) {
            self.messages.push(message);
        }
    }

    fn host_capabilities(&self) -> RuntimeHostCapabilities<Self, KeyboardTimestampMessage> {
        RuntimeHostCapabilities::new().with_input()
    }
}

impl RuntimeInputHost<KeyboardTimestampMessage> for KeyboardTimestampBridge {
    fn resolve_key_press(
        &mut self,
        _pending_chord: Option<KeyPress>,
        press: KeyPress,
        _focus: FocusSurface,
    ) -> ShortcutResolution<KeyboardTimestampMessage> {
        if press.key == KeyCode::ArrowUp {
            ShortcutResolution::handled()
        } else {
            ShortcutResolution::unhandled()
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum FocusedKeyRouteMessage {
    Press {
        key: WidgetKey,
        modifiers: KeyboardModifiers,
        repeat: bool,
        timestamp: Option<InputTimestamp>,
    },
    Release {
        key: WidgetKey,
        modifiers: KeyboardModifiers,
        timestamp: Option<InputTimestamp>,
    },
}

#[derive(Clone)]
struct FocusedKeyRoutingWidget {
    common: WidgetCommon,
    captured: Option<WidgetKey>,
    cancel_escape: bool,
}

impl FocusedKeyRoutingWidget {
    fn new(id: WidgetId, cancel_escape: bool) -> Self {
        Self {
            common: WidgetCommon::new(id, WidgetSizing::fixed(Vector2::new(160.0, 28.0)))
                .with_keyboard_focus(),
            captured: None,
            cancel_escape,
        }
    }
}

impl Widget for FocusedKeyRoutingWidget {
    fn common(&self) -> &WidgetCommon {
        &self.common
    }

    fn common_mut(&mut self) -> &mut WidgetCommon {
        &mut self.common
    }

    fn handle_input(&mut self, _bounds: Rect, input: WidgetInput) -> Option<WidgetOutput> {
        match input {
            WidgetInput::KeyPress {
                key,
                modifiers,
                repeat,
                timestamp,
            } => {
                if !repeat && key == WidgetKey::ArrowUp {
                    self.captured = Some(key);
                } else if !repeat && key == WidgetKey::Escape && self.cancel_escape {
                    self.captured = None;
                }
                Some(WidgetOutput::typed(FocusedKeyRouteMessage::Press {
                    key,
                    modifiers,
                    repeat,
                    timestamp,
                }))
            }
            WidgetInput::KeyRelease {
                key,
                modifiers,
                timestamp,
            } => {
                if self.captured == Some(key) {
                    self.captured = None;
                }
                Some(WidgetOutput::typed(FocusedKeyRouteMessage::Release {
                    key,
                    modifiers,
                    timestamp,
                }))
            }
            _ => None,
        }
    }

    fn synchronize_from_previous(&mut self, previous: &dyn Widget) {
        if let Some(previous) = previous.as_any().downcast_ref::<Self>() {
            self.captured = previous.captured;
        }
    }

    fn participates_in_focused_key_routing(&self) -> bool {
        true
    }

    fn captured_focused_key(&self) -> Option<WidgetKey> {
        self.captured
    }

    fn preempts_host_shortcut_key(&self, key: WidgetKey) -> bool {
        self.cancel_escape && key == WidgetKey::Escape
    }

    fn append_paint(
        &self,
        _primitives: &mut Vec<PaintPrimitive>,
        _bounds: Rect,
        _layout: &LayoutOutput,
        _theme: &ThemeTokens,
    ) {
    }
}

struct FocusedKeyRoutingBridge {
    messages: Vec<FocusedKeyRouteMessage>,
    host_presses: Vec<KeyPress>,
    host_handled: bool,
    host_binding: Option<KeyPress>,
    cancel_escape: bool,
}

impl FocusedKeyRoutingBridge {
    fn new(host_handled: bool, cancel_escape: bool) -> Self {
        Self {
            messages: Vec::new(),
            host_presses: Vec::new(),
            host_handled,
            host_binding: None,
            cancel_escape,
        }
    }

    #[cfg(not(target_os = "macos"))]
    fn with_host_binding(host_binding: KeyPress, cancel_escape: bool) -> Self {
        Self {
            messages: Vec::new(),
            host_presses: Vec::new(),
            host_handled: false,
            host_binding: Some(host_binding),
            cancel_escape,
        }
    }
}

impl RuntimeBridge<FocusedKeyRouteMessage> for FocusedKeyRoutingBridge {
    fn project_surface(&mut self) -> Arc<UiSurface<FocusedKeyRouteMessage>> {
        crate::runtime::test_arc_surface(UiSurface::new(SurfaceNode::widget(
            FocusedKeyRoutingWidget::new(91, self.cancel_escape),
            WidgetMessageMapper::typed(|message: FocusedKeyRouteMessage| message),
        )))
    }

    fn reduce_message(&mut self, message: FocusedKeyRouteMessage) {
        self.messages.push(message);
    }

    fn host_capabilities(&self) -> RuntimeHostCapabilities<Self, FocusedKeyRouteMessage> {
        RuntimeHostCapabilities::new().with_input()
    }
}

impl RuntimeInputHost<FocusedKeyRouteMessage> for FocusedKeyRoutingBridge {
    fn resolve_key_press(
        &mut self,
        _pending_chord: Option<KeyPress>,
        press: KeyPress,
        _focus: FocusSurface,
    ) -> ShortcutResolution<FocusedKeyRouteMessage> {
        self.host_presses.push(press);
        if self.host_handled || self.host_binding == Some(press) {
            ShortcutResolution::handled()
        } else {
            ShortcutResolution::unhandled()
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum NativeTabRouteMessage {
    KeyPress {
        widget_id: WidgetId,
        key: WidgetKey,
        modifiers: KeyboardModifiers,
        repeat: bool,
    },
    KeyRelease {
        widget_id: WidgetId,
        key: WidgetKey,
        modifiers: KeyboardModifiers,
    },
    Ignored,
}

fn map_native_tab_canvas_message(
    widget_id: WidgetId,
    message: CanvasMessage,
) -> NativeTabRouteMessage {
    match message {
        CanvasMessage::Input {
            input:
                WidgetInput::KeyPress {
                    key,
                    modifiers,
                    repeat,
                    ..
                },
        } => NativeTabRouteMessage::KeyPress {
            widget_id,
            key,
            modifiers,
            repeat,
        },
        CanvasMessage::Input {
            input: WidgetInput::KeyRelease { key, modifiers, .. },
        } => NativeTabRouteMessage::KeyRelease {
            widget_id,
            key,
            modifiers,
        },
        CanvasMessage::Input { .. } => NativeTabRouteMessage::Ignored,
    }
}

#[derive(Default)]
struct NativeTabBridge {
    messages: Vec<NativeTabRouteMessage>,
    host_presses: Vec<KeyPress>,
}

impl RuntimeBridge<NativeTabRouteMessage> for NativeTabBridge {
    fn project_surface(&mut self) -> Arc<UiSurface<NativeTabRouteMessage>> {
        crate::runtime::test_arc_surface(UiSurface::new(SurfaceNode::row(
            1,
            0.0,
            vec![
                SurfaceChild::fill(SurfaceNode::widget(
                    CanvasWidget::new(101, WidgetSizing::fixed(Vector2::new(120.0, 28.0))),
                    WidgetMessageMapper::canvas(move |message| {
                        map_native_tab_canvas_message(101, message)
                    }),
                )),
                SurfaceChild::fill(SurfaceNode::widget(
                    CanvasWidget::new(102, WidgetSizing::fixed(Vector2::new(120.0, 28.0))),
                    WidgetMessageMapper::canvas(move |message| {
                        map_native_tab_canvas_message(102, message)
                    }),
                )),
            ],
        )))
    }

    fn reduce_message(&mut self, message: NativeTabRouteMessage) {
        if !matches!(message, NativeTabRouteMessage::Ignored) {
            self.messages.push(message);
        }
    }

    fn host_capabilities(&self) -> RuntimeHostCapabilities<Self, NativeTabRouteMessage> {
        RuntimeHostCapabilities::new().with_input()
    }
}

impl RuntimeInputHost<NativeTabRouteMessage> for NativeTabBridge {
    fn resolve_key_press(
        &mut self,
        _pending_chord: Option<KeyPress>,
        press: KeyPress,
        _focus: FocusSurface,
    ) -> ShortcutResolution<NativeTabRouteMessage> {
        self.host_presses.push(press);
        ShortcutResolution::unhandled()
    }
}

fn native_tab_runner() -> GenericNativeVelloRunner<NativeTabBridge, NativeTabRouteMessage> {
    GenericNativeVelloRunner::new(
        NativeRunOptions::default(),
        NativeTabBridge::default(),
        Vector2::new(240.0, 28.0),
    )
}

#[derive(Default)]
struct UnclaimedTabBridge {
    host_presses: Vec<KeyPress>,
}

impl RuntimeBridge<()> for UnclaimedTabBridge {
    fn project_surface(&mut self) -> Arc<UiSurface<()>> {
        crate::runtime::test_arc_surface(UiSurface::new(SurfaceNode::static_widget(
            TextWidget::new(
                201,
                "unclaimed",
                WidgetSizing::fixed(Vector2::new(120.0, 28.0)),
            ),
        )))
    }

    fn host_capabilities(&self) -> RuntimeHostCapabilities<Self, ()> {
        RuntimeHostCapabilities::new().with_input()
    }
}

impl RuntimeInputHost<()> for UnclaimedTabBridge {
    fn resolve_key_press(
        &mut self,
        _pending_chord: Option<KeyPress>,
        press: KeyPress,
        _focus: FocusSurface,
    ) -> ShortcutResolution<()> {
        self.host_presses.push(press);
        ShortcutResolution::unhandled()
    }
}

#[test]
fn native_unclaimed_initial_tab_falls_back_once_without_latching() {
    let mut runner = GenericNativeVelloRunner::new(
        NativeRunOptions::default(),
        UnclaimedTabBridge::default(),
        Vector2::new(120.0, 28.0),
    );

    let initial = runner
        .route_native_tab_for_test(false)
        .expect("initial Tab should be routed");
    assert!(!initial.routed);
    assert!(runner.input.tab_sequence_latch.is_none());
    assert_eq!(runner.core.runtime.bridge().host_presses.len(), 1);

    assert!(runner.route_native_tab_for_test(true).is_none());
    let release = runner
        .route_native_key_release(PhysicalKey::Code(WinitKeyCode::Tab))
        .expect("supported physical release should produce a route outcome");
    assert!(!release.routed);
    assert_eq!(runner.core.runtime.bridge().host_presses.len(), 1);
}

#[test]
fn native_shift_tab_traverses_backward_and_latches_direction() {
    let mut runner = native_tab_runner();
    assert!(runner.core.runtime.focus_widget(102));
    runner.input.modifiers = ModifiersState::SHIFT;

    let outcome = runner
        .route_native_tab_for_test(false)
        .expect("initial Shift-Tab should be routed");
    assert!(outcome.routed);
    assert_eq!(runner.core.runtime.focused_widget(), Some(101));
    assert_eq!(
        runner.input.tab_sequence_latch.map(|latch| latch.direction),
        Some(FocusTraversal::Backward)
    );
    assert!(runner.core.runtime.bridge().host_presses.is_empty());
    assert!(runner.core.runtime.bridge().messages.is_empty());
}

#[test]
fn native_admitted_tab_repeat_and_matching_release_do_not_reach_new_focus() {
    let mut runner = native_tab_runner();
    assert!(runner.core.runtime.focus_widget(101));

    let initial = runner
        .route_native_tab_for_test(false)
        .expect("initial Tab should be routed");
    assert!(initial.routed);
    assert_eq!(runner.core.runtime.focused_widget(), Some(102));
    assert_eq!(
        runner.input.tab_sequence_latch.map(|latch| latch.direction),
        Some(FocusTraversal::Forward)
    );
    let messages_before = runner.core.runtime.bridge().messages.clone();
    let host_presses_before = runner.core.runtime.bridge().host_presses.clone();

    assert!(
        runner
            .route_native_tab_for_test(true)
            .expect("latched Tab repeat should be consumed")
            .routed
    );
    assert!(
        runner
            .route_native_key_release(PhysicalKey::Code(WinitKeyCode::Tab))
            .expect("latched Tab release should be consumed")
            .routed
    );

    assert_eq!(runner.core.runtime.focused_widget(), Some(102));
    assert_eq!(runner.core.runtime.bridge().messages, messages_before);
    assert_eq!(
        runner.core.runtime.bridge().host_presses,
        host_presses_before
    );
    assert!(runner.input.tab_sequence_latch.is_none());
}

#[test]
fn native_terminal_tab_dispositions_consume_repeat_and_matching_release() {
    for disposition in [
        SequentialFocusTraversalDisposition::Vetoed,
        SequentialFocusTraversalDisposition::Invalidated,
    ] {
        let mut runner = native_tab_runner();
        assert!(runner.core.runtime.focus_widget(101));
        runner.input.tab_sequence_latch =
            Some(super::super::super::runner_state::NativeTabSequenceLatch {
                direction: FocusTraversal::Forward,
            });
        let messages_before = runner.core.runtime.bridge().messages.clone();
        let host_presses_before = runner.core.runtime.bridge().host_presses.clone();

        assert!(
            runner
                .route_native_tab_for_test(true)
                .expect("terminal Tab repeat should be consumed")
                .routed,
            "terminal disposition {disposition:?} should consume repeat"
        );
        assert!(
            runner
                .route_native_key_release(PhysicalKey::Code(WinitKeyCode::Tab))
                .expect("terminal Tab release should be consumed")
                .routed,
            "terminal disposition {disposition:?} should consume release"
        );

        assert_eq!(runner.core.runtime.focused_widget(), Some(101));
        assert_eq!(runner.core.runtime.bridge().messages, messages_before);
        assert_eq!(
            runner.core.runtime.bridge().host_presses,
            host_presses_before
        );
        assert!(runner.input.tab_sequence_latch.is_none());
    }
}

#[test]
fn native_command_control_and_alt_tab_bypass_sequential_focus_traversal() {
    for native_modifiers in [
        ModifiersState::SUPER,
        ModifiersState::CONTROL,
        ModifiersState::ALT,
    ] {
        let mut runner = native_tab_runner();
        assert!(runner.core.runtime.focus_widget(101));
        runner.input.modifiers = native_modifiers;

        let outcome = runner
            .route_native_tab_for_test(false)
            .expect("modified Tab should be routed");
        assert!(outcome.routed);
        assert_eq!(runner.core.runtime.focused_widget(), Some(101));
        assert!(runner.input.tab_sequence_latch.is_none());
        assert_eq!(runner.core.runtime.bridge().host_presses.len(), 1);
        assert_eq!(
            runner.core.runtime.bridge().host_presses,
            vec![keypress_from_input(KeyCode::Tab, native_modifiers)]
        );
        assert_eq!(
            runner.core.runtime.bridge().messages,
            vec![NativeTabRouteMessage::KeyPress {
                widget_id: 101,
                key: WidgetKey::Tab,
                modifiers: keyboard_modifiers_from_winit(native_modifiers),
                repeat: false,
            }]
        );
    }
}

#[test]
fn native_focus_loss_clears_tab_sequence_latch() {
    let mut runner = native_tab_runner();
    assert!(runner.core.runtime.focus_widget(101));
    assert!(
        runner
            .route_native_tab_for_test(false)
            .expect("initial Tab should be routed")
            .routed
    );
    assert!(runner.input.tab_sequence_latch.is_some());

    assert!(runner.handle_focus_lost_before_external_drag().routed);
    assert!(runner.input.tab_sequence_latch.is_none());
    assert_eq!(runner.core.runtime.focused_widget(), None);
}

#[test]
fn direct_physical_key_route_preserves_one_timestamp() {
    let timestamp = Some(InputTimestamp::capture());
    let mut core = GenericNativeRuntimeCore::new(
        KeyboardTimestampBridge::default(),
        Vector2::new(160.0, 28.0),
    );

    assert!(core.runtime.focus_widget(90));
    assert!(
        core.route_key_press_with_timestamp(
            KeyPress::new(KeyCode::Enter),
            Some(WidgetKey::Enter),
            KeyboardModifiers::default(),
            timestamp,
            false,
        )
        .routed
    );
    assert_eq!(
        core.runtime.bridge().messages,
        vec![KeyboardTimestampMessage::KeyPress {
            modifiers: KeyboardModifiers::default(),
            repeat: false,
            timestamp,
        }]
    );
}

#[test]
fn direct_physical_key_route_preserves_modifier_and_repeat_metadata() {
    let timestamp = Some(InputTimestamp::capture());
    let mut core = GenericNativeRuntimeCore::new(
        KeyboardTimestampBridge::default(),
        Vector2::new(160.0, 28.0),
    );

    assert!(core.runtime.focus_widget(90));
    assert!(
        core.route_key_press_with_timestamp(
            KeyPress {
                key: KeyCode::ArrowRight,
                command: true,
                control: true,
                shift: true,
                alt: true,
            },
            Some(WidgetKey::ArrowRight),
            KeyboardModifiers {
                command: true,
                control: true,
                shift: true,
                alt: true,
            },
            timestamp,
            true,
        )
        .routed
    );
    assert_eq!(
        core.runtime.bridge().messages,
        vec![KeyboardTimestampMessage::KeyPress {
            modifiers: KeyboardModifiers {
                command: true,
                control: true,
                shift: true,
                alt: true,
            },
            repeat: true,
            timestamp,
        }]
    );
}

#[test]
fn logical_deletion_fallback_preserves_modifier_and_repeat_metadata() {
    let timestamp = Some(InputTimestamp::capture());
    let modifiers = KeyboardModifiers {
        command: true,
        control: true,
        shift: true,
        alt: true,
    };
    let mut core = GenericNativeRuntimeCore::new(
        KeyboardTimestampBridge::default(),
        Vector2::new(160.0, 28.0),
    );

    assert!(core.runtime.focus_widget(90));
    assert!(
        core.route_widget_key_with_metadata(WidgetKey::Backspace, modifiers, true, timestamp)
            .routed
    );
    assert_eq!(
        core.runtime.bridge().messages,
        vec![KeyboardTimestampMessage::KeyPress {
            modifiers,
            repeat: true,
            timestamp,
        }]
    );
}

#[cfg(not(target_os = "macos"))]
#[test]
fn unhandled_native_control_keeps_host_and_widget_modifier_views_distinct() {
    let native_modifiers = ModifiersState::CONTROL;
    let host_press = keypress_from_input(KeyCode::ArrowRight, native_modifiers);
    assert!(host_press.command);
    assert!(!host_press.control);
    let widget_modifiers = keyboard_modifiers_from_winit(native_modifiers);
    assert_eq!(
        widget_modifiers,
        KeyboardModifiers {
            command: false,
            control: true,
            shift: false,
            alt: false,
        }
    );

    let timestamp = Some(InputTimestamp::capture());
    let mut core = GenericNativeRuntimeCore::new(
        KeyboardTimestampBridge::default(),
        Vector2::new(160.0, 28.0),
    );
    assert!(core.runtime.focus_widget(90));
    assert!(
        core.route_key_press_with_timestamp(
            host_press,
            Some(WidgetKey::ArrowRight),
            widget_modifiers,
            timestamp,
            true,
        )
        .routed
    );
    assert_eq!(
        core.runtime.bridge().messages,
        vec![KeyboardTimestampMessage::KeyPress {
            modifiers: widget_modifiers,
            repeat: true,
            timestamp,
        }]
    );
}

#[test]
fn handled_native_host_shortcut_does_not_reach_focused_widget() {
    let native_modifiers = ModifiersState::CONTROL;
    let host_press = keypress_from_input(KeyCode::ArrowUp, native_modifiers);
    let widget_modifiers = keyboard_modifiers_from_winit(native_modifiers);
    let mut core = GenericNativeRuntimeCore::new(
        KeyboardTimestampBridge::default(),
        Vector2::new(160.0, 28.0),
    );
    assert!(core.runtime.focus_widget(90));

    assert!(
        core.route_key_press_with_timestamp(
            host_press,
            Some(WidgetKey::ArrowUp),
            widget_modifiers,
            Some(InputTimestamp::capture()),
            false,
        )
        .routed
    );
    assert!(core.runtime.bridge().messages.is_empty());
}

#[test]
fn focused_native_capture_preserves_metadata_bypasses_host_and_owner_cancels() {
    let initial_timestamp = Some(InputTimestamp::capture());
    let repeat_timestamp = Some(InputTimestamp::capture());
    let cancel_timestamp = Some(InputTimestamp::capture());
    let initial_modifiers = KeyboardModifiers {
        command: true,
        control: true,
        shift: false,
        alt: false,
    };
    let repeat_modifiers = KeyboardModifiers {
        command: false,
        control: true,
        shift: true,
        alt: true,
    };
    let cancel_modifiers = KeyboardModifiers {
        command: true,
        control: false,
        shift: true,
        alt: true,
    };
    let mut core = GenericNativeRuntimeCore::new(
        FocusedKeyRoutingBridge::new(false, true),
        Vector2::new(160.0, 28.0),
    );
    assert!(core.runtime.focus_widget(91));

    assert!(
        core.route_key_press_with_timestamp(
            KeyPress {
                key: KeyCode::ArrowUp,
                command: initial_modifiers.command,
                control: initial_modifiers.control,
                shift: initial_modifiers.shift,
                alt: initial_modifiers.alt,
            },
            Some(WidgetKey::ArrowUp),
            initial_modifiers,
            initial_timestamp,
            false,
        )
        .routed
    );
    assert!(
        core.route_key_press_with_timestamp(
            KeyPress {
                key: KeyCode::ArrowUp,
                command: false,
                control: false,
                shift: true,
                alt: true,
            },
            Some(WidgetKey::ArrowUp),
            repeat_modifiers,
            repeat_timestamp,
            true,
        )
        .routed
    );
    assert!(
        core.route_key_press_with_timestamp(
            KeyPress {
                key: KeyCode::Escape,
                command: cancel_modifiers.command,
                control: cancel_modifiers.control,
                shift: cancel_modifiers.shift,
                alt: cancel_modifiers.alt,
            },
            Some(WidgetKey::Escape),
            cancel_modifiers,
            cancel_timestamp,
            false,
        )
        .routed
    );

    let bridge = core.runtime.bridge();
    assert_eq!(bridge.host_presses.len(), 1);
    assert_eq!(
        bridge.messages,
        vec![
            FocusedKeyRouteMessage::Press {
                key: WidgetKey::ArrowUp,
                modifiers: initial_modifiers,
                repeat: false,
                timestamp: initial_timestamp,
            },
            FocusedKeyRouteMessage::Press {
                key: WidgetKey::ArrowUp,
                modifiers: repeat_modifiers,
                repeat: true,
                timestamp: repeat_timestamp,
            },
            FocusedKeyRouteMessage::Press {
                key: WidgetKey::Escape,
                modifiers: cancel_modifiers,
                repeat: false,
                timestamp: cancel_timestamp,
            },
        ]
    );
}

#[test]
fn native_captured_release_uses_generic_owner_and_preserves_native_modifiers() {
    let initial_modifiers = KeyboardModifiers::default();
    let mut runner = GenericNativeVelloRunner::new(
        NativeRunOptions::default(),
        FocusedKeyRoutingBridge::new(false, false),
        Vector2::new(160.0, 28.0),
    );
    assert!(runner.core.runtime.focus_widget(91));
    assert!(
        runner
            .core
            .route_key_press_with_timestamp(
                KeyPress::new(KeyCode::ArrowUp),
                Some(WidgetKey::ArrowUp),
                initial_modifiers,
                Some(InputTimestamp::capture()),
                false,
            )
            .routed
    );

    runner.input.modifiers = ModifiersState::CONTROL | ModifiersState::SHIFT;
    let outcome = runner
        .route_native_key_release(PhysicalKey::Code(WinitKeyCode::ArrowUp))
        .expect("captured physical release should produce a route outcome");
    assert!(outcome.routed);
    assert_eq!(runner.core.runtime.bridge().host_presses.len(), 1);
    let Some(FocusedKeyRouteMessage::Release {
        key,
        modifiers,
        timestamp,
    }) = runner.core.runtime.bridge().messages.last()
    else {
        panic!("native captured release should deliver a release message");
    };
    assert_eq!(*key, WidgetKey::ArrowUp);
    assert_eq!(
        *modifiers,
        KeyboardModifiers {
            command: false,
            control: true,
            shift: true,
            alt: false,
        }
    );
    assert!(timestamp.is_some());
}

fn run_focused_key_path(
    path: u8,
    initial_timestamp: Option<InputTimestamp>,
    repeat_timestamp: Option<InputTimestamp>,
    release_timestamp: Option<InputTimestamp>,
) -> (Vec<FocusedKeyRouteMessage>, Vec<KeyPress>) {
    let modifiers = KeyboardModifiers {
        command: true,
        control: true,
        shift: true,
        alt: true,
    };
    let mut core = GenericNativeRuntimeCore::new(
        FocusedKeyRoutingBridge::new(false, false),
        Vector2::new(160.0, 28.0),
    );
    assert!(core.runtime.focus_widget(91));
    match path {
        0 => {
            assert_eq!(
                core.runtime.dispatch_event(Event::key_press_with_metadata(
                    WidgetKey::ArrowUp,
                    modifiers,
                    false,
                    initial_timestamp,
                )),
                Some(91)
            );
            assert_eq!(
                core.runtime.dispatch_event(Event::key_press_with_metadata(
                    WidgetKey::ArrowUp,
                    modifiers,
                    true,
                    repeat_timestamp,
                )),
                Some(91)
            );
            assert_eq!(
                core.runtime
                    .dispatch_event(Event::key_release_with_metadata(
                        WidgetKey::ArrowUp,
                        modifiers,
                        release_timestamp,
                    )),
                Some(91)
            );
        }
        1 => {
            assert!(
                core.route_widget_key_with_metadata(
                    WidgetKey::ArrowUp,
                    modifiers,
                    false,
                    initial_timestamp,
                )
                .routed
            );
            assert!(
                core.route_widget_key_with_metadata(
                    WidgetKey::ArrowUp,
                    modifiers,
                    true,
                    repeat_timestamp,
                )
                .routed
            );
            assert!(
                core.route_key_release_with_metadata(
                    WidgetKey::ArrowUp,
                    modifiers,
                    release_timestamp,
                )
                .routed
            );
        }
        2 => {
            assert!(
                core.route_key_press_with_timestamp(
                    keypress_from_input(
                        KeyCode::ArrowUp,
                        ModifiersState::SUPER
                            | ModifiersState::CONTROL
                            | ModifiersState::SHIFT
                            | ModifiersState::ALT,
                    ),
                    Some(WidgetKey::ArrowUp),
                    modifiers,
                    initial_timestamp,
                    false,
                )
                .routed
            );
            assert!(
                core.route_key_press_with_timestamp(
                    KeyPress {
                        key: KeyCode::ArrowUp,
                        command: false,
                        control: false,
                        shift: true,
                        alt: true,
                    },
                    Some(WidgetKey::ArrowUp),
                    modifiers,
                    repeat_timestamp,
                    true,
                )
                .routed
            );
            assert!(
                core.route_key_release_with_metadata(
                    WidgetKey::ArrowUp,
                    modifiers,
                    release_timestamp,
                )
                .routed
            );
        }
        _ => panic!("unsupported focused-key test path"),
    }
    (
        core.runtime.bridge().messages.clone(),
        core.runtime.bridge().host_presses.clone(),
    )
}

#[test]
fn native_direct_and_synthetic_focused_key_paths_are_equivalent() {
    let timestamps = (
        Some(InputTimestamp::capture()),
        Some(InputTimestamp::capture()),
        Some(InputTimestamp::capture()),
    );
    let direct = run_focused_key_path(0, timestamps.0, timestamps.1, timestamps.2);
    let synthetic = run_focused_key_path(1, timestamps.0, timestamps.1, timestamps.2);
    let native = run_focused_key_path(2, timestamps.0, timestamps.1, timestamps.2);
    assert_eq!(direct, synthetic);
    assert_eq!(direct, native);
}

#[cfg(not(target_os = "macos"))]
fn run_control_only_key_path(path: u8) -> (Vec<FocusedKeyRouteMessage>, Vec<KeyPress>) {
    let native_modifiers = ModifiersState::CONTROL;
    let widget_modifiers = keyboard_modifiers_from_winit(native_modifiers);
    let keys = [WidgetKey::ArrowUp, WidgetKey::ArrowRight];

    match path {
        0 => {
            let mut runner = GenericNativeVelloRunner::new(
                NativeRunOptions::default(),
                FocusedKeyRoutingBridge::with_host_binding(
                    KeyPress::with_command(KeyCode::ArrowUp),
                    false,
                ),
                Vector2::new(160.0, 28.0),
            );
            runner.input.modifiers = native_modifiers;
            assert!(runner.core.runtime.focus_widget(91));
            for key in keys {
                assert!(
                    runner
                        .core
                        .route_key_press_with_timestamp(
                            keypress_from_input(key.to_key_code(), native_modifiers),
                            Some(key),
                            widget_modifiers,
                            None,
                            false,
                        )
                        .routed
                );
            }
            (
                runner.core.runtime.bridge().messages.clone(),
                runner.core.runtime.bridge().host_presses.clone(),
            )
        }
        1 => {
            let mut core = GenericNativeRuntimeCore::new(
                FocusedKeyRoutingBridge::with_host_binding(
                    KeyPress::with_command(KeyCode::ArrowUp),
                    false,
                ),
                Vector2::new(160.0, 28.0),
            );
            assert!(core.runtime.focus_widget(91));
            assert_eq!(
                core.runtime.dispatch_event(Event::key_press_with_metadata(
                    WidgetKey::ArrowUp,
                    widget_modifiers,
                    false,
                    None,
                )),
                None
            );
            assert_eq!(
                core.runtime.dispatch_event(Event::key_press_with_metadata(
                    WidgetKey::ArrowRight,
                    widget_modifiers,
                    false,
                    None,
                )),
                Some(91)
            );
            (
                core.runtime.bridge().messages.clone(),
                core.runtime.bridge().host_presses.clone(),
            )
        }
        2 => {
            let mut core = GenericNativeRuntimeCore::new(
                FocusedKeyRoutingBridge::with_host_binding(
                    KeyPress::with_command(KeyCode::ArrowUp),
                    false,
                ),
                Vector2::new(160.0, 28.0),
            );
            assert!(core.runtime.focus_widget(91));
            for key in keys {
                assert!(
                    core.route_widget_key_with_metadata(key, widget_modifiers, false, None)
                        .routed
                );
            }
            (
                core.runtime.bridge().messages.clone(),
                core.runtime.bridge().host_presses.clone(),
            )
        }
        _ => panic!("unsupported control-only key test path"),
    }
}

#[cfg(not(target_os = "macos"))]
#[test]
fn native_direct_and_synthetic_control_only_paths_match_host_and_widget_observations() {
    let native = run_control_only_key_path(0);
    let direct = run_control_only_key_path(1);
    let synthetic = run_control_only_key_path(2);

    assert_eq!(native, direct);
    assert_eq!(native, synthetic);
    assert_eq!(
        native.0,
        vec![FocusedKeyRouteMessage::Press {
            key: WidgetKey::ArrowRight,
            modifiers: KeyboardModifiers {
                command: false,
                control: true,
                shift: false,
                alt: false,
            },
            repeat: false,
            timestamp: None,
        }]
    );
    assert_eq!(
        native.1,
        vec![
            KeyPress::with_command(KeyCode::ArrowUp),
            KeyPress::with_command(KeyCode::ArrowRight),
        ]
    );
}

#[test]
fn native_physical_key_release_routes_once_with_metadata() {
    let mut runner = GenericNativeVelloRunner::new(
        NativeRunOptions::default(),
        KeyboardTimestampBridge::default(),
        Vector2::new(160.0, 28.0),
    );
    runner.input.modifiers = ModifiersState::CONTROL
        | ModifiersState::SUPER
        | ModifiersState::SHIFT
        | ModifiersState::ALT;
    let expected_modifiers = KeyboardModifiers {
        command: true,
        control: true,
        shift: true,
        alt: true,
    };

    assert!(runner.core.runtime.focus_widget(90));
    assert!(
        runner
            .route_native_key_release(PhysicalKey::Code(WinitKeyCode::ArrowDown))
            .expect("supported physical release should produce a route outcome")
            .routed
    );
    let messages = &runner.core.runtime.bridge().messages;
    let Some(KeyboardTimestampMessage::KeyRelease {
        key,
        modifiers,
        timestamp,
    }) = messages.first()
    else {
        panic!("native release should deliver one key-release message");
    };
    assert_eq!(*key, WidgetKey::ArrowDown);
    assert_eq!(*modifiers, expected_modifiers);
    assert!(timestamp.is_some());
    assert_eq!(messages.len(), 1);
}

#[test]
fn unsupported_or_unfocused_key_release_is_not_routed() {
    let mut runner = GenericNativeVelloRunner::new(
        NativeRunOptions::default(),
        KeyboardTimestampBridge::default(),
        Vector2::new(160.0, 28.0),
    );

    assert_eq!(
        runner.route_native_key_release(PhysicalKey::Code(WinitKeyCode::Numpad1)),
        None
    );
    assert!(runner.core.runtime.bridge().messages.is_empty());

    assert!(
        !runner
            .route_native_key_release(PhysicalKey::Code(WinitKeyCode::ArrowDown))
            .expect("supported physical release should produce a route outcome")
            .routed
    );
    assert!(runner.core.runtime.bridge().messages.is_empty());
}

#[test]
fn focused_text_input_enter_and_tab_preserve_native_key_metadata() {
    let enter_timestamp = Some(InputTimestamp::capture());
    let tab_timestamp = Some(InputTimestamp::capture());
    let mut runner = GenericNativeVelloRunner::new(
        NativeRunOptions::default(),
        KeyboardTimestampBridge::default(),
        Vector2::new(160.0, 28.0),
    );

    assert!(runner.core.runtime.focus_widget(90));
    runner.input.modifiers = ModifiersState::SHIFT;

    let mut enter_outcome = GenericRouteOutcome::default();
    assert!(runner.route_focused_text_input_before_shortcuts(
        KeyCode::Enter,
        None,
        enter_timestamp,
        true,
        &mut enter_outcome,
    ));

    let mut tab_outcome = GenericRouteOutcome::default();
    assert!(runner.route_focused_text_input_before_shortcuts(
        KeyCode::Tab,
        None,
        tab_timestamp,
        false,
        &mut tab_outcome,
    ));

    assert_eq!(
        runner.core.runtime.bridge().messages,
        vec![
            KeyboardTimestampMessage::KeyPress {
                modifiers: KeyboardModifiers {
                    command: false,
                    control: false,
                    shift: true,
                    alt: false,
                },
                repeat: true,
                timestamp: enter_timestamp,
            },
            KeyboardTimestampMessage::KeyPress {
                modifiers: KeyboardModifiers {
                    command: false,
                    control: false,
                    shift: true,
                    alt: false,
                },
                repeat: false,
                timestamp: tab_timestamp,
            },
        ]
    );
}

#[test]
fn printable_text_fanout_reuses_one_timestamp_for_every_character() {
    let timestamp = Some(InputTimestamp::capture());
    let mut runner = GenericNativeVelloRunner::new(
        NativeRunOptions::default(),
        KeyboardTimestampBridge::default(),
        Vector2::new(160.0, 28.0),
    );
    assert!(runner.core.runtime.focus_widget(90));

    let mut outcome = GenericRouteOutcome::default();
    assert!(runner.route_text_input_after_unhandled_keypress("éx", timestamp, &mut outcome));
    assert_eq!(
        runner.core.runtime.bridge().messages,
        vec![
            KeyboardTimestampMessage::Character {
                character: 'é',
                timestamp,
            },
            KeyboardTimestampMessage::Character {
                character: 'x',
                timestamp,
            },
        ]
    );
}

#[test]
fn direct_text_edit_route_preserves_timestamp() {
    let timestamp = Some(InputTimestamp::capture());
    let mut core = GenericNativeRuntimeCore::new(
        KeyboardTimestampBridge::default(),
        Vector2::new(160.0, 28.0),
    );
    assert!(core.runtime.focus_widget(90));

    // The canvas accepts the normalized command directly; the native text-input
    // eligibility gate is covered by the focused text-input routing tests.
    assert!(
        core.runtime
            .dispatch_focused_input(WidgetInput::TextEdit {
                command: TextEditCommand::SelectAll,
                timestamp,
            })
            .is_some()
    );
    assert_eq!(
        core.runtime.bridge().messages,
        vec![KeyboardTimestampMessage::TextEdit(timestamp)]
    );
}
