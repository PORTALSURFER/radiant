use super::*;
use radiant::{
    application::{ApplicationEnvironment, LocaleId, TextScale, numeric_input},
    runtime::{NumericAccessibilityDispatchResult, NumericAccessibilityRequest},
    widgets::{
        CompositionRange, CompositionSample, EditPhase, InteractionProvenance, KeyboardModifier,
        KeyboardModifiers, NumericAccessibilityAction, NumericAdjustment, NumericCodec,
        NumericInputInteraction, NumericInputInteractionBatch, NumericParseResult,
        NumericScrubPolicy, NumericStep, NumericStepDirection, NumericStepModifiers,
        NumericWheelPolicy, PointerModifiers, TextInputWidget, ToggleWidget, WheelDelta,
        WheelPhase, WheelSample,
    },
};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

#[derive(Clone, Copy)]
enum ScrollFocusedControl {
    Button,
    Toggle,
    TextInput,
}

fn intrinsic_scroll_slot() -> SlotParams {
    SlotParams {
        size_main: SizeModeMain::Intrinsic,
        size_cross: SizeModeCross::Fill,
        constraints: Constraints::unconstrained(),
        margin: Default::default(),
        align_cross_override: None,
        allow_fixed_compress: false,
    }
}

struct ScrollFocusedBridge {
    control: ScrollFocusedControl,
    host_calls: usize,
    host_handled: bool,
}

impl ScrollFocusedBridge {
    fn new(control: ScrollFocusedControl) -> Self {
        Self {
            control,
            host_calls: 0,
            host_handled: false,
        }
    }
}

impl RuntimeBridge<()> for ScrollFocusedBridge {
    fn project_surface(&mut self) -> Arc<UiSurface<()>> {
        let child = match self.control {
            ScrollFocusedControl::Button => SurfaceNode::widget(
                ButtonWidget::new(50, "Button", WidgetSizing::fixed(Vector2::new(120.0, 40.0))),
                WidgetMessageMapper::none(),
            ),
            ScrollFocusedControl::Toggle => SurfaceNode::widget(
                ToggleWidget::new(50, "Toggle", WidgetSizing::fixed(Vector2::new(120.0, 40.0))),
                WidgetMessageMapper::none(),
            ),
            ScrollFocusedControl::TextInput => SurfaceNode::widget(
                TextInputWidget::new(
                    50,
                    "initial text",
                    WidgetSizing::fixed(Vector2::new(120.0, 40.0)),
                ),
                WidgetMessageMapper::none(),
            ),
        };
        let content = SurfaceNode::column(
            2,
            0.0,
            vec![
                SurfaceChild::new(intrinsic_scroll_slot(), child),
                SurfaceChild::new(
                    intrinsic_scroll_slot(),
                    SurfaceNode::text(
                        70,
                        "filler",
                        WidgetSizing::fixed(Vector2::new(120.0, 360.0)),
                    ),
                ),
            ],
        );
        arc_surface(UiSurface::new(SurfaceNode::scroll_area(1, content)))
    }

    fn host_capabilities(&self) -> RuntimeHostCapabilities<Self, ()> {
        RuntimeHostCapabilities::new().with_input()
    }
}

