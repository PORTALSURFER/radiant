use super::*;
use radiant::{
    application::numeric_input,
    widgets::{
        EditPhase, InteractionProvenance, KeyboardModifier, KeyboardModifiers, NumericAdjustment,
        NumericCodec, NumericInputInteraction, NumericInputInteractionBatch, NumericParseResult,
        NumericScrubPolicy, NumericStep, NumericStepDirection, NumericStepModifiers,
        PointerModifiers, WheelDelta, WheelPhase, WheelSample,
    },
};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

#[test]
fn surface_runtime_resolves_host_shortcuts_before_widget_key_routing() {
    let mut runtime = SurfaceRuntime::new(ShortcutDemoBridge::default(), Vector2::new(420.0, 32.0));

    assert!(runtime.dispatch_key_press(
        KeyPress::with_command(KeyCode::I),
        None,
        FocusSurface::None
    ));
    assert_eq!(runtime.bridge().state.count, 1);
}

#[test]
fn surface_runtime_routes_backend_neutral_events() {
    let bridge = declarative_runtime_bridge(
        DemoState::default(),
        project_surface,
        |state: &mut DemoState, message| match message {
            DemoMessage::Increment => state.count += 1,
            DemoMessage::Rename(name) => state.name = name,
            DemoMessage::CanvasInput(_) => {}
        },
    );
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(420.0, 32.0));

    assert_eq!(
        runtime.dispatch_event(Event::resize(Vector2::new(360.0, 40.0))),
        None
    );
    assert_eq!(runtime.viewport(), Vector2::new(360.0, 40.0));

    assert_eq!(
        runtime.dispatch_event(Event::primary_press(Point::new(150.0, 10.0))),
        Some(11)
    );
    assert_eq!(runtime.focused_widget(), Some(11));
    assert_eq!(runtime.pointer_capture(), Some(11));
    assert_eq!(
        runtime.dispatch_event(Event::primary_release(Point::new(150.0, 10.0))),
        Some(11)
    );
    assert_eq!(runtime.pointer_capture(), None);
    assert_eq!(
        runtime.dispatch_event(Event::traverse_focus(FocusTraversal::Forward)),
        Some(12)
    );
    assert_eq!(runtime.dispatch_event(Event::character('R')), Some(12));

    assert_eq!(
        widget_ref::<TextWidget, _>(runtime.surface(), 10, "text").text,
        "R (1)"
    );
    assert_eq!(
        widget_ref::<TextInputWidget, _>(runtime.surface(), 12, "text input")
            .state
            .value,
        "R"
    );
}

#[test]
fn surface_runtime_skips_duplicate_viewport_resize_work() {
    let bridge = declarative_runtime_bridge(
        DemoState::default(),
        project_surface,
        |state: &mut DemoState, message| match message {
            DemoMessage::Increment => state.count += 1,
            DemoMessage::Rename(name) => state.name = name,
            DemoMessage::CanvasInput(_) => {}
        },
    );
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(420.0, 32.0));
    let initial_stats = runtime.layout().stats;

    runtime.dispatch_event(Event::resize(Vector2::new(420.0, 32.0)));

    assert_eq!(runtime.viewport(), Vector2::new(420.0, 32.0));
    assert_eq!(
        runtime.layout().stats,
        initial_stats,
        "duplicate logical resize should not replace the current layout evaluation"
    );
}

#[test]
fn backend_neutral_event_constructors_preserve_payloads() {
    let point = Point::new(20.0, 10.0);
    let delta = Vector2::new(0.0, -32.0);
    let modifiers = PointerModifiers {
        command: true,
        shift: true,
        alt: false,
    };

    assert_eq!(
        Event::pointer_press(point, PointerButton::Auxiliary, modifiers),
        Event::PointerPress {
            position: point,
            button: PointerButton::Auxiliary,
            modifiers,
            timestamp: None,
        }
    );
    assert_eq!(
        Event::secondary_press(point),
        Event::PointerPress {
            position: point,
            button: PointerButton::Secondary,
            modifiers: PointerModifiers::default(),
            timestamp: None,
        }
    );
    assert_eq!(
        Event::primary_release(point),
        Event::PointerRelease {
            position: point,
            button: PointerButton::Primary,
            modifiers: PointerModifiers::default(),
            timestamp: None,
        }
    );
    assert_eq!(
        Event::primary_double_click(point),
        Event::PointerDoubleClick {
            position: point,
            button: PointerButton::Primary,
            modifiers: PointerModifiers::default(),
            timestamp: None,
        }
    );
    assert_eq!(
        Event::pointer_modifiers_changed(modifiers),
        Event::PointerModifiersChanged {
            modifiers,
            timestamp: None,
        }
    );
    assert_eq!(
        Event::scroll(point, delta),
        Event::Scroll {
            position: point,
            delta,
            modifiers: PointerModifiers::default(),
            timestamp: None,
            sequence_range: None,
        }
    );
}

