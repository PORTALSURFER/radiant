use crate::{
    gui::types::Rect,
    gui::{
        focus::FocusSurface,
        input::{InputTimestamp, KeyPress},
        shortcuts::ShortcutResolution,
    },
    layout::LayoutOutput,
    runtime::{
        Event, PaintPrimitive, RuntimeBridge, RuntimeHostCapabilities, RuntimeInputHost,
        SurfaceNode, SurfaceRuntime, UiSurface, WidgetMessageMapper,
    },
    theme::ThemeTokens,
    widgets::{
        KeyboardModifiers, TextEditCommand, Widget, WidgetCommon, WidgetInput, WidgetKey,
        WidgetOutput, WidgetSizing,
    },
};
use std::sync::Arc;

#[derive(Clone, Copy, Debug, PartialEq)]
enum KeyboardTimestampMessage {
    KeyPress(Option<InputTimestamp>),
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
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum FocusedKeyMessage {
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
struct FocusedKeyWidget {
    common: WidgetCommon,
    captured: Option<WidgetKey>,
    cancel_escape: bool,
    emit_repeat: bool,
}

impl FocusedKeyWidget {
    fn new(cancel_escape: bool, emit_repeat: bool) -> Self {
        Self {
            common: WidgetCommon::new(
                50,
                WidgetSizing::fixed(crate::gui::types::Vector2::new(120.0, 40.0)),
            )
            .with_keyboard_focus(),
            captured: None,
            cancel_escape,
            emit_repeat,
        }
    }
}

impl Widget for FocusedKeyWidget {
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
                if repeat && !self.emit_repeat {
                    return None;
                }
                Some(WidgetOutput::typed(FocusedKeyMessage::Press {
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
                Some(WidgetOutput::typed(FocusedKeyMessage::Release {
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

    fn focused_key_disposition(&self, key: WidgetKey) -> crate::widgets::FocusedKeyDisposition {
        if key == WidgetKey::PageDown {
            crate::widgets::FocusedKeyDisposition::Unhandled
        } else {
            crate::widgets::FocusedKeyDisposition::Consumed
        }
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

struct FocusedKeyBridge {
    messages: Vec<FocusedKeyMessage>,
    host_calls: usize,
    host_handled: bool,
    cancel_escape: bool,
    emit_repeat: bool,
}

impl FocusedKeyBridge {
    fn new(host_handled: bool, cancel_escape: bool) -> Self {
        Self {
            messages: Vec::new(),
            host_calls: 0,
            host_handled,
            cancel_escape,
            emit_repeat: true,
        }
    }

    fn without_repeat_output(mut self) -> Self {
        self.emit_repeat = false;
        self
    }
}

impl RuntimeBridge<FocusedKeyMessage> for FocusedKeyBridge {
    fn project_surface(&mut self) -> Arc<UiSurface<FocusedKeyMessage>> {
        crate::runtime::test_arc_surface(UiSurface::new(SurfaceNode::widget(
            FocusedKeyWidget::new(self.cancel_escape, self.emit_repeat),
            WidgetMessageMapper::typed(|message: FocusedKeyMessage| message),
        )))
    }

    fn reduce_message(&mut self, message: FocusedKeyMessage) {
        self.messages.push(message);
    }

    fn host_capabilities(&self) -> RuntimeHostCapabilities<Self, FocusedKeyMessage> {
        RuntimeHostCapabilities::new().with_input()
    }
}

impl RuntimeInputHost<FocusedKeyMessage> for FocusedKeyBridge {
    fn resolve_key_press(
        &mut self,
        _pending_chord: Option<KeyPress>,
        _press: KeyPress,
        _focus: FocusSurface,
    ) -> ShortcutResolution<FocusedKeyMessage> {
        self.host_calls += 1;
        if self.host_handled {
            ShortcutResolution::handled()
        } else {
            ShortcutResolution::unhandled()
        }
    }
}

#[derive(Clone)]
struct KeyboardTimestampWidget {
    common: WidgetCommon,
}

impl KeyboardTimestampWidget {
    fn new() -> Self {
        Self {
            common: WidgetCommon::new(
                40,
                WidgetSizing::fixed(crate::gui::types::Vector2::new(120.0, 40.0)),
            )
            .with_keyboard_focus(),
        }
    }
}

impl Widget for KeyboardTimestampWidget {
    fn common(&self) -> &WidgetCommon {
        &self.common
    }

    fn common_mut(&mut self) -> &mut WidgetCommon {
        &mut self.common
    }

    fn handle_input(&mut self, _bounds: Rect, input: WidgetInput) -> Option<WidgetOutput> {
        match input {
            WidgetInput::KeyPress { timestamp, .. } => Some(WidgetOutput::typed(
                KeyboardTimestampMessage::KeyPress(timestamp),
            )),
            WidgetInput::KeyRelease {
                key,
                modifiers,
                timestamp,
            } => Some(WidgetOutput::typed(KeyboardTimestampMessage::KeyRelease {
                key,
                modifiers,
                timestamp,
            })),
            WidgetInput::Character {
                character,
                timestamp,
            } => Some(WidgetOutput::typed(KeyboardTimestampMessage::Character {
                character,
                timestamp,
            })),
            WidgetInput::TextEdit { timestamp, .. } => Some(WidgetOutput::typed(
                KeyboardTimestampMessage::TextEdit(timestamp),
            )),
            _ => None,
        }
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

#[derive(Default)]
struct KeyboardTimestampBridge {
    messages: Vec<KeyboardTimestampMessage>,
}

impl RuntimeBridge<KeyboardTimestampMessage> for KeyboardTimestampBridge {
    fn project_surface(&mut self) -> Arc<UiSurface<KeyboardTimestampMessage>> {
        crate::runtime::test_arc_surface(UiSurface::new(SurfaceNode::widget(
            KeyboardTimestampWidget::new(),
            WidgetMessageMapper::typed(|message: KeyboardTimestampMessage| message),
        )))
    }

    fn reduce_message(&mut self, message: KeyboardTimestampMessage) {
        self.messages.push(message);
    }
}

#[test]
fn injected_keyboard_event_timestamp_survives_event_to_widget_dispatch() {
    let timestamp = Some(InputTimestamp::capture());
    let mut runtime = SurfaceRuntime::new(
        KeyboardTimestampBridge::default(),
        crate::gui::types::Vector2::new(120.0, 40.0),
    );

    assert!(runtime.focus_widget(40));
    assert_eq!(
        runtime.dispatch_event(Event::KeyPress {
            key: WidgetKey::Enter,
            modifiers: KeyboardModifiers::default(),
            repeat: false,
            timestamp,
        }),
        Some(40)
    );
    assert_eq!(
        runtime.dispatch_event(Event::Character {
            character: 'x',
            timestamp,
        }),
        Some(40)
    );
    let release_modifiers = KeyboardModifiers {
        command: true,
        control: true,
        shift: false,
        alt: true,
    };
    assert_eq!(
        runtime.dispatch_event(Event::KeyRelease {
            key: WidgetKey::ArrowDown,
            modifiers: release_modifiers,
            timestamp,
        }),
        Some(40)
    );
    assert_eq!(
        runtime.bridge().messages,
        vec![
            KeyboardTimestampMessage::KeyPress(timestamp),
            KeyboardTimestampMessage::Character {
                character: 'x',
                timestamp,
            },
            KeyboardTimestampMessage::KeyRelease {
                key: WidgetKey::ArrowDown,
                modifiers: release_modifiers,
                timestamp,
            },
        ]
    );
}

#[test]
fn key_release_event_without_focus_is_not_routed() {
    let mut runtime = SurfaceRuntime::new(
        KeyboardTimestampBridge::default(),
        crate::gui::types::Vector2::new(120.0, 40.0),
    );

    assert_eq!(
        runtime.dispatch_event(Event::key_release(WidgetKey::ArrowDown)),
        None
    );
    assert!(runtime.bridge().messages.is_empty());
}

#[test]
fn direct_text_edit_timestamp_survives_widget_dispatch() {
    let timestamp = Some(InputTimestamp::capture());
    let mut runtime = SurfaceRuntime::new(
        KeyboardTimestampBridge::default(),
        crate::gui::types::Vector2::new(120.0, 40.0),
    );

    assert!(runtime.focus_widget(40));
    assert_eq!(
        runtime.dispatch_focused_input(WidgetInput::TextEdit {
            command: TextEditCommand::SelectAll,
            timestamp,
        }),
        Some(40)
    );
    assert_eq!(
        runtime.bridge().messages,
        vec![KeyboardTimestampMessage::TextEdit(timestamp)]
    );
}

#[test]
fn focused_key_host_handled_initial_is_not_retried_or_captured() {
    let mut runtime = SurfaceRuntime::new(
        FocusedKeyBridge::new(true, false),
        crate::gui::types::Vector2::new(120.0, 40.0),
    );
    assert!(runtime.focus_widget(50));

    assert_eq!(
        runtime.dispatch_event(Event::key_press_with_metadata(
            WidgetKey::ArrowUp,
            KeyboardModifiers::default(),
            false,
            None,
        )),
        None
    );
    assert_eq!(runtime.bridge().host_calls, 1);
    assert!(runtime.bridge().messages.is_empty());

    assert_eq!(
        runtime.dispatch_event(Event::key_press_with_metadata(
            WidgetKey::ArrowUp,
            KeyboardModifiers::default(),
            true,
            None,
        )),
        None
    );
    assert_eq!(runtime.bridge().host_calls, 1);
    assert!(runtime.bridge().messages.is_empty());
}

#[test]
fn focused_key_capture_preserves_continuation_metadata_and_bypasses_host() {
    let initial_timestamp = Some(InputTimestamp::capture());
    let repeat_timestamp = Some(InputTimestamp::capture());
    let release_timestamp = Some(InputTimestamp::capture());
    let repeat_modifiers = KeyboardModifiers {
        command: true,
        control: false,
        shift: true,
        alt: true,
    };
    let release_modifiers = KeyboardModifiers {
        command: false,
        control: true,
        shift: true,
        alt: false,
    };
    let mut runtime = SurfaceRuntime::new(
        FocusedKeyBridge::new(false, true),
        crate::gui::types::Vector2::new(120.0, 40.0),
    );
    assert!(runtime.focus_widget(50));

    assert_eq!(
        runtime.dispatch_event(Event::key_press_with_metadata(
            WidgetKey::ArrowUp,
            KeyboardModifiers::default(),
            false,
            initial_timestamp,
        )),
        Some(50)
    );
    assert_eq!(runtime.bridge().host_calls, 1);
    assert_eq!(
        runtime.bridge().messages,
        vec![FocusedKeyMessage::Press {
            key: WidgetKey::ArrowUp,
            modifiers: KeyboardModifiers::default(),
            repeat: false,
            timestamp: initial_timestamp,
        }]
    );

    assert_eq!(
        runtime.dispatch_event(Event::key_press_with_metadata(
            WidgetKey::ArrowUp,
            repeat_modifiers,
            true,
            repeat_timestamp,
        )),
        Some(50)
    );
    assert_eq!(runtime.bridge().host_calls, 1);
    assert_eq!(
        runtime.bridge().messages.last().copied(),
        Some(FocusedKeyMessage::Press {
            key: WidgetKey::ArrowUp,
            modifiers: repeat_modifiers,
            repeat: true,
            timestamp: repeat_timestamp,
        })
    );

    assert_eq!(
        runtime.dispatch_event(Event::key_release_with_metadata(
            WidgetKey::ArrowUp,
            release_modifiers,
            release_timestamp,
        )),
        Some(50)
    );
    assert_eq!(runtime.bridge().host_calls, 1);
    assert_eq!(
        runtime.bridge().messages.last().copied(),
        Some(FocusedKeyMessage::Release {
            key: WidgetKey::ArrowUp,
            modifiers: release_modifiers,
            timestamp: release_timestamp,
        })
    );
}

#[test]
fn focused_key_owner_without_output_still_bypasses_host() {
    let mut runtime = SurfaceRuntime::new(
        FocusedKeyBridge::new(false, false).without_repeat_output(),
        crate::gui::types::Vector2::new(120.0, 40.0),
    );
    assert!(runtime.focus_widget(50));
    assert_eq!(
        runtime.dispatch_event(Event::key_press_with_metadata(
            WidgetKey::ArrowUp,
            KeyboardModifiers::default(),
            false,
            None,
        )),
        Some(50)
    );
    assert_eq!(
        runtime.dispatch_event(Event::key_press_with_metadata(
            WidgetKey::ArrowUp,
            KeyboardModifiers::default(),
            true,
            None,
        )),
        Some(50)
    );
    assert_eq!(runtime.bridge().host_calls, 1);
    assert_eq!(runtime.bridge().messages.len(), 1);
}

#[test]
fn unhandled_metadata_page_repeat_delivers_without_acquiring_capture() {
    let mut runtime = SurfaceRuntime::new(
        FocusedKeyBridge::new(false, false),
        crate::gui::types::Vector2::new(120.0, 40.0),
    );
    assert!(runtime.focus_widget(50));

    for repeat in [false, true] {
        assert_eq!(
            runtime.dispatch_event(Event::key_press_with_metadata(
                WidgetKey::PageDown,
                KeyboardModifiers::default(),
                repeat,
                None,
            )),
            Some(50)
        );
    }
    assert_eq!(runtime.bridge().host_calls, 1);
    assert_eq!(runtime.bridge().messages.len(), 2);
    assert_eq!(runtime.interaction.focus.focused_key_capture, None);
}

#[test]
fn focused_key_competing_orphan_and_stale_samples_are_ignored_without_repaint() {
    let timestamp = Some(InputTimestamp::capture());
    let mut runtime = SurfaceRuntime::new(
        FocusedKeyBridge::new(false, true),
        crate::gui::types::Vector2::new(120.0, 40.0),
    );
    assert!(runtime.focus_widget(50));
    let _ = runtime.take_repaint_requested();

    assert_eq!(
        runtime.dispatch_event(Event::key_press_with_metadata(
            WidgetKey::ArrowUp,
            KeyboardModifiers::default(),
            false,
            timestamp,
        )),
        Some(50)
    );
    let messages_after_initial = runtime.bridge().messages.len();
    assert_eq!(runtime.bridge().host_calls, 1);
    let _ = runtime.take_repaint_requested();

    for event in [
        Event::key_press_with_metadata(
            WidgetKey::ArrowDown,
            KeyboardModifiers::default(),
            true,
            timestamp,
        ),
        Event::key_release_with_metadata(
            WidgetKey::ArrowDown,
            KeyboardModifiers::default(),
            timestamp,
        ),
    ] {
        assert_eq!(runtime.dispatch_event(event), None);
    }
    assert_eq!(runtime.bridge().messages.len(), messages_after_initial);
    assert_eq!(runtime.bridge().host_calls, 1);
    assert!(!runtime.repaint_requested());

    let cancellation_modifiers = KeyboardModifiers {
        command: false,
        control: false,
        shift: true,
        alt: true,
    };
    assert_eq!(
        runtime.dispatch_event(Event::key_press_with_metadata(
            WidgetKey::Escape,
            cancellation_modifiers,
            false,
            timestamp,
        )),
        Some(50)
    );
    assert_eq!(runtime.bridge().host_calls, 1);
    assert_eq!(
        runtime.bridge().messages.last().copied(),
        Some(FocusedKeyMessage::Press {
            key: WidgetKey::Escape,
            modifiers: cancellation_modifiers,
            repeat: false,
            timestamp,
        })
    );

    let messages_after_cancel = runtime.bridge().messages.len();
    assert_eq!(
        runtime.dispatch_event(Event::key_press_with_metadata(
            WidgetKey::ArrowUp,
            KeyboardModifiers::default(),
            true,
            timestamp,
        )),
        None
    );
    assert_eq!(
        runtime.dispatch_event(Event::key_release_with_metadata(
            WidgetKey::ArrowUp,
            KeyboardModifiers::default(),
            timestamp,
        )),
        None
    );
    assert_eq!(runtime.bridge().messages.len(), messages_after_cancel);
    assert_eq!(runtime.bridge().host_calls, 1);

    assert_eq!(
        runtime.dispatch_event(Event::key_press_with_metadata(
            WidgetKey::ArrowUp,
            KeyboardModifiers::default(),
            false,
            timestamp,
        )),
        Some(50)
    );
    runtime.clear_focus();
    let _ = runtime.take_repaint_requested();
    let messages_before_stale = runtime.bridge().messages.len();
    assert_eq!(
        runtime.dispatch_event(Event::key_press_with_metadata(
            WidgetKey::ArrowUp,
            KeyboardModifiers::default(),
            true,
            timestamp,
        )),
        None
    );
    assert_eq!(
        runtime.dispatch_event(Event::key_release_with_metadata(
            WidgetKey::ArrowUp,
            KeyboardModifiers::default(),
            timestamp,
        )),
        None
    );
    assert_eq!(runtime.bridge().messages.len(), messages_before_stale);
    assert_eq!(runtime.bridge().host_calls, 2);
    assert!(!runtime.repaint_requested());
}

#[test]
#[allow(clippy::arc_with_non_send_sync)]
fn container_keyboard_edits_preserve_native_timestamp() {
    use crate::gui::types::Vector2;
    use crate::runtime::{ScrollEditBatch, declarative_runtime_bridge};
    use crate::widgets::EditPhase;
    use crate::widgets::{InteractionProvenance, KeyboardModifiers, WidgetKey};
    use std::{cell::RefCell, rc::Rc};
    fn phases(batch: &ScrollEditBatch) -> Vec<EditPhase> {
        batch.events().iter().map(|event| event.phase).collect()
    }
    let edits = Rc::new(RefCell::new(Vec::<ScrollEditBatch>::new()));
    let sink = Rc::clone(&edits);
    let bridge = declarative_runtime_bridge(
        (),
        |_| {
            Arc::new(UiSurface::new(
                SurfaceNode::scroll_area(
                    31,
                    SurfaceNode::button(
                        32,
                        "content",
                        WidgetSizing::fixed(Vector2::new(180.0, 400.0)),
                        None,
                    ),
                )
                .on_scroll_edit(Some),
            ))
        },
        move |_, batch: Option<ScrollEditBatch>| {
            if let Some(batch) = batch {
                sink.borrow_mut().push(batch);
            }
        },
    );
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(220.0, 96.0));
    assert!(runtime.focus_widget(32));
    runtime.execute_command(crate::runtime::Command::scroll_to(
        31,
        Vector2::new(0.0, 0.0),
    ));
    edits.borrow_mut().clear();
    let timestamp = Some(InputTimestamp::capture());
    for repeat in [false, true] {
        runtime.dispatch_event(Event::KeyPress {
            key: WidgetKey::PageDown,
            modifiers: KeyboardModifiers::default(),
            repeat,
            timestamp,
        });
    }
    let edits = edits.borrow();
    assert_eq!(edits.len(), 2);
    assert_ne!(edits[0].transaction(), edits[1].transaction());
    for batch in edits.iter() {
        assert_eq!(
            phases(batch),
            [EditPhase::Begin, EditPhase::Update, EditPhase::Commit]
        );
        assert!(
            batch
                .events()
                .iter()
                .all(|event| event.provenance == InteractionProvenance::Keyboard { timestamp })
        );
        assert_eq!(batch.offset_update().unwrap().metadata.timestamp, timestamp);
    }
}