impl RuntimeInputHost<()> for ScrollFocusedBridge {
    fn resolve_key_press(
        &mut self,
        _pending_chord: Option<KeyPress>,
        _press: KeyPress,
        _focus: FocusSurface,
    ) -> ShortcutResolution<()> {
        self.host_calls += 1;
        if self.host_handled {
            ShortcutResolution::handled()
        } else {
            ShortcutResolution::unhandled()
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ReprojectMessage {
    KeyPress,
}

#[derive(Clone)]
struct ReprojectingFocusedWidget {
    common: WidgetCommon,
}

impl ReprojectingFocusedWidget {
    fn new() -> Self {
        Self {
            common: WidgetCommon::new(61, WidgetSizing::fixed(Vector2::new(120.0, 40.0)))
                .with_keyboard_focus(),
        }
    }
}

impl Widget for ReprojectingFocusedWidget {
    fn common(&self) -> &WidgetCommon {
        &self.common
    }

    fn common_mut(&mut self) -> &mut WidgetCommon {
        &mut self.common
    }

    fn focused_key_disposition(&self, key: WidgetKey) -> radiant::widgets::FocusedKeyDisposition {
        if matches!(
            key,
            WidgetKey::PageUp | WidgetKey::PageDown | WidgetKey::Home | WidgetKey::End
        ) {
            radiant::widgets::FocusedKeyDisposition::Unhandled
        } else {
            radiant::widgets::FocusedKeyDisposition::Consumed
        }
    }

    fn handle_input(&mut self, _bounds: Rect, input: WidgetInput) -> Option<WidgetOutput> {
        matches!(
            input,
            WidgetInput::KeyPress {
                key: WidgetKey::PageDown,
                ..
            }
        )
        .then(|| WidgetOutput::typed(ReprojectMessage::KeyPress))
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

struct ReprojectingBridge {
    reductions: usize,
}

impl RuntimeBridge<ReprojectMessage> for ReprojectingBridge {
    fn project_surface(&mut self) -> Arc<UiSurface<ReprojectMessage>> {
        let content = SurfaceNode::column(
            62,
            0.0,
            vec![
                SurfaceChild::new(
                    intrinsic_scroll_slot(),
                    SurfaceNode::widget(
                        ReprojectingFocusedWidget::new(),
                        WidgetMessageMapper::typed(|message: ReprojectMessage| message),
                    ),
                ),
                SurfaceChild::new(
                    intrinsic_scroll_slot(),
                    SurfaceNode::text(
                        70,
                        "filler",
                        WidgetSizing::fixed(Vector2::new(120.0, 360.0)),
                    ),
                ),
            ],
        );
        arc_surface(UiSurface::new(SurfaceNode::scroll_area(60, content)))
    }

    fn reduce_message(&mut self, _message: ReprojectMessage) {
        self.reductions += 1;
    }
}

#[test]
fn focused_activation_controls_admit_page_keys_and_preserve_event_projection() {
    for control in [ScrollFocusedControl::Button, ScrollFocusedControl::Toggle] {
        let mut runtime =
            SurfaceRuntime::new(ScrollFocusedBridge::new(control), Vector2::new(120.0, 80.0));
        assert!(runtime.focus_widget(50));
        let initial = runtime.layout().rects[&50];
        assert!(runtime.scroll_at(Point::new(10.0, 10.0), Vector2::new(0.0, 40.0)));
        let manually_scrolled = runtime.layout().rects[&50];
        assert!(manually_scrolled.min.y < initial.min.y);
        runtime.dispatch_event(Event::key_press(WidgetKey::Home));
        let initial = runtime.layout().rects[&50];

        assert_eq!(
            runtime.dispatch_event(Event::key_press(WidgetKey::PageDown)),
            Some(50)
        );
        let paged = runtime.layout().rects[&50];
        assert!(paged.min.y < initial.min.y);

        assert_eq!(
            runtime.dispatch_event(Event::key_press(WidgetKey::End)),
            Some(50)
        );
        let ended = runtime.layout().rects[&50];
        assert!(ended.min.y < paged.min.y);

        assert_eq!(
            runtime.dispatch_event(Event::key_press(WidgetKey::Home)),
            Some(50)
        );
        assert_eq!(runtime.layout().rects[&50].min.y, initial.min.y);
    }
}

#[test]
fn host_first_page_down_falls_back_once_for_focused_button() {
    let mut runtime = SurfaceRuntime::new(
        ScrollFocusedBridge::new(ScrollFocusedControl::Button),
        Vector2::new(120.0, 80.0),
    );
    assert!(runtime.focus_widget(50));
    let initial = runtime.layout().rects[&50];
    assert!(runtime.scroll_at(Point::new(10.0, 10.0), Vector2::new(0.0, 40.0)));
    let manually_scrolled = runtime.layout().rects[&50];
    assert!(manually_scrolled.min.y < initial.min.y);
    runtime.dispatch_event(Event::key_press(WidgetKey::Home));
    let initial = runtime.layout().rects[&50];

    assert!(runtime.dispatch_key_press(
        KeyPress::new(KeyCode::PageDown),
        Some(WidgetKey::PageDown),
        FocusSurface::None,
    ));
    assert_eq!(runtime.bridge().host_calls, 1);
    assert!(runtime.layout().rects[&50].min.y < initial.min.y);
}

#[test]
fn host_handled_page_down_blocks_focused_delivery_and_scroll_fallback() {
    let mut runtime = SurfaceRuntime::new(
        ScrollFocusedBridge::new(ScrollFocusedControl::Button),
        Vector2::new(120.0, 80.0),
    );
    runtime.bridge_mut().host_handled = true;
    assert!(runtime.focus_widget(50));
    let initial = runtime.layout().rects[&50];

    assert!(runtime.dispatch_key_press(
        KeyPress::new(KeyCode::PageDown),
        Some(WidgetKey::PageDown),
        FocusSurface::None,
    ));
    assert_eq!(runtime.bridge().host_calls, 1);
    assert_eq!(runtime.layout().rects[&50], initial);
}

#[test]
fn text_input_home_and_end_are_consumed_without_scroll_fallback() {
    let mut runtime = SurfaceRuntime::new(
        ScrollFocusedBridge::new(ScrollFocusedControl::TextInput),
        Vector2::new(120.0, 80.0),
    );
    assert!(runtime.focus_widget(50));
    let initial = runtime.layout().rects[&50];
    assert!(runtime.scroll_at(Point::new(10.0, 10.0), Vector2::new(0.0, 40.0)));
    let manually_scrolled = runtime.layout().rects[&50];
    assert!(manually_scrolled.min.y < initial.min.y);
    runtime.dispatch_event(Event::key_press(WidgetKey::Home));
    let initial = runtime.layout().rects[&50];

    assert_eq!(
        runtime.dispatch_event(Event::key_press(WidgetKey::PageDown)),
        Some(50)
    );
    let paged = runtime.layout().rects[&50];
    assert!(paged.min.y < initial.min.y);

    assert_eq!(
        runtime.dispatch_event(Event::key_press(WidgetKey::Home)),
        Some(50)
    );
    assert_eq!(runtime.layout().rects[&50].min.y, paged.min.y);
    assert_eq!(
        runtime.dispatch_event(Event::key_press(WidgetKey::End)),
        Some(50)
    );
    assert_eq!(runtime.layout().rects[&50].min.y, paged.min.y);
}

#[test]
fn focused_delivery_reprojection_disables_stale_keyboard_fallback() {
    let mut runtime = SurfaceRuntime::new(
        ReprojectingBridge { reductions: 0 },
        Vector2::new(120.0, 80.0),
    );
    assert!(runtime.focus_widget(61));
    let initial = runtime.layout().rects[&61];

    assert_eq!(
        runtime.dispatch_event(Event::key_press(WidgetKey::PageDown)),
        Some(61)
    );
    assert_eq!(runtime.bridge().reductions, 1);
    assert_eq!(runtime.layout().rects[&61].min.y, initial.min.y);
}

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
    wheel_calls: Rc<Cell<usize>>,
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
        delta: f32,
        _step: NumericStep,
    ) -> Result<RuntimeNumericValue, Self::Error> {
        self.wheel_calls.set(self.wheel_calls.get() + 1);
        Ok(RuntimeNumericValue(
            value.0.saturating_add(delta.round().max(0.0) as u32),
        ))
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
    application_environment: ApplicationEnvironment,
    projected_text_scales: Vec<f32>,
    host_calls: usize,
    host_handled: bool,
    mapped_phases: Vec<Vec<EditPhase>>,
    format_calls: Rc<Cell<usize>>,
    parse_calls: Rc<Cell<usize>>,
    inverse_calls: Rc<Cell<usize>>,
    step_calls: Rc<Cell<usize>>,
    wheel_calls: Rc<Cell<usize>>,
    mapped_provenance: Vec<Vec<InteractionProvenance>>,
}

impl Default for RuntimeNumericBridge {
    fn default() -> Self {
        Self {
            value: RuntimeNumericValue(7),
            application_environment: ApplicationEnvironment::new(LocaleId::english()),
            projected_text_scales: Vec::new(),
            host_calls: 0,
            host_handled: true,
            mapped_phases: Vec::new(),
            format_calls: Rc::new(Cell::new(0)),
            parse_calls: Rc::new(Cell::new(0)),
            inverse_calls: Rc::new(Cell::new(0)),
            step_calls: Rc::new(Cell::new(0)),
            wheel_calls: Rc::new(Cell::new(0)),
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
    fn application_environment(&mut self) -> Option<ApplicationEnvironment> {
        Some(self.application_environment.clone())
    }

    fn project_surface(&mut self) -> Arc<UiSurface<RuntimeNumericMessage>> {
        self.projected_text_scales
            .push(self.application_environment.text_scale().factor());
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
                    wheel_calls: Rc::clone(&self.wheel_calls),
                },
            )
            .expect("runtime numeric fixture should construct")
            .step_modifiers(NumericStepModifiers::new(
                KeyboardModifier::Shift,
                KeyboardModifier::Control,
            ))
            .scrub_policy(NumericScrubPolicy::default())
            .wheel_policy(NumericWheelPolicy::default())
            .on_interaction(RuntimeNumericMessage::Interaction)
            .id(150)
            .into_surface()
            .with_application_environment(self.application_environment.clone()),
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum LiveNumericOwner {
    TextEdit,
    ImeComposition,
    KeyboardAdjustment,
    PointerScrub,
    WheelSequence,
}

#[derive(Clone, Debug, PartialEq)]
struct RuntimeNumericProjectionSnapshot {
    draft: String,
    caret: usize,
    selection_anchor: usize,
    bounds: Rect,
    text_rect: Rect,
    font_size: f32,
    automation_value: Option<String>,
}

fn runtime_numeric_projection(
    runtime: &SurfaceRuntime<RuntimeNumericBridge, RuntimeNumericMessage>,
) -> RuntimeNumericProjectionSnapshot {
    let paint = runtime
        .paint_plan(&ThemeTokens::default())
        .primitives
        .into_iter()
        .find_map(|primitive| match primitive {
            PaintPrimitive::TextInput(input) if input.widget_id == 150 => Some(input),
            _ => None,
        })
        .expect("runtime numeric projection should paint its text input");
    let target = runtime
        .automation_target_snapshot()
        .targets
        .into_iter()
        .find(|target| target.id.0 == "150")
        .expect("runtime numeric target should be materialized");
    RuntimeNumericProjectionSnapshot {
        draft: paint.state.value.clone(),
        caret: paint.state.caret,
        selection_anchor: paint.state.selection_anchor,
        bounds: runtime.layout().rects[&150],
        text_rect: paint.rect,
        font_size: paint.font_size,
        automation_value: target.value,
    }
}

fn runtime_numeric_target(
    runtime: &SurfaceRuntime<RuntimeNumericBridge, RuntimeNumericMessage>,
) -> radiant::gui::automation::AutomationTarget {
    runtime
        .automation_target_snapshot()
        .targets
        .into_iter()
        .find(|target| target.id.0 == "150")
        .expect("runtime numeric target should be materialized")
}

#[test]
fn public_numeric_text_scale_refresh_retains_each_live_owner_and_geometry() {
    for owner in [
        LiveNumericOwner::TextEdit,
        LiveNumericOwner::ImeComposition,
        LiveNumericOwner::KeyboardAdjustment,
        LiveNumericOwner::PointerScrub,
        LiveNumericOwner::WheelSequence,
    ] {
        let mut runtime =
            SurfaceRuntime::new(RuntimeNumericBridge::default(), Vector2::new(120.0, 32.0));
        assert!(runtime.focus_widget(150));

        match owner {
            LiveNumericOwner::TextEdit => {
                assert_eq!(
                    runtime.dispatch_event(Event::Character {
                        character: '8',
                        timestamp: None,
                    }),
                    Some(150)
                );
                assert_eq!(
                    runtime.dispatch_focused_input(WidgetInput::text_edit(
                        TextEditCommand::SelectAll,
                    )),
                    Some(150)
                );
            }
            LiveNumericOwner::ImeComposition => {
                let replacement =
                    CompositionRange::new(0, 1, 1).expect("numeric replacement range");
                let selection = CompositionRange::new(1, 1, 1).expect("numeric selection");
                assert_eq!(
                    runtime.dispatch_composition_sample(
                        CompositionSample::start(replacement, selection)
                            .expect("numeric composition start"),
                    ),
                    Some(150)
                );
                assert_eq!(
                    runtime.dispatch_composition_sample(
                        CompositionSample::update(
                            "12",
                            CompositionRange::new(0, 1, 2).expect("numeric preedit selection"),
                        )
                        .expect("numeric composition update"),
                    ),
                    Some(150)
                );
            }
            LiveNumericOwner::KeyboardAdjustment => {
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
                assert_eq!(
                    runtime.dispatch_event(Event::KeyPress {
                        key: WidgetKey::ArrowUp,
                        modifiers: KeyboardModifiers::default(),
                        repeat: true,
                        timestamp: None,
                    }),
                    Some(150)
                );
            }
            LiveNumericOwner::PointerScrub => {
                let modifiers = PointerModifiers {
                    alt: true,
                    ..PointerModifiers::default()
                };
                assert_eq!(
                    runtime.dispatch_event(Event::PointerPress {
                        position: Point::new(10.0, 16.0),
                        button: PointerButton::Primary,
                        modifiers,
                        timestamp: None,
                    }),
                    Some(150)
                );
                assert_eq!(runtime.pointer_capture(), Some(150));
                assert_eq!(
                    runtime.dispatch_event(Event::PointerMove {
                        position: Point::new(110.0, 16.0),
                        modifiers,
                        timestamp: None,
                        sequence_range: None,
                    }),
                    Some(150)
                );
            }
            LiveNumericOwner::WheelSequence => {
                let point = Point::new(40.0, 16.0);
                assert!(
                    runtime.wheel_or_scroll_at_with_sample(
                        point,
                        WheelSample::new(
                            WheelDelta::lines(Vector2::new(0.0, 1.0)).expect("wheel start delta"),
                            Some(WheelPhase::Started),
                            PointerModifiers::default(),
                        )
                        .expect("wheel start sample"),
                    )
                );
                assert!(
                    runtime.wheel_or_scroll_at_with_sample(
                        point,
                        WheelSample::new(
                            WheelDelta::pixels(Vector2::new(0.0, 40.0))
                                .expect("wheel changed delta"),
                            Some(WheelPhase::Changed),
                            PointerModifiers::default(),
                        )
                        .expect("wheel changed sample"),
                    )
                );
            }
        }

        let before = runtime_numeric_projection(&runtime);
        assert_ne!(before.draft, "7");
        if matches!(
            owner,
            LiveNumericOwner::TextEdit | LiveNumericOwner::ImeComposition
        ) {
            assert_ne!(
                before.caret, before.selection_anchor,
                "owner {owner:?} should retain a noncollapsed live selection: {before:?}"
            );
        }
        let before_counters = runtime.refresh_counters();
        let mapped_before = runtime.bridge().mapped_phases.clone();
        runtime.bridge_mut().application_environment =
            ApplicationEnvironment::new(LocaleId::english())
                .with_text_scale(TextScale::new(1.5).expect("numeric runtime scale"));
        runtime.refresh_with_scope(RepaintScope::PaintOnly);

        let after = runtime_numeric_projection(&runtime);
        assert_eq!(
            runtime.refresh_counters().layout,
            before_counters.layout + 1
        );
        assert_eq!(after.bounds, before.bounds);
        assert_eq!(after.draft, before.draft);
        assert_eq!(after.caret, before.caret);
        assert_eq!(after.selection_anchor, before.selection_anchor);
        assert_eq!(after.font_size, before.font_size * 1.5);
        assert_eq!(after.text_rect.min.x, after.bounds.min.x + 24.0);
        assert_eq!(after.text_rect.min.y, after.bounds.min.y + 6.0);
        assert_eq!(after.automation_value, Some(after.draft.clone()));
        assert_eq!(runtime.bridge().mapped_phases, mapped_before);
        assert_eq!(runtime.bridge().projected_text_scales.last(), Some(&1.5));

        let current_target = runtime_numeric_target(&runtime);
        assert!(current_target.interaction_target);
        assert!(
            current_target
                .authority
                .is_some_and(|authority| authority.materialized)
        );
        let blocked = runtime.dispatch_numeric_accessibility_action(
            NumericAccessibilityRequest::new(current_target, NumericAccessibilityAction::Increment),
        );
        assert_eq!(
            blocked,
            NumericAccessibilityDispatchResult::Blocked {
                owner: match owner {
                    LiveNumericOwner::TextEdit => {
                        radiant::widgets::NumericAccessibilityBlockOwner::TextEdit
                    }
                    LiveNumericOwner::ImeComposition => {
                        radiant::widgets::NumericAccessibilityBlockOwner::ImeComposition
                    }
                    LiveNumericOwner::KeyboardAdjustment => {
                        radiant::widgets::NumericAccessibilityBlockOwner::KeyboardAdjustment
                    }
                    LiveNumericOwner::PointerScrub => {
                        radiant::widgets::NumericAccessibilityBlockOwner::PointerScrub
                    }
                    LiveNumericOwner::WheelSequence => {
                        radiant::widgets::NumericAccessibilityBlockOwner::WheelSequence
                    }
                }
            }
        );
        assert_eq!(runtime.bridge().mapped_phases, mapped_before);

        match owner {
            LiveNumericOwner::TextEdit => {
                assert_eq!(
                    runtime.dispatch_focused_input(WidgetInput::key_press(WidgetKey::Enter)),
                    Some(150)
                );
            }
            LiveNumericOwner::ImeComposition => {
                assert_eq!(
                    runtime.dispatch_composition_sample(CompositionSample::commit("8")),
                    Some(150)
                );
            }
            LiveNumericOwner::KeyboardAdjustment => {
                assert_eq!(
                    runtime.dispatch_event(Event::KeyRelease {
                        key: WidgetKey::ArrowUp,
                        modifiers: KeyboardModifiers::default(),
                        timestamp: None,
                    }),
                    Some(150)
                );
            }
            LiveNumericOwner::PointerScrub => {
                let modifiers = PointerModifiers {
                    alt: true,
                    ..PointerModifiers::default()
                };
                assert_eq!(
                    runtime.dispatch_event(Event::PointerRelease {
                        position: Point::new(110.0, 16.0),
                        button: PointerButton::Primary,
                        modifiers,
                        timestamp: None,
                    }),
                    Some(150)
                );
                assert_eq!(runtime.pointer_capture(), None);
            }
            LiveNumericOwner::WheelSequence => {
                assert!(
                    runtime.wheel_or_scroll_at_with_sample(
                        Point::new(40.0, 16.0),
                        WheelSample::new(
                            WheelDelta::pixels(Vector2::new(0.0, 0.0)).expect("wheel end delta"),
                            Some(WheelPhase::Ended),
                            PointerModifiers::default(),
                        )
                        .expect("wheel end sample"),
                    )
                );
            }
        }

        let expected_terminal = match owner {
            LiveNumericOwner::TextEdit | LiveNumericOwner::ImeComposition => {
                vec![vec![EditPhase::Begin, EditPhase::Commit]]
            }
            LiveNumericOwner::KeyboardAdjustment => vec![
                vec![EditPhase::Begin, EditPhase::Update],
                vec![EditPhase::Update],
                vec![EditPhase::Commit],
            ],
            LiveNumericOwner::PointerScrub | LiveNumericOwner::WheelSequence => vec![
                vec![EditPhase::Begin, EditPhase::Update],
                vec![EditPhase::Commit],
            ],
        };
        assert_eq!(runtime.bridge().mapped_phases, expected_terminal);
        if owner == LiveNumericOwner::WheelSequence {
            let wheel_calls = runtime.bridge().wheel_calls.get();
            assert!(
                !runtime.wheel_or_scroll_at_with_sample(
                    Point::new(40.0, 16.0),
                    WheelSample::new(
                        WheelDelta::pixels(Vector2::new(0.0, 40.0)).expect("orphan wheel delta"),
                        Some(WheelPhase::Changed),
                        PointerModifiers::default(),
                    )
                    .expect("orphan wheel sample"),
                )
            );
            assert_eq!(runtime.bridge().wheel_calls.get(), wheel_calls);
            assert_eq!(runtime.bridge().mapped_phases, expected_terminal);
        }
        assert_eq!(runtime.dispatch_event(Event::ClearFocus), None);
        assert_eq!(runtime.focused_widget(), None);
        assert_eq!(runtime.pointer_capture(), None);
    }
}

#[test]
fn public_numeric_composition_keeps_preedit_local_until_one_valid_commit() {
    let mut runtime =
        SurfaceRuntime::new(RuntimeNumericBridge::default(), Vector2::new(120.0, 32.0));
    assert!(runtime.focus_widget(150));

    let replacement = CompositionRange::new(0, 1, 1).expect("one-scalar replacement range");
    let selection = CompositionRange::new(1, 1, 1).expect("collapsed numeric selection");
    assert_eq!(
        runtime.dispatch_composition_sample(
            CompositionSample::start(replacement, selection).expect("valid numeric start"),
        ),
        Some(150)
    );
    assert_eq!(
        runtime.dispatch_composition_sample(
            CompositionSample::update(
                "12",
                CompositionRange::new(1, 1, 2).expect("collapsed preedit selection"),
            )
            .expect("valid numeric preedit"),
        ),
        Some(150)
    );
    assert!(runtime.bridge().mapped_phases.is_empty());
    assert_eq!(runtime.bridge().value, RuntimeNumericValue(7));
    assert_eq!(runtime.bridge().parse_calls.get(), 0);

    runtime.refresh();
    assert_eq!(
        runtime.dispatch_composition_sample(CompositionSample::commit("8")),
        Some(150)
    );
    assert_eq!(
        runtime.bridge().mapped_phases,
        vec![vec![EditPhase::Begin, EditPhase::Commit]]
    );
    assert_eq!(runtime.bridge().value, RuntimeNumericValue(8));
    assert_eq!(runtime.bridge().parse_calls.get(), 1);
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
fn public_retained_numeric_home_end_are_consumed_and_page_keys_are_unhandled() {
    let mut runtime =
        SurfaceRuntime::new(RuntimeNumericBridge::default(), Vector2::new(120.0, 32.0));
    assert!(runtime.focus_widget(150));
    runtime.bridge_mut().host_handled = false;

    for key in [
        WidgetKey::Home,
        WidgetKey::End,
        WidgetKey::PageUp,
        WidgetKey::PageDown,
    ] {
        assert_eq!(
            runtime.dispatch_event(Event::key_press(key)),
            Some(150),
            "retained numeric wrapper should admit {key:?} exactly once"
        );
    }
    assert_eq!(runtime.bridge().value, RuntimeNumericValue(7));
    assert!(runtime.bridge().mapped_phases.is_empty());
    assert_eq!(runtime.bridge().host_calls, 4);
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

#[test]
fn public_numeric_wheel_consumes_exact_samples_and_legacy_scroll_stays_on_fallback() {
    let mut runtime =
        SurfaceRuntime::new(RuntimeNumericBridge::default(), Vector2::new(120.0, 32.0));
    assert!(runtime.focus_widget(150));
    let point = Point::new(40.0, 16.0);
    let modifiers = PointerModifiers::default();
    let timestamp = None;
    let sequence_range = None;
    let started = WheelSample::new_with_metadata(
        WheelDelta::lines(Vector2::new(0.0, 1.0)).expect("finite wheel line"),
        Some(WheelPhase::Started),
        modifiers,
        timestamp,
        sequence_range,
    )
    .expect("finite started sample");
    assert!(runtime.wheel_or_scroll_at_with_sample(point, started));
    assert_eq!(runtime.bridge().wheel_calls.get(), 0);
    assert!(runtime.bridge().mapped_phases.is_empty());

    let changed = WheelSample::new_with_metadata(
        WheelDelta::pixels(Vector2::new(0.0, 40.0)).expect("finite wheel pixels"),
        Some(WheelPhase::Changed),
        modifiers,
        timestamp,
        sequence_range,
    )
    .expect("finite changed sample");
    assert!(runtime.wheel_or_scroll_at_with_sample(point, changed));
    assert_eq!(runtime.bridge().wheel_calls.get(), 1);
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
                timestamp,
                sequence_range,
            },
            InteractionProvenance::Pointer {
                modifiers,
                timestamp,
                sequence_range,
            },
        ]]
    );

    let ended = WheelSample::new_with_metadata(
        WheelDelta::pixels(Vector2::new(0.0, 0.0)).expect("finite terminal pixels"),
        Some(WheelPhase::Ended),
        modifiers,
        None,
        None,
    )
    .expect("finite ended sample");
    assert!(runtime.wheel_or_scroll_at_with_sample(point, ended));
    assert_eq!(runtime.bridge().wheel_calls.get(), 1);
    assert_eq!(
        runtime.bridge().mapped_phases,
        vec![
            vec![EditPhase::Begin, EditPhase::Update],
            vec![EditPhase::Commit],
        ]
    );

    let mapped_before_legacy = runtime.bridge().mapped_phases.clone();
    let calls_before_legacy = runtime.bridge().wheel_calls.get();
    assert!(!runtime.wheel_or_scroll_at(point, Vector2::new(0.0, 40.0)));
    assert_eq!(runtime.bridge().wheel_calls.get(), calls_before_legacy);
    assert_eq!(runtime.bridge().mapped_phases, mapped_before_legacy);
}

#[test]
fn public_numeric_wheel_superseding_start_cancels_owner_before_retargeting() {
    let mut runtime =
        SurfaceRuntime::new(RuntimeNumericBridge::default(), Vector2::new(120.0, 32.0));
    assert!(runtime.focus_widget(150));
    let active_point = Point::new(40.0, 16.0);
    let outside_point = Point::new(200.0, 16.0);
    let delta = WheelDelta::pixels(Vector2::new(0.0, 40.0)).expect("finite wheel delta");

    assert!(runtime.wheel_or_scroll_at_with_sample(
        active_point,
        runtime_wheel_sample(delta, WheelPhase::Started),
    ));
    assert!(runtime.wheel_or_scroll_at_with_sample(
        active_point,
        runtime_wheel_sample(delta, WheelPhase::Changed),
    ));
    assert_eq!(runtime.bridge().value, RuntimeNumericValue(8));
    assert_eq!(
        runtime.bridge().mapped_phases,
        vec![vec![EditPhase::Begin, EditPhase::Update]]
    );

    assert!(!runtime.wheel_or_scroll_at_with_sample(
        outside_point,
        runtime_wheel_sample(delta, WheelPhase::Started),
    ));
    assert_eq!(runtime.bridge().value, RuntimeNumericValue(7));
    assert_eq!(
        runtime.bridge().mapped_phases,
        vec![
            vec![EditPhase::Begin, EditPhase::Update],
            vec![EditPhase::Cancel],
        ]
    );
    assert_eq!(runtime.bridge().wheel_calls.get(), 1);

    assert!(runtime.wheel_or_scroll_at_with_sample(
        active_point,
        runtime_wheel_sample(delta, WheelPhase::Started),
    ));
    assert!(runtime.wheel_or_scroll_at_with_sample(
        active_point,
        runtime_wheel_sample(delta, WheelPhase::Changed),
    ));
    assert_eq!(runtime.bridge().value, RuntimeNumericValue(8));
    assert_eq!(runtime.bridge().wheel_calls.get(), 2);
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
    terminal_reprojects: bool,
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
            terminal_reprojects: false,
        }
    }

    fn with_terminal_reprojection_if(mut self, enabled: bool) -> Self {
        self.terminal_reprojects = enabled;
        self
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

    fn handle_input(&mut self, _bounds: Rect, input: WidgetInput) -> Option<WidgetOutput> {
        if let WidgetInput::Wheel { delta, .. } = input {
            self.observations
                .borrow_mut()
                .push(RuntimeWheelObservation {
                    widget_id: self.common.id,
                    delta: WheelDelta::Pixels(delta),
                    phase: None,
                    replacement: self.replacement,
                });
        }
        None
    }

    fn accepts_wheel_input(&self) -> bool {
        !self.common.state.read_only
    }

    fn handle_wheel_sample(
        &mut self,
        _bounds: Rect,
        _position: Point,
        sample: WheelSample,
    ) -> Option<WidgetOutput> {
        self.record_sample(sample);
        (self.terminal_reprojects
            && matches!(
                sample.phase(),
                Some(WheelPhase::Ended | WheelPhase::Cancelled)
            ))
        .then_some(WidgetOutput::typed(RuntimeWheelMessage::RemoveOwner))
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

    fn accepts_wheel_input(&self) -> bool {
        self.inner.accepts_wheel_input()
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
    duplicate_owner: bool,
    target_focusable: bool,
    terminal_reprojects: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RuntimeWheelMessage {
    RemoveOwner,
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
            duplicate_owner: false,
            target_focusable: false,
            terminal_reprojects: false,
        }
    }
}

impl RuntimeWheelBridge {
    fn with_target_focusable(mut self) -> Self {
        self.target_focusable = true;
        self
    }

    fn with_terminal_reprojection(mut self) -> Self {
        self.terminal_reprojects = true;
        self
    }
}

#[test]
fn public_legacy_scroll_dispatches_legacy_wheel_input_not_the_exact_sample_hook() {
    let mut runtime = SurfaceRuntime::new(RuntimeWheelBridge::default(), Vector2::new(160.0, 80.0));
    let point = Point::new(20.0, 20.0);
    let delta = Vector2::new(0.0, 40.0);

    assert!(!runtime.wheel_or_scroll_at(point, delta));
    assert_eq!(
        runtime.bridge().observations.borrow().as_slice(),
        [RuntimeWheelObservation {
            widget_id: 401,
            delta: WheelDelta::Pixels(delta),
            phase: None,
            replacement: false,
        }]
    );

    assert!(runtime.wheel_or_scroll_at_with_sample(
        point,
        runtime_wheel_sample(WheelDelta::Pixels(delta), WheelPhase::Started,),
    ));
    assert_eq!(
        runtime.bridge().observations.borrow().as_slice(),
        [
            RuntimeWheelObservation {
                widget_id: 401,
                delta: WheelDelta::Pixels(delta),
                phase: None,
                replacement: false,
            },
            runtime_wheel_observation(401, WheelDelta::Pixels(delta), WheelPhase::Started, false),
        ]
    );
}

impl RuntimeBridge<RuntimeWheelMessage> for RuntimeWheelBridge {
    fn project_surface(&mut self) -> Arc<UiSurface<RuntimeWheelMessage>> {
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
                    )
                    .with_terminal_reprojection_if(self.terminal_reprojects),
                    WidgetMessageMapper::typed(|message: RuntimeWheelMessage| message),
                )
            };
            children.push(SurfaceChild::fill(owner));
        }
        if self.duplicate_owner {
            children.push(SurfaceChild::fill(SurfaceNode::custom_widget(
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
            )));
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

    fn reduce_message(&mut self, message: RuntimeWheelMessage) {
        match message {
            RuntimeWheelMessage::RemoveOwner => self.remove_owner = true,
        }
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

fn runtime_with_blocked_wheel(
    delta: WheelDelta,
) -> SurfaceRuntime<RuntimeWheelBridge, RuntimeWheelMessage> {
    let mut runtime = SurfaceRuntime::new(RuntimeWheelBridge::default(), Vector2::new(160.0, 80.0));
    assert!(runtime.focus_widget(401));
    assert!(runtime.wheel_or_scroll_at_with_sample(
        Point::new(20.0, 20.0),
        runtime_wheel_sample(delta, WheelPhase::Started),
    ));
    runtime.bridge_mut().retention_enabled = false;
    runtime.refresh();
    runtime
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

#[test]
fn public_blocked_wheel_ignores_multiple_changes_preserves_refresh_and_closes_on_terminal() {
    let delta = WheelDelta::pixels(Vector2::new(0.0, 1.0)).expect("finite pixel delta");
    let mut runtime = runtime_with_blocked_wheel(delta);

    for _ in 0..3 {
        assert!(!runtime.wheel_or_scroll_at_with_sample(
            Point::new(20.0, 60.0),
            runtime_wheel_sample(delta, WheelPhase::Changed),
        ));
    }
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
            false,
        )]
    );

    assert!(!runtime.wheel_or_scroll_at_with_sample(
        Point::new(20.0, 60.0),
        runtime_wheel_sample(delta, WheelPhase::Ended),
    ));
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

    let mut cancelled = runtime_with_blocked_wheel(delta);
    assert!(!cancelled.wheel_or_scroll_at_with_sample(
        Point::new(20.0, 60.0),
        runtime_wheel_sample(delta, WheelPhase::Cancelled),
    ));
    assert_eq!(
        cancelled.bridge().observations.borrow().as_slice(),
        [runtime_wheel_observation(
            401,
            delta,
            WheelPhase::Started,
            false,
        )]
    );
}

#[test]
fn public_wheel_started_supersedes_active_and_blocked_slots() {
    let delta = WheelDelta::pixels(Vector2::new(0.0, 1.0)).expect("finite pixel delta");
    let mut active = SurfaceRuntime::new(RuntimeWheelBridge::default(), Vector2::new(160.0, 80.0));
    assert!(active.focus_widget(401));
    assert!(active.wheel_or_scroll_at_with_sample(
        Point::new(20.0, 20.0),
        runtime_wheel_sample(delta, WheelPhase::Started),
    ));
    assert!(active.wheel_or_scroll_at_with_sample(
        Point::new(20.0, 60.0),
        runtime_wheel_sample(delta, WheelPhase::Started),
    ));
    assert!(active.wheel_or_scroll_at_with_sample(
        Point::new(20.0, 20.0),
        runtime_wheel_sample(delta, WheelPhase::Changed),
    ));
    assert_eq!(
        active.bridge().observations.borrow().as_slice(),
        [
            runtime_wheel_observation(401, delta, WheelPhase::Started, false),
            runtime_wheel_observation(
                401,
                WheelDelta::Pixels(Vector2::new(0.0, 0.0)),
                WheelPhase::Cancelled,
                false,
            ),
            runtime_wheel_observation(402, delta, WheelPhase::Started, false),
            runtime_wheel_observation(402, delta, WheelPhase::Changed, false),
        ]
    );

    let mut blocked = runtime_with_blocked_wheel(delta);
    blocked.bridge_mut().retention_enabled = true;
    blocked.refresh();
    assert!(blocked.wheel_or_scroll_at_with_sample(
        Point::new(20.0, 60.0),
        runtime_wheel_sample(delta, WheelPhase::Started),
    ));
    assert!(blocked.wheel_or_scroll_at_with_sample(
        Point::new(20.0, 20.0),
        runtime_wheel_sample(delta, WheelPhase::Changed),
    ));
    assert_eq!(
        blocked.bridge().observations.borrow().as_slice(),
        [
            runtime_wheel_observation(401, delta, WheelPhase::Started, false),
            runtime_wheel_observation(402, delta, WheelPhase::Started, false),
            runtime_wheel_observation(402, delta, WheelPhase::Changed, false),
        ]
    );
}

#[test]
fn public_phase_less_and_discrete_wheel_samples_do_not_replace_blocked() {
    let delta = WheelDelta::pixels(Vector2::new(0.0, 1.0)).expect("finite pixel delta");
    let mut runtime = runtime_with_blocked_wheel(delta);
    let phase_less = WheelSample::phase_less(delta, PointerModifiers::default())
        .expect("finite phase-less wheel sample");
    let discrete = WheelSample::discrete(delta, PointerModifiers::default())
        .expect("finite discrete wheel sample");

    assert!(!runtime.wheel_or_scroll_at_with_sample(Point::new(20.0, 60.0), phase_less));
    assert!(!runtime.wheel_or_scroll_at_with_sample(Point::new(20.0, 60.0), discrete));
    assert!(!runtime.wheel_or_scroll_at_with_sample(
        Point::new(20.0, 60.0),
        runtime_wheel_sample(delta, WheelPhase::Changed),
    ));
    assert_eq!(
        runtime.bridge().observations.borrow().as_slice(),
        [
            runtime_wheel_observation(401, delta, WheelPhase::Started, false),
            RuntimeWheelObservation {
                widget_id: 402,
                delta,
                phase: None,
                replacement: false,
            },
            RuntimeWheelObservation {
                widget_id: 402,
                delta,
                phase: Some(WheelPhase::Discrete),
                replacement: false,
            },
        ]
    );
}

#[test]
fn public_terminal_clears_before_reprojection_and_does_not_leave_an_orphan() {
    let delta = WheelDelta::pixels(Vector2::new(0.0, 1.0)).expect("finite pixel delta");
    let mut runtime = SurfaceRuntime::new(
        RuntimeWheelBridge::default().with_terminal_reprojection(),
        Vector2::new(160.0, 80.0),
    );
    assert!(runtime.focus_widget(401));
    assert!(runtime.wheel_or_scroll_at_with_sample(
        Point::new(20.0, 20.0),
        runtime_wheel_sample(delta, WheelPhase::Started),
    ));
    assert!(runtime.wheel_or_scroll_at_with_sample(
        Point::new(20.0, 20.0),
        runtime_wheel_sample(delta, WheelPhase::Ended),
    ));
    assert!(runtime.bridge().remove_owner);

    assert!(!runtime.wheel_or_scroll_at_with_sample(
        Point::new(20.0, 20.0),
        runtime_wheel_sample(delta, WheelPhase::Changed),
    ));
    assert_eq!(
        runtime.bridge().observations.borrow().as_slice(),
        [
            runtime_wheel_observation(401, delta, WheelPhase::Started, false),
            runtime_wheel_observation(401, delta, WheelPhase::Ended, false),
            runtime_wheel_observation(402, delta, WheelPhase::Changed, false),
        ]
    );
}

#[test]
fn public_duplicate_or_hard_replacement_blocks_before_retargeting() {
    let delta = WheelDelta::pixels(Vector2::new(0.0, 1.0)).expect("finite pixel delta");
    let mut duplicate =
        SurfaceRuntime::new(RuntimeWheelBridge::default(), Vector2::new(160.0, 80.0));
    assert!(duplicate.focus_widget(401));
    assert!(duplicate.wheel_or_scroll_at_with_sample(
        Point::new(20.0, 20.0),
        runtime_wheel_sample(delta, WheelPhase::Started),
    ));
    duplicate.bridge_mut().duplicate_owner = true;
    duplicate.refresh();
    assert!(!duplicate.wheel_or_scroll_at_with_sample(
        Point::new(20.0, 60.0),
        runtime_wheel_sample(delta, WheelPhase::Changed),
    ));
    assert!(!duplicate.wheel_or_scroll_at_with_sample(
        Point::new(20.0, 60.0),
        runtime_wheel_sample(delta, WheelPhase::Changed),
    ));
    assert_eq!(
        duplicate.bridge().observations.borrow().as_slice(),
        [runtime_wheel_observation(
            401,
            delta,
            WheelPhase::Started,
            false,
        )]
    );

    let mut disabled_replacement =
        SurfaceRuntime::new(RuntimeWheelBridge::default(), Vector2::new(160.0, 80.0));
    assert!(disabled_replacement.focus_widget(401));
    assert!(disabled_replacement.wheel_or_scroll_at_with_sample(
        Point::new(20.0, 20.0),
        runtime_wheel_sample(delta, WheelPhase::Started),
    ));
    disabled_replacement.bridge_mut().replace_owner = true;
    disabled_replacement.bridge_mut().disabled = true;
    disabled_replacement.refresh();
    assert!(!disabled_replacement.wheel_or_scroll_at_with_sample(
        Point::new(20.0, 60.0),
        runtime_wheel_sample(delta, WheelPhase::Changed),
    ));
    assert_eq!(
        disabled_replacement
            .bridge()
            .observations
            .borrow()
            .as_slice(),
        [runtime_wheel_observation(
            401,
            delta,
            WheelPhase::Started,
            false,
        )]
    );
}