#[test]
fn normalized_key_release_constructors_are_public() {
    assert_eq!(
        Event::key_release(WidgetKey::ArrowDown),
        Event::KeyRelease {
            key: WidgetKey::ArrowDown,
            modifiers: Default::default(),
            timestamp: None,
        }
    );
    assert_eq!(
        WidgetInput::key_release(WidgetKey::ArrowDown),
        WidgetInput::KeyRelease {
            key: WidgetKey::ArrowDown,
            modifiers: Default::default(),
            timestamp: None,
        }
    );
}

#[test]
fn surface_runtime_routes_pointer_click_convenience_through_press_and_release_events() {
    let bridge = declarative_runtime_bridge(
        DemoState::default(),
        project_surface,
        |state: &mut DemoState, message| match message {
            DemoMessage::Increment => state.count += 1,
            DemoMessage::Rename(name) => state.name = name,
            DemoMessage::CanvasInput(_) => {}
        },
    );
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(420.0, 32.0));

    let outcome = runtime.dispatch_primary_click(Point::new(150.0, 10.0));

    assert_eq!(outcome.press_target, Some(11));
    assert_eq!(outcome.release_target, Some(11));
    assert_eq!(outcome.completed_widget(), Some(11));
    assert_eq!(runtime.pointer_capture(), None);
    assert_eq!(runtime.bridge().state().count, 1);
    assert_eq!(
        widget_ref::<TextWidget, _>(runtime.surface(), 10, "text").text,
        "Untitled (1)"
    );
}

#[test]
fn surface_runtime_routes_secondary_click_convenience() {
    let bridge = declarative_runtime_bridge(
        DemoState::default(),
        project_surface,
        |state: &mut DemoState, message| match message {
            DemoMessage::Increment => state.count += 1,
            DemoMessage::Rename(name) => state.name = name,
            DemoMessage::CanvasInput(_) => {}
        },
    );
    let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(420.0, 32.0));

    let outcome = runtime.dispatch_secondary_click(Point::new(150.0, 10.0));

    assert_eq!(outcome.press_target, Some(11));
    assert_eq!(outcome.release_target, Some(11));
    assert_eq!(outcome.completed_widget(), Some(11));
    assert_eq!(runtime.pointer_capture(), None);
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PublicFocusedKeyMessage {
    Press { key: WidgetKey, repeat: bool },
    Release { key: WidgetKey },
}

#[derive(Clone)]
struct PublicFocusedKeyWidget {
    common: WidgetCommon,
    captured: Option<WidgetKey>,
}

impl PublicFocusedKeyWidget {
    fn new() -> Self {
        Self {
            common: WidgetCommon::new(140, WidgetSizing::fixed(Vector2::new(120.0, 32.0)))
                .with_keyboard_focus(),
            captured: None,
        }
    }
}

impl Widget for PublicFocusedKeyWidget {
    fn common(&self) -> &WidgetCommon {
        &self.common
    }

    fn common_mut(&mut self) -> &mut WidgetCommon {
        &mut self.common
    }

    fn handle_input(&mut self, _bounds: Rect, input: WidgetInput) -> Option<WidgetOutput> {
        match input {
            WidgetInput::KeyPress { key, repeat, .. } => {
                if !repeat && key == WidgetKey::ArrowUp {
                    self.captured = Some(key);
                }
                Some(WidgetOutput::typed(PublicFocusedKeyMessage::Press {
                    key,
                    repeat,
                }))
            }
            WidgetInput::KeyRelease { key, .. } => {
                if self.captured == Some(key) {
                    self.captured = None;
                }
                Some(WidgetOutput::typed(PublicFocusedKeyMessage::Release {
                    key,
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

    fn append_paint(
        &self,
        _primitives: &mut Vec<PaintPrimitive>,
        _bounds: Rect,
        _layout: &radiant::layout::LayoutOutput,
        _theme: &ThemeTokens,
    ) {
    }
}

#[derive(Default)]
struct PublicFocusedKeyBridge {
    messages: Vec<PublicFocusedKeyMessage>,
}

impl RuntimeBridge<PublicFocusedKeyMessage> for PublicFocusedKeyBridge {
    fn project_surface(&mut self) -> Arc<UiSurface<PublicFocusedKeyMessage>> {
        arc_surface(UiSurface::new(SurfaceNode::custom_widget(
            PublicFocusedKeyWidget::new(),
            WidgetMessageMapper::typed(|message: PublicFocusedKeyMessage| message),
        )))
    }

    fn reduce_message(&mut self, message: PublicFocusedKeyMessage) {
        self.messages.push(message);
    }
}

#[test]
fn public_widget_focused_key_opt_in_is_object_safe_and_captures_continuations() {
    let widget: Box<dyn Widget> = Box::new(PublicFocusedKeyWidget::new());
    assert!(widget.participates_in_focused_key_routing());
    assert_eq!(widget.captured_focused_key(), None);
    assert_eq!(WidgetKey::ArrowUp.to_key_code(), KeyCode::ArrowUp);

    let mut runtime =
        SurfaceRuntime::new(PublicFocusedKeyBridge::default(), Vector2::new(120.0, 32.0));
    assert!(runtime.focus_widget(140));
    assert_eq!(
        runtime.dispatch_event(Event::KeyPress {
            key: WidgetKey::ArrowUp,
            modifiers: Default::default(),
            repeat: false,
            timestamp: None,
        }),
        Some(140)
    );
    assert_eq!(
        runtime.dispatch_event(Event::KeyPress {
            key: WidgetKey::ArrowUp,
            modifiers: Default::default(),
            repeat: true,
            timestamp: None,
        }),
        Some(140)
    );
    assert_eq!(
        runtime.bridge().messages,
        vec![
            PublicFocusedKeyMessage::Press {
                key: WidgetKey::ArrowUp,
                repeat: false,
            },
            PublicFocusedKeyMessage::Press {
                key: WidgetKey::ArrowUp,
                repeat: true,
            },
        ]
    );
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct RuntimeNumericValue(u32);

#[derive(Debug, PartialEq, Eq)]
struct RuntimeNumericStepError;

#[derive(Debug, PartialEq, Eq)]
struct RuntimeNumericFormatError;

struct RuntimeNumericCodec {
    format_calls: Rc<Cell<usize>>,
    parse_calls: Rc<Cell<usize>>,
}

impl NumericCodec<RuntimeNumericValue> for RuntimeNumericCodec {
    type Error = RuntimeNumericFormatError;

    fn parse(&self, text: &str) -> NumericParseResult<RuntimeNumericValue> {
        self.parse_calls.set(self.parse_calls.get() + 1);
        text.parse::<u32>()
            .map(RuntimeNumericValue)
            .map_or(NumericParseResult::Invalid, NumericParseResult::Valid)
    }

    fn format_editable(
        &self,
        value: &RuntimeNumericValue,
        output: &mut dyn std::fmt::Write,
    ) -> Result<(), Self::Error> {
        self.format_calls.set(self.format_calls.get() + 1);
        write!(output, "{}", value.0).map_err(|_| RuntimeNumericFormatError)
    }
}

struct RuntimeNumericAdjustment {
    inverse_calls: Rc<Cell<usize>>,
    step_calls: Rc<Cell<usize>>,
}

impl NumericAdjustment<RuntimeNumericValue> for RuntimeNumericAdjustment {
    type Error = RuntimeNumericStepError;

    fn normalized_to_value(&self, normalized: f32) -> Result<RuntimeNumericValue, Self::Error> {
        Ok(RuntimeNumericValue(normalized as u32))
    }

    fn value_to_normalized(&self, value: &RuntimeNumericValue) -> Result<f32, Self::Error> {
        self.inverse_calls.set(self.inverse_calls.get() + 1);
        Ok(value.0 as f32)
    }

    fn step(
        &self,
        value: &RuntimeNumericValue,
        direction: NumericStepDirection,
        step: NumericStep,
    ) -> Result<RuntimeNumericValue, Self::Error> {
        self.step_calls.set(self.step_calls.get() + 1);
        let amount = match step {
            NumericStep::Base => 1,
            NumericStep::Fine => 2,
            NumericStep::Coarse => 10,
        };
        let value = match direction {
            NumericStepDirection::Decrease => value.0.saturating_sub(amount),
            NumericStepDirection::Increase => value.0.saturating_add(amount),
        };
        Ok(RuntimeNumericValue(value))
    }

    fn scrub(
        &self,
        value: &RuntimeNumericValue,
        normalized_delta: f32,
        _step: NumericStep,
    ) -> Result<RuntimeNumericValue, Self::Error> {
        Ok(RuntimeNumericValue(value.0.saturating_add(
            if normalized_delta > 0.0 { 1 } else { 0 },
        )))
    }

    fn wheel(
        &self,
        value: &RuntimeNumericValue,
        _delta: f32,
        _step: NumericStep,
    ) -> Result<RuntimeNumericValue, Self::Error> {
        Ok(value.clone())
    }
}

type RuntimeNumericBatch = NumericInputInteractionBatch<
    RuntimeNumericValue,
    RuntimeNumericStepError,
    RuntimeNumericFormatError,
>;

enum RuntimeNumericMessage {
    Interaction(RuntimeNumericBatch),
}

struct RuntimeNumericBridge {
    value: RuntimeNumericValue,
    host_calls: usize,
    host_handled: bool,
    mapped_phases: Vec<Vec<EditPhase>>,
    format_calls: Rc<Cell<usize>>,
    parse_calls: Rc<Cell<usize>>,
    inverse_calls: Rc<Cell<usize>>,
    step_calls: Rc<Cell<usize>>,
    mapped_provenance: Vec<Vec<InteractionProvenance>>,
}

impl Default for RuntimeNumericBridge {
    fn default() -> Self {
        Self {
            value: RuntimeNumericValue(7),
            host_calls: 0,
            host_handled: true,
            mapped_phases: Vec::new(),
            format_calls: Rc::new(Cell::new(0)),
            parse_calls: Rc::new(Cell::new(0)),
            inverse_calls: Rc::new(Cell::new(0)),
            step_calls: Rc::new(Cell::new(0)),
            mapped_provenance: Vec::new(),
        }
    }
}

impl RuntimeNumericBridge {
    fn numeric_policy_calls(&self) -> (usize, usize, usize, usize) {
        (
            self.format_calls.get(),
            self.parse_calls.get(),
            self.inverse_calls.get(),
            self.step_calls.get(),
        )
    }
}

impl RuntimeBridge<RuntimeNumericMessage> for RuntimeNumericBridge {
    fn project_surface(&mut self) -> Arc<UiSurface<RuntimeNumericMessage>> {
        arc_surface(
            numeric_input(
                self.value.clone(),
                RuntimeNumericCodec {
                    format_calls: Rc::clone(&self.format_calls),
                    parse_calls: Rc::clone(&self.parse_calls),
                },
                RuntimeNumericAdjustment {
                    inverse_calls: Rc::clone(&self.inverse_calls),
                    step_calls: Rc::clone(&self.step_calls),
                },
            )
            .expect("runtime numeric fixture should construct")
            .step_modifiers(NumericStepModifiers::new(
                KeyboardModifier::Shift,
                KeyboardModifier::Control,
            ))
            .scrub_policy(NumericScrubPolicy::default())
            .on_interaction(RuntimeNumericMessage::Interaction)
            .id(150)
            .into_surface(),
        )
    }

    fn reduce_message(&mut self, message: RuntimeNumericMessage) {
        let RuntimeNumericMessage::Interaction(batch) = message;
        let mut phases = Vec::new();
        let mut provenance = Vec::new();
        for interaction in batch.parts() {
            if let NumericInputInteraction::Edit(edit) = interaction {
                phases.extend(edit.events().iter().map(|event| event.phase));
                provenance.extend(edit.events().iter().map(|event| event.provenance));
                if let Some(event) = edit.events().last() {
                    self.value = event.value.clone();
                }
            }
        }
        self.mapped_phases.push(phases);
        self.mapped_provenance.push(provenance);
    }

    fn host_capabilities(&self) -> RuntimeHostCapabilities<Self, RuntimeNumericMessage> {
        RuntimeHostCapabilities::new().with_input()
    }
}

impl RuntimeInputHost<RuntimeNumericMessage> for RuntimeNumericBridge {
    fn resolve_key_press(
        &mut self,
        _pending_chord: Option<KeyPress>,
        _press: KeyPress,
        _focus: FocusSurface,
    ) -> ShortcutResolution<RuntimeNumericMessage> {
        self.host_calls += 1;
        if self.host_handled {
            ShortcutResolution::handled()
        } else {
            ShortcutResolution::unhandled()
        }
    }
}

#[test]
fn public_numeric_keyboard_owner_conflict_still_resolves_host_first() {
    let mut runtime =
        SurfaceRuntime::new(RuntimeNumericBridge::default(), Vector2::new(120.0, 32.0));
    assert!(runtime.focus_widget(150));

    assert_eq!(
        runtime.dispatch_event(Event::Character {
            character: '8',
            timestamp: None,
        }),
        Some(150),
        "normal character dispatch should establish the active TextEdit owner"
    );
    assert!(runtime.bridge().mapped_phases.is_empty());

    let before_value = runtime.bridge().value.clone();
    let before_policy_calls = runtime.bridge().numeric_policy_calls();
    let before_mapped_phases = runtime.bridge().mapped_phases.clone();

    assert_eq!(
        runtime.dispatch_event(Event::KeyPress {
            key: WidgetKey::ArrowUp,
            modifiers: KeyboardModifiers::default(),
            repeat: false,
            timestamp: None,
        }),
        None,
        "host-handled initial must not reach the active TextEdit widget"
    );
    assert_eq!(runtime.bridge().host_calls, 1);
    assert_eq!(runtime.bridge().value, before_value);
    assert_eq!(runtime.bridge().numeric_policy_calls(), before_policy_calls);
    assert_eq!(runtime.bridge().mapped_phases, before_mapped_phases);

    runtime.bridge_mut().host_handled = false;
    assert_eq!(
        runtime.dispatch_event(Event::KeyPress {
            key: WidgetKey::ArrowDown,
            modifiers: KeyboardModifiers::default(),
            repeat: false,
            timestamp: None,
        }),
        Some(150),
        "host-unhandled initial should fall back to widget admission"
    );
    assert_eq!(runtime.bridge().host_calls, 2);
    assert_eq!(runtime.bridge().value, before_value);
    assert_eq!(runtime.bridge().numeric_policy_calls(), before_policy_calls);
    assert_eq!(runtime.bridge().mapped_phases, before_mapped_phases);
}

#[test]
fn public_numeric_keyboard_routes_host_first_then_captured_continuations() {
    let mut runtime =
        SurfaceRuntime::new(RuntimeNumericBridge::default(), Vector2::new(120.0, 32.0));
    assert!(runtime.focus_widget(150));

    assert_eq!(
        runtime.dispatch_event(Event::KeyPress {
            key: WidgetKey::ArrowUp,
            modifiers: KeyboardModifiers::default(),
            repeat: false,
            timestamp: None,
        }),
        None,
        "host-handled initial must not reach the widget"
    );
    assert_eq!(runtime.bridge().host_calls, 1);
    assert!(runtime.bridge().mapped_phases.is_empty());

    runtime.bridge_mut().host_handled = false;
    assert_eq!(
        runtime.dispatch_event(Event::KeyPress {
            key: WidgetKey::ArrowUp,
            modifiers: KeyboardModifiers::default(),
            repeat: false,
            timestamp: None,
        }),
        Some(150)
    );
    assert_eq!(runtime.bridge().host_calls, 2);
    assert_eq!(
        runtime.bridge().mapped_phases,
        vec![vec![EditPhase::Begin, EditPhase::Update]]
    );

    runtime.refresh();

    assert_eq!(
        runtime.dispatch_event(Event::KeyPress {
            key: WidgetKey::ArrowUp,
            modifiers: KeyboardModifiers {
                shift: true,
                ..KeyboardModifiers::default()
            },
            repeat: true,
            timestamp: None,
        }),
        Some(150)
    );
    assert_eq!(runtime.bridge().host_calls, 2);
    assert_eq!(
        runtime.bridge().mapped_phases,
        vec![
            vec![EditPhase::Begin, EditPhase::Update],
            vec![EditPhase::Update],
        ]
    );

    assert_eq!(
        runtime.dispatch_event(Event::KeyRelease {
            key: WidgetKey::ArrowUp,
            modifiers: KeyboardModifiers::default(),
            timestamp: None,
        }),
        Some(150)
    );
    assert_eq!(runtime.bridge().host_calls, 2);
    assert_eq!(
        runtime.bridge().mapped_phases,
        vec![
            vec![EditPhase::Begin, EditPhase::Update],
            vec![EditPhase::Update],
            vec![EditPhase::Commit],
        ]
    );
}

#[test]
fn public_numeric_pointer_scrub_reaches_generic_managed_capture_with_synthetic_provenance() {
    let mut runtime =
        SurfaceRuntime::new(RuntimeNumericBridge::default(), Vector2::new(120.0, 32.0));
    assert!(runtime.focus_widget(150));
    let modifiers = PointerModifiers {
        alt: true,
        ..PointerModifiers::default()
    };
    let press_timestamp = None;
    assert_eq!(
        runtime.dispatch_event(Event::PointerPress {
            position: Point::new(10.0, 16.0),
            button: PointerButton::Primary,
            modifiers,
            timestamp: press_timestamp,
        }),
        Some(150)
    );
    assert_eq!(runtime.pointer_capture(), Some(150));
    assert!(runtime.bridge().mapped_phases.is_empty());

    let move_timestamp = None;
    let sequence_range = None;
    assert_eq!(
        runtime.dispatch_event(Event::PointerMove {
            position: Point::new(110.0, 16.0),
            modifiers,
            timestamp: move_timestamp,
            sequence_range,
        }),
        Some(150)
    );
    assert_eq!(runtime.bridge().value, RuntimeNumericValue(8));
    assert_eq!(
        runtime.bridge().mapped_phases,
        vec![vec![EditPhase::Begin, EditPhase::Update]]
    );
    assert_eq!(
        runtime.bridge().mapped_provenance,
        vec![vec![
            InteractionProvenance::Pointer {
                modifiers,
                timestamp: press_timestamp,
                sequence_range: None,
            },
            InteractionProvenance::Pointer {
                modifiers,
                timestamp: move_timestamp,
                sequence_range,
            },
        ]]
    );

    let release_timestamp = None;
    assert_eq!(
        runtime.dispatch_event(Event::PointerRelease {
            position: Point::new(110.0, 16.0),
            button: PointerButton::Primary,
            modifiers,
            timestamp: release_timestamp,
        }),
        Some(150)
    );
    assert_eq!(runtime.pointer_capture(), None);
    assert_eq!(
        runtime.bridge().mapped_phases,
        vec![
            vec![EditPhase::Begin, EditPhase::Update],
            vec![EditPhase::Commit]
        ]
    );
    assert_eq!(
        runtime.bridge().mapped_provenance[1],
        vec![InteractionProvenance::Pointer {
            modifiers,
            timestamp: release_timestamp,
            sequence_range: None,
        }]
    );
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct RuntimeWheelObservation {
    widget_id: u64,
    delta: WheelDelta,
    phase: Option<WheelPhase>,
    replacement: bool,
}

#[derive(Clone)]
struct RuntimeWheelWidget {
    common: WidgetCommon,
    observations: Rc<RefCell<Vec<RuntimeWheelObservation>>>,
    retained: bool,
    retention_enabled: bool,
    replacement: bool,
}

impl RuntimeWheelWidget {
    fn new(
        widget_id: u64,
        observations: Rc<RefCell<Vec<RuntimeWheelObservation>>>,
        focusable: bool,
        disabled: bool,
        read_only: bool,
        retention_enabled: bool,
        replacement: bool,
    ) -> Self {
        let common = WidgetCommon::new(widget_id, WidgetSizing::fixed(Vector2::new(140.0, 40.0)));
        let mut common = if focusable {
            common.with_keyboard_focus()
        } else {
            common
        };
        common.state.disabled = disabled;
        common.state.read_only = read_only;
        Self {
            common,
            observations,
            retained: false,
            retention_enabled,
            replacement,
        }
    }

    fn record_sample(&mut self, sample: WheelSample) {
        self.observations
            .borrow_mut()
            .push(RuntimeWheelObservation {
                widget_id: self.common.id,
                delta: sample.delta(),
                phase: sample.phase(),
                replacement: self.replacement,
            });
        match sample.phase() {
            Some(WheelPhase::Started) => self.retained = self.retention_enabled,
            Some(WheelPhase::Ended | WheelPhase::Cancelled) => self.retained = false,
            Some(WheelPhase::Changed) | Some(WheelPhase::Discrete) | None => {}
        }
    }
}

impl Widget for RuntimeWheelWidget {
    fn common(&self) -> &WidgetCommon {
        &self.common
    }

    fn common_mut(&mut self) -> &mut WidgetCommon {
        &mut self.common
    }

    fn handle_input(&mut self, _bounds: Rect, _input: WidgetInput) -> Option<WidgetOutput> {
        None
    }

    fn handle_wheel_sample(
        &mut self,
        _bounds: Rect,
        _position: Point,
        sample: WheelSample,
    ) -> Option<WidgetOutput> {
        self.record_sample(sample);
        None
    }

    fn retains_managed_wheel_sequence(&self) -> bool {
        self.retained && self.retention_enabled
    }

    fn synchronize_from_previous(&mut self, previous: &dyn Widget) {
        if let Some(previous) = previous.as_any().downcast_ref::<Self>() {
            self.retained = previous.retained;
        }
    }

    fn append_paint(
        &self,
        _primitives: &mut Vec<PaintPrimitive>,
        _bounds: Rect,
        _layout: &radiant::layout::LayoutOutput,
        _theme: &ThemeTokens,
    ) {
    }
}

#[derive(Clone)]
struct RuntimeReplacementWheelWidget {
    inner: RuntimeWheelWidget,
}

impl RuntimeReplacementWheelWidget {
    fn new(
        observations: Rc<RefCell<Vec<RuntimeWheelObservation>>>,
        disabled: bool,
        read_only: bool,
        retention_enabled: bool,
    ) -> Self {
        Self {
            inner: RuntimeWheelWidget::new(
                401,
                observations,
                true,
                disabled,
                read_only,
                retention_enabled,
                true,
            ),
        }
    }
}

impl Widget for RuntimeReplacementWheelWidget {
    fn common(&self) -> &WidgetCommon {
        self.inner.common()
    }

    fn common_mut(&mut self) -> &mut WidgetCommon {
        self.inner.common_mut()
    }

    fn handle_input(&mut self, bounds: Rect, input: WidgetInput) -> Option<WidgetOutput> {
        self.inner.handle_input(bounds, input)
    }

    fn handle_wheel_sample(
        &mut self,
        bounds: Rect,
        position: Point,
        sample: WheelSample,
    ) -> Option<WidgetOutput> {
        self.inner.handle_wheel_sample(bounds, position, sample)
    }

    fn retains_managed_wheel_sequence(&self) -> bool {
        false
    }

    fn append_paint(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        bounds: Rect,
        layout: &radiant::layout::LayoutOutput,
        theme: &ThemeTokens,
    ) {
        self.inner.append_paint(primitives, bounds, layout, theme)
    }
}

struct RuntimeWheelBridge {
    observations: Rc<RefCell<Vec<RuntimeWheelObservation>>>,
    disabled: bool,
    read_only: bool,
    retention_enabled: bool,
    remove_owner: bool,
    replace_owner: bool,
    target_focusable: bool,
}

impl Default for RuntimeWheelBridge {
    fn default() -> Self {
        Self {
            observations: Rc::new(RefCell::new(Vec::new())),
            disabled: false,
            read_only: false,
            retention_enabled: true,
            remove_owner: false,
            replace_owner: false,
            target_focusable: false,
        }
    }
}

impl RuntimeWheelBridge {
    fn with_target_focusable(mut self) -> Self {
        self.target_focusable = true;
        self
    }
}

impl RuntimeBridge<()> for RuntimeWheelBridge {
    fn project_surface(&mut self) -> Arc<UiSurface<()>> {
        let mut children = Vec::new();
        if !self.remove_owner {
            let owner = if self.replace_owner {
                SurfaceNode::custom_widget(
                    RuntimeReplacementWheelWidget::new(
                        Rc::clone(&self.observations),
                        self.disabled,
                        self.read_only,
                        self.retention_enabled,
                    ),
                    WidgetMessageMapper::none(),
                )
            } else {
                SurfaceNode::custom_widget(
                    RuntimeWheelWidget::new(
                        401,
                        Rc::clone(&self.observations),
                        true,
                        self.disabled,
                        self.read_only,
                        self.retention_enabled,
                        false,
                    ),
                    WidgetMessageMapper::none(),
                )
            };
            children.push(SurfaceChild::fill(owner));
        }
        children.push(SurfaceChild::fill(SurfaceNode::custom_widget(
            RuntimeWheelWidget::new(
                402,
                Rc::clone(&self.observations),
                self.target_focusable,
                false,
                false,
                self.retention_enabled,
                false,
            ),
            WidgetMessageMapper::none(),
        )));
        arc_surface(UiSurface::new(SurfaceNode::column(1, 0.0, children)))
    }
}

fn runtime_wheel_sample(delta: WheelDelta, phase: WheelPhase) -> WheelSample {
    WheelSample::new(delta, Some(phase), PointerModifiers::default())
        .expect("runtime fixture uses finite wheel samples")
}

fn runtime_wheel_observation(
    widget_id: u64,
    delta: WheelDelta,
    phase: WheelPhase,
    replacement: bool,
) -> RuntimeWheelObservation {
    RuntimeWheelObservation {
        widget_id,
        delta,
        phase: Some(phase),
        replacement,
    }
}

#[test]
fn public_exact_wheel_sequence_retains_pending_no_output_and_routes_owner_first() {
    let mut runtime = SurfaceRuntime::new(RuntimeWheelBridge::default(), Vector2::new(160.0, 80.0));
    assert!(runtime.focus_widget(401));

    let owner_delta = WheelDelta::lines(Vector2::new(0.0, 1.0)).expect("finite line delta");
    let changed_delta = WheelDelta::pixels(Vector2::new(0.0, 2.5)).expect("finite pixel delta");
    assert!(runtime.wheel_or_scroll_at_with_sample(
        Point::new(20.0, 20.0),
        runtime_wheel_sample(owner_delta, WheelPhase::Started),
    ));
    assert!(runtime.wheel_or_scroll_at_with_sample(
        Point::new(20.0, 60.0),
        runtime_wheel_sample(changed_delta, WheelPhase::Changed),
    ));
    assert!(runtime.wheel_or_scroll_at_with_sample(
        Point::new(20.0, 60.0),
        runtime_wheel_sample(changed_delta, WheelPhase::Ended),
    ));

    assert_eq!(
        *runtime.bridge().observations.borrow(),
        vec![
            runtime_wheel_observation(401, owner_delta, WheelPhase::Started, false),
            runtime_wheel_observation(401, changed_delta, WheelPhase::Changed, false),
            runtime_wheel_observation(401, changed_delta, WheelPhase::Ended, false),
        ]
    );

    assert!(runtime.wheel_or_scroll_at_with_sample(
        Point::new(20.0, 60.0),
        runtime_wheel_sample(changed_delta, WheelPhase::Started),
    ));
    assert_eq!(
        runtime.bridge().observations.borrow().last().copied(),
        Some(runtime_wheel_observation(
            402,
            changed_delta,
            WheelPhase::Started,
            false
        ))
    );
}

#[test]
fn public_wheel_sequence_clears_on_focus_and_authority_boundaries() {
    let mut runtime = SurfaceRuntime::new(
        RuntimeWheelBridge::default().with_target_focusable(),
        Vector2::new(160.0, 80.0),
    );
    assert!(runtime.focus_widget(401));
    let delta = WheelDelta::pixels(Vector2::new(0.0, 1.0)).expect("finite pixel delta");
    assert!(runtime.wheel_or_scroll_at_with_sample(
        Point::new(20.0, 20.0),
        runtime_wheel_sample(delta, WheelPhase::Started),
    ));

    assert!(runtime.focus_widget(402));
    assert!(!runtime.wheel_or_scroll_at_with_sample(
        Point::new(20.0, 60.0),
        runtime_wheel_sample(delta, WheelPhase::Changed),
    ));
    assert_eq!(
        runtime.bridge().observations.borrow().as_slice(),
        [
            runtime_wheel_observation(401, delta, WheelPhase::Started, false),
            runtime_wheel_observation(402, delta, WheelPhase::Changed, false),
        ]
    );

    let mut runtime = SurfaceRuntime::new(RuntimeWheelBridge::default(), Vector2::new(160.0, 80.0));
    assert!(runtime.focus_widget(401));
    assert!(runtime.wheel_or_scroll_at_with_sample(
        Point::new(20.0, 20.0),
        runtime_wheel_sample(delta, WheelPhase::Started),
    ));
    runtime.bridge_mut().disabled = true;
    runtime.refresh();
    assert!(!runtime.wheel_or_scroll_at_with_sample(
        Point::new(20.0, 20.0),
        runtime_wheel_sample(delta, WheelPhase::Changed),
    ));
    assert_eq!(
        runtime.bridge().observations.borrow().as_slice(),
        [runtime_wheel_observation(
            401,
            delta,
            WheelPhase::Started,
            false
        )]
    );

    let mut runtime = SurfaceRuntime::new(RuntimeWheelBridge::default(), Vector2::new(160.0, 80.0));
    assert!(runtime.focus_widget(401));
    assert!(runtime.wheel_or_scroll_at_with_sample(
        Point::new(20.0, 20.0),
        runtime_wheel_sample(delta, WheelPhase::Started),
    ));
    runtime.bridge_mut().read_only = true;
    runtime.refresh();
    assert!(!runtime.wheel_or_scroll_at_with_sample(
        Point::new(20.0, 20.0),
        runtime_wheel_sample(delta, WheelPhase::Changed),
    ));
    assert_eq!(
        runtime.bridge().observations.borrow().as_slice(),
        [runtime_wheel_observation(
            401,
            delta,
            WheelPhase::Started,
            false
        )]
    );

    let mut runtime = SurfaceRuntime::new(RuntimeWheelBridge::default(), Vector2::new(160.0, 80.0));
    assert!(runtime.focus_widget(401));
    assert!(runtime.wheel_or_scroll_at_with_sample(
        Point::new(20.0, 20.0),
        runtime_wheel_sample(delta, WheelPhase::Started),
    ));
    runtime.bridge_mut().retention_enabled = false;
    runtime.refresh();
    assert!(!runtime.wheel_or_scroll_at_with_sample(
        Point::new(20.0, 20.0),
        runtime_wheel_sample(delta, WheelPhase::Changed),
    ));
    assert_eq!(
        runtime.bridge().observations.borrow().as_slice(),
        [runtime_wheel_observation(
            401,
            delta,
            WheelPhase::Started,
            false
        )]
    );
}

#[test]
fn public_wheel_sequence_refresh_preserves_compatible_owner_and_clears_replacement_or_removal() {
    let mut runtime = SurfaceRuntime::new(RuntimeWheelBridge::default(), Vector2::new(160.0, 80.0));
    assert!(runtime.focus_widget(401));
    let delta = WheelDelta::pixels(Vector2::new(0.0, 1.0)).expect("finite pixel delta");
    assert!(runtime.wheel_or_scroll_at_with_sample(
        Point::new(20.0, 20.0),
        runtime_wheel_sample(delta, WheelPhase::Started),
    ));

    runtime.refresh();
    assert!(runtime.wheel_or_scroll_at_with_sample(
        Point::new(20.0, 60.0),
        runtime_wheel_sample(delta, WheelPhase::Changed),
    ));
    assert_eq!(
        runtime.bridge().observations.borrow().as_slice(),
        [
            runtime_wheel_observation(401, delta, WheelPhase::Started, false),
            runtime_wheel_observation(401, delta, WheelPhase::Changed, false),
        ]
    );

    runtime.bridge_mut().replace_owner = true;
    runtime.refresh();
    assert!(!runtime.wheel_or_scroll_at_with_sample(
        Point::new(20.0, 20.0),
        runtime_wheel_sample(delta, WheelPhase::Changed),
    ));
    assert_eq!(
        runtime.bridge().observations.borrow().last().copied(),
        Some(runtime_wheel_observation(
            401,
            delta,
            WheelPhase::Changed,
            true
        ))
    );

    let mut runtime = SurfaceRuntime::new(RuntimeWheelBridge::default(), Vector2::new(160.0, 80.0));
    assert!(runtime.focus_widget(401));
    assert!(runtime.wheel_or_scroll_at_with_sample(
        Point::new(20.0, 20.0),
        runtime_wheel_sample(delta, WheelPhase::Started),
    ));
    runtime.bridge_mut().remove_owner = true;
    runtime.refresh();
    assert!(!runtime.wheel_or_scroll_at_with_sample(
        Point::new(20.0, 60.0),
        runtime_wheel_sample(delta, WheelPhase::Changed),
    ));
    assert_eq!(
        runtime.bridge().observations.borrow().as_slice(),
        [runtime_wheel_observation(
            401,
            delta,
            WheelPhase::Started,
            false
        )]
    );
}
