use super::super::frame_scheduler_policy::{FrameStageBudgetStatus, ImmediateTransientCompletion};
use super::super::lifecycle_pointer::finalize_native_immediate_transient_route;
use super::*;
use crate::application::{ApplicationEnvironment, IntoView, LocaleId, TextScale};
use crate::gui::{
    focus::FocusSurface,
    input::{InputSequenceRange, InputTimestamp, KeyCode, KeyPress},
    pointer_ingress::{DeviceKind, PointerEvent, PointerIngressDisposition, PointerPhase},
    shortcuts::ShortcutResolution,
};
use crate::runtime::{ExternalDragRequest, RuntimeHostCapabilities, RuntimeInputHost};
use crate::{
    gui_runtime::native_vello::CaretAffinity,
    layout::LayoutOutput,
    theme::ThemeTokens,
    widgets::{
        EditPhase, KeyboardModifier, NumericAdjustment, NumericCodec, NumericInputInteraction,
        NumericInputInteractionBatch, NumericParseResult, NumericStep, NumericStepDirection,
        NumericStepModifiers, PointerModifiers, WheelDelta, WheelPhase, WheelSample, Widget,
        WidgetCommon, WidgetInput, WidgetKey, WidgetOutput, WidgetSizing,
    },
};
use std::{
    cell::Cell,
    convert::Infallible,
    fmt,
    time::{Duration, Instant},
};
use winit::{
    dpi::PhysicalPosition,
    event::{DeviceId, ElementState, MouseButton, MouseScrollDelta, TouchPhase},
    keyboard::{KeyCode as WinitKeyCode, ModifiersState, PhysicalKey},
};

const NATIVE_NUMERIC_ID: u64 = 1386;
const NATIVE_NUMERIC_SOURCE: &str = "iiiiWאב";

#[derive(Clone, Copy, Debug, Default)]
struct NativeNumericCodec;

impl NumericCodec<String> for NativeNumericCodec {
    type Error = fmt::Error;

    fn parse(&self, text: &str) -> NumericParseResult<String> {
        if text.is_empty() {
            NumericParseResult::Incomplete
        } else {
            NumericParseResult::Valid(text.to_owned())
        }
    }

    fn format_editable(
        &self,
        value: &String,
        output: &mut dyn fmt::Write,
    ) -> Result<(), Self::Error> {
        output.write_str(value)
    }
}

#[derive(Clone, Copy, Debug, Default)]
struct NativeNumericAdjustment;

impl NumericAdjustment<String> for NativeNumericAdjustment {
    type Error = Infallible;

    fn normalized_to_value(&self, _normalized: f32) -> Result<String, Self::Error> {
        Ok(String::new())
    }

    fn value_to_normalized(&self, _value: &String) -> Result<f32, Self::Error> {
        Ok(0.5)
    }

    fn step(
        &self,
        value: &String,
        _direction: NumericStepDirection,
        _step: NumericStep,
    ) -> Result<String, Self::Error> {
        Ok(format!("{value}!"))
    }

    fn scrub(
        &self,
        value: &String,
        _normalized_delta: f32,
        _step: NumericStep,
    ) -> Result<String, Self::Error> {
        Ok(value.clone())
    }

    fn wheel(
        &self,
        value: &String,
        _delta: f32,
        _step: NumericStep,
    ) -> Result<String, Self::Error> {
        Ok(value.clone())
    }
}

enum NativeNumericMessage {
    Interaction(NumericInputInteractionBatch<String, Infallible, fmt::Error>),
}

struct NativeNumericBridge {
    value: String,
    environment: ApplicationEnvironment,
    interactions: Vec<Vec<(EditPhase, String)>>,
}

impl Default for NativeNumericBridge {
    fn default() -> Self {
        Self {
            value: String::from(NATIVE_NUMERIC_SOURCE),
            environment: ApplicationEnvironment::new(LocaleId::english())
                .with_text_scale(TextScale::new(1.5).expect("valid native numeric text scale")),
            interactions: Vec::new(),
        }
    }
}

impl RuntimeBridge<NativeNumericMessage> for NativeNumericBridge {
    fn application_environment(&mut self) -> Option<ApplicationEnvironment> {
        Some(self.environment.clone())
    }

    fn project_surface(&mut self) -> Arc<UiSurface<NativeNumericMessage>> {
        crate::application::numeric_input(
            self.value.clone(),
            NativeNumericCodec,
            NativeNumericAdjustment,
        )
        .expect("native numeric fixture should construct")
        .step_modifiers(NumericStepModifiers::new(
            KeyboardModifier::Shift,
            KeyboardModifier::Command,
        ))
        .on_interaction(NativeNumericMessage::Interaction)
        .id(NATIVE_NUMERIC_ID)
        .into_projection()
        .into_surface()
        .into()
    }

    fn reduce_message(&mut self, message: NativeNumericMessage) {
        let NativeNumericMessage::Interaction(batch) = message;
        for part in batch.parts() {
            if let NumericInputInteraction::Edit(edit) = part {
                self.interactions.push(
                    edit.events()
                        .iter()
                        .map(|event| (event.phase, event.value.clone()))
                        .collect(),
                );
            }
        }
    }
}

fn native_numeric_paint_input(
    runner: &GenericNativeVelloRunner<NativeNumericBridge, NativeNumericMessage>,
) -> crate::runtime::PaintTextInput {
    runner
        .frame
        .last_paint_plan
        .primitives
        .iter()
        .find_map(|primitive| match primitive {
            PaintPrimitive::TextInput(input) => Some(input.clone()),
            _ => None,
        })
        .expect("numeric input should emit a text-input paint primitive")
}

fn focus_native_numeric_with_tab(
    harness: &mut NativePointerHarness<NativeNumericBridge, NativeNumericMessage>,
) {
    let outcome = harness
        .runner
        .route_native_tab_for_test(false)
        .expect("native Tab focus should produce a route outcome");
    assert!(outcome.routed, "native Tab should focus Numeric");
    harness.runner.apply_route_outcome(outcome);
    assert_eq!(
        harness.runner.core.runtime.focused_widget(),
        Some(crate::widgets::WidgetId::from(NATIVE_NUMERIC_ID))
    );

    let release = harness
        .runner
        .route_native_key_release(PhysicalKey::Code(WinitKeyCode::Tab))
        .expect("native Tab release should produce a route outcome");
    assert!(release.routed, "native Tab release should be routed");
    harness.runner.apply_route_outcome(release);
}

fn generic_numeric_caret_at(position: Point) -> usize {
    let mut fallback = GenericNativeVelloRunner::new(
        NativeRunOptions::default(),
        NativeNumericBridge::default(),
        Vector2::new(260.0, 60.0),
    );
    fallback.rebuild_scene();
    let target = fallback
        .core
        .runtime
        .dispatch_event(crate::runtime::Event::PointerPress {
            position,
            button: crate::widgets::PointerButton::Primary,
            modifiers: PointerModifiers::default(),
            timestamp: None,
        });
    assert_eq!(
        target,
        Some(crate::widgets::WidgetId::from(NATIVE_NUMERIC_ID))
    );
    fallback.rebuild_scene();
    native_numeric_paint_input(&fallback).state.caret
}

fn generic_numeric_caret_at_value(position: Point, value: &str) -> usize {
    let bridge = NativeNumericBridge {
        value: value.to_owned(),
        ..NativeNumericBridge::default()
    };
    let mut fallback = GenericNativeVelloRunner::new(
        NativeRunOptions::default(),
        bridge,
        Vector2::new(260.0, 60.0),
    );
    fallback.rebuild_scene();
    let target = fallback
        .core
        .runtime
        .dispatch_event(crate::runtime::Event::PointerPress {
            position,
            button: crate::widgets::PointerButton::Primary,
            modifiers: PointerModifiers::default(),
            timestamp: None,
        });
    assert_eq!(
        target,
        Some(crate::widgets::WidgetId::from(NATIVE_NUMERIC_ID))
    );
    fallback.rebuild_scene();
    native_numeric_paint_input(&fallback).state.caret
}

fn native_numeric_divergence(
    runner: &mut GenericNativeVelloRunner<NativeNumericBridge, NativeNumericMessage>,
) -> (Point, usize, CaretAffinity, usize) {
    let input = native_numeric_paint_input(runner);
    (0..=(input.rect.width() * 2.0).ceil() as usize)
        .map(|offset| {
            Point::new(
                input.rect.min.x + offset as f32 * 0.5,
                input.rect.center().y,
            )
        })
        .find_map(|position| {
            let (_, source, caret, affinity) =
                runner.frame.native_text_pointer_target(position, None)?;
            if source != NATIVE_NUMERIC_SOURCE || affinity != CaretAffinity::Upstream {
                return None;
            }
            let fallback_caret = generic_numeric_caret_at(position);
            (caret != fallback_caret && affinity == CaretAffinity::Upstream).then_some((
                position,
                caret,
                affinity,
                fallback_caret,
            ))
        })
        .expect("shaped native caret and generic fallback should diverge")
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ModifierWheelMessage {
    Wheel {
        modifiers: PointerModifiers,
        timestamp: Option<InputTimestamp>,
        sequence_range: Option<InputSequenceRange>,
    },
}

#[derive(Clone, Debug)]
struct ModifierWheelWidget {
    common: WidgetCommon,
}

impl ModifierWheelWidget {
    fn new(id: u64) -> Self {
        Self {
            common: WidgetCommon::new(id, WidgetSizing::fixed(Vector2::new(120.0, 80.0))),
        }
    }
}

impl crate::widgets::WidgetHitTest for ModifierWheelWidget {
    fn revision(&self) -> crate::widgets::WidgetHitTestRevision {
        crate::widgets::WidgetHitTestRevision::exact(())
    }

    fn hit_test(
        &self,
        _bounds: crate::layout::Rect,
        _point: crate::gui::types::Point,
        input: &WidgetInput,
    ) -> crate::widgets::WidgetHitTestResult {
        matches!(
            input,
            WidgetInput::Wheel {
                modifiers,
                timestamp: None,
                ..
            } if modifiers.shift
        )
        .then_some(crate::widgets::WidgetHitTestResult::Opaque)
        .unwrap_or(crate::widgets::WidgetHitTestResult::PassThrough)
    }
}

impl Widget for ModifierWheelWidget {
    fn common(&self) -> &WidgetCommon {
        &self.common
    }

    fn common_mut(&mut self) -> &mut WidgetCommon {
        &mut self.common
    }

    fn handle_input(&mut self, _bounds: Rect, input: WidgetInput) -> Option<WidgetOutput> {
        match input {
            WidgetInput::Wheel {
                modifiers,
                timestamp,
                sequence_range,
                ..
            } => Some(WidgetOutput::typed(ModifierWheelMessage::Wheel {
                modifiers,
                timestamp,
                sequence_range,
            })),
            _ => None,
        }
    }

    fn capabilities(&self) -> crate::widgets::WidgetCapabilities<'_> {
        crate::widgets::WidgetCapabilities::none()
    }

    fn capabilities_v2(&self) -> crate::widgets::WidgetCapabilitiesV2<'_> {
        crate::widgets::WidgetCapabilitiesV2::new().with_hit_test(self)
    }

    fn accepts_wheel_input(&self) -> bool {
        true
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
struct ModifierWheelBridge {
    samples: Vec<(
        PointerModifiers,
        Option<InputTimestamp>,
        Option<InputSequenceRange>,
    )>,
}

impl RuntimeBridge<ModifierWheelMessage> for ModifierWheelBridge {
    fn project_surface(&mut self) -> Arc<UiSurface<ModifierWheelMessage>> {
        let rows = (0..20)
            .map(|index| {
                crate::application::text(format!("Row {index}"))
                    .height(20.0)
                    .fill_width()
            })
            .collect();
        crate::application::overlay_stack(crate::application::bounded_scroll_column(
            rows, 4, 20.0, 0.0,
        ))
        .input(crate::application::custom_widget_direct(
            ModifierWheelWidget::new(4),
        ))
        .into_view()
        .into_surface()
        .into()
    }

    fn reduce_message(&mut self, message: ModifierWheelMessage) {
        let ModifierWheelMessage::Wheel {
            modifiers,
            timestamp,
            sequence_range,
        } = message;
        self.samples.push((modifiers, timestamp, sequence_range));
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct ManagedWheelMessage {
    delta: WheelDelta,
    phase: Option<WheelPhase>,
    modifiers: PointerModifiers,
    timestamp: Option<InputTimestamp>,
    sequence_range: Option<InputSequenceRange>,
}

#[derive(Clone, Debug)]
struct ManagedWheelWidget {
    common: WidgetCommon,
}

impl ManagedWheelWidget {
    fn new(id: u64) -> Self {
        Self {
            common: WidgetCommon::new(id, WidgetSizing::fixed(Vector2::new(120.0, 40.0))),
        }
    }
}

impl Widget for ManagedWheelWidget {
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
        Some(WidgetOutput::typed(ManagedWheelMessage {
            delta: sample.delta(),
            phase: sample.phase(),
            modifiers: sample.modifiers(),
            timestamp: sample.timestamp(),
            sequence_range: sample.sequence_range(),
        }))
    }

    fn accepts_wheel_input(&self) -> bool {
        true
    }

    fn retains_managed_wheel_sequence(&self) -> bool {
        true
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
struct ManagedWheelBridge {
    samples: Vec<ManagedWheelMessage>,
}

impl RuntimeBridge<ManagedWheelMessage> for ManagedWheelBridge {
    fn project_surface(&mut self) -> Arc<UiSurface<ManagedWheelMessage>> {
        let rows = (0..20)
            .map(|index| {
                crate::application::text(format!("Row {index}"))
                    .height(20.0)
                    .fill_width()
            })
            .collect();
        crate::application::overlay_stack(crate::application::bounded_scroll_column(
            rows, 4, 20.0, 0.0,
        ))
        .input(crate::application::custom_widget_direct(
            ManagedWheelWidget::new(4),
        ))
        .into_view()
        .into_surface()
        .into()
    }

    fn reduce_message(&mut self, message: ManagedWheelMessage) {
        self.samples.push(message);
    }
}

#[derive(Default)]
struct PressTimestampBridge {
    timestamps: Vec<Option<InputTimestamp>>,
}

#[derive(Clone)]
struct PressTimestampWidget {
    common: WidgetCommon,
}

impl PressTimestampWidget {
    fn new(id: u64) -> Self {
        Self {
            common: WidgetCommon::new(id, WidgetSizing::fixed(Vector2::new(120.0, 40.0))),
        }
    }
}

impl Widget for PressTimestampWidget {
    fn common(&self) -> &WidgetCommon {
        &self.common
    }

    fn common_mut(&mut self) -> &mut WidgetCommon {
        &mut self.common
    }

    fn handle_input(&mut self, bounds: Rect, input: WidgetInput) -> Option<WidgetOutput> {
        match input {
            WidgetInput::PointerPress {
                position,
                timestamp,
                ..
            }
            | WidgetInput::PointerDoubleClick {
                position,
                timestamp,
                ..
            }
            | WidgetInput::PointerRelease {
                position,
                timestamp,
                ..
            } if bounds.contains(position) => Some(WidgetOutput::typed(timestamp)),
            WidgetInput::PointerModifiersChanged { timestamp, .. } => {
                Some(WidgetOutput::typed(timestamp))
            }
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

impl RuntimeBridge<Option<InputTimestamp>> for PressTimestampBridge {
    fn project_surface(&mut self) -> Arc<UiSurface<Option<InputTimestamp>>> {
        crate::runtime::test_arc_surface(UiSurface::new(SurfaceNode::widget(
            PressTimestampWidget::new(1),
            WidgetMessageMapper::typed(|timestamp: Option<InputTimestamp>| timestamp),
        )))
    }

    fn reduce_message(&mut self, timestamp: Option<InputTimestamp>) {
        self.timestamps.push(timestamp);
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct NativeMoveSample {
    position: Point,
    modifiers: PointerModifiers,
    timestamp: Option<InputTimestamp>,
    sequence_range: Option<InputSequenceRange>,
}

#[derive(Default)]
struct NativeMoveMetadataBridge {
    samples: Vec<NativeMoveSample>,
}

#[derive(Default)]
struct NativeTypedPointerBridge {
    events: Vec<PointerEvent>,
}

impl RuntimeBridge<PointerEvent> for NativeTypedPointerBridge {
    fn project_surface(&mut self) -> Arc<UiSurface<PointerEvent>> {
        crate::runtime::test_arc_surface(UiSurface::new(SurfaceNode::widget(
            crate::widgets::CanvasWidget::new(1, WidgetSizing::fixed(Vector2::new(120.0, 40.0))),
            WidgetMessageMapper::canvas_pointer(|event| event),
        )))
    }

    fn reduce_message(&mut self, event: PointerEvent) {
        self.events.push(event);
    }
}

#[derive(Clone)]
struct NativeMoveMetadataWidget {
    common: WidgetCommon,
}

impl Widget for NativeMoveMetadataWidget {
    fn common(&self) -> &WidgetCommon {
        &self.common
    }

    fn common_mut(&mut self) -> &mut WidgetCommon {
        &mut self.common
    }

    fn handle_input(&mut self, bounds: Rect, input: WidgetInput) -> Option<WidgetOutput> {
        match input {
            WidgetInput::PointerMove {
                position,
                modifiers,
                timestamp,
                sequence_range,
                ..
            } if bounds.contains(position) => Some(WidgetOutput::typed(NativeMoveSample {
                position,
                modifiers,
                timestamp,
                sequence_range,
            })),
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

impl RuntimeBridge<NativeMoveSample> for NativeMoveMetadataBridge {
    fn project_surface(&mut self) -> Arc<UiSurface<NativeMoveSample>> {
        crate::runtime::test_arc_surface(UiSurface::new(SurfaceNode::widget(
            NativeMoveMetadataWidget {
                common: WidgetCommon::new(2, WidgetSizing::fixed(Vector2::new(120.0, 40.0))),
            },
            WidgetMessageMapper::typed(|sample: NativeMoveSample| sample),
        )))
    }

    fn reduce_message(&mut self, sample: NativeMoveSample) {
        self.samples.push(sample);
    }
}

#[test]
fn scroll_coalescing_preserves_modifier_sensitive_wheel_ownership() {
    let mut runner = GenericNativeVelloRunner::new(
        NativeRunOptions::default(),
        ModifierWheelBridge::default(),
        Vector2::new(120.0, 80.0),
    );
    runner.rebuild_scene();
    let point = Point::new(40.0, 20.0);
    let delta = Vector2::new(0.0, -40.0);

    assert!(runner.can_coalesce_scroll_container_wheel(point, delta, PointerModifiers::default()));
    assert!(!runner.can_coalesce_scroll_container_wheel(
        point,
        delta,
        PointerModifiers {
            shift: true,
            ..PointerModifiers::default()
        }
    ));
    assert!(!runner.can_coalesce_scroll_container_wheel_with_timestamp(
        point,
        delta,
        PointerModifiers {
            shift: true,
            ..PointerModifiers::default()
        },
        Some(InputTimestamp::capture()),
    ));
}

fn assert_phaseful_scroll_container_fallback(raw_delta: MouseScrollDelta, expected_delta: Vector2) {
    let mut runner = GenericNativeVelloRunner::new(
        NativeRunOptions::default(),
        GpuWheelScrollBridge::default(),
        Vector2::new(120.0, 40.0),
    );
    runner.rebuild_scene();
    runner.input.last_cursor = Some(Point::new(40.0, 20.0));
    runner.timing.redraw_requested = false;
    runner.timing.redraw_requested_at = None;

    // Moved is the native Changed phase. Ordinary scroll fallback coalesces
    // it into one logical-pixel, single-axis, phase-less ScrollUpdate.
    let route = runner.route_native_mouse_wheel_with_phase(raw_delta, TouchPhase::Moved);
    assert_eq!(route.diagnostic.result, NativePointerRouteResult::Coalesced);
    let pending = runner
        .input
        .pending_scroll_container_wheel
        .expect("ordinary scroll fallback should be pending");
    assert_eq!(pending.delta, expected_delta);
    assert!(pending.delta.x == 0.0 || pending.delta.y == 0.0);
    assert!(pending.timestamp.is_some());
    assert!(pending.sequence_range.is_some());

    runner.flush_pending_scroll_container_wheel(&mut RenderFrameProfile::default());

    let bridge = runner.core.runtime.bridge();
    assert_eq!(bridge.scroll_updates.len(), 1);
    let update = bridge.scroll_updates[0];
    assert_eq!(update.delta, expected_delta);
    assert!(update.metadata.timestamp.is_some());
    assert!(update.metadata.sequence_range.is_some());
}

#[test]
fn native_changed_scroll_container_vertical_dominant_diagonal_drops_horizontal() {
    assert_phaseful_scroll_container_fallback(
        MouseScrollDelta::LineDelta(1.0, -2.0),
        Vector2::new(0.0, 80.0),
    );
}

#[test]
fn native_changed_scroll_container_horizontal_dominant_diagonal_drops_vertical() {
    assert_phaseful_scroll_container_fallback(
        MouseScrollDelta::PixelDelta(PhysicalPosition::new(-30.0, 10.0)),
        Vector2::new(30.0, 0.0),
    );
}

#[test]
fn native_changed_scroll_container_tied_diagonal_selects_vertical() {
    assert_phaseful_scroll_container_fallback(
        MouseScrollDelta::PixelDelta(PhysicalPosition::new(20.0, -20.0)),
        Vector2::new(0.0, 20.0),
    );
}

#[test]
fn native_phaseful_scroll_container_burst_coalesces_delta_and_metadata() {
    let mut runner = GenericNativeVelloRunner::new(
        NativeRunOptions::default(),
        GpuWheelScrollBridge::default(),
        Vector2::new(240.0, 40.0),
    );
    runner.rebuild_scene();
    let first_position = Point::new(40.0, 20.0);
    let newest_position = Point::new(80.0, 24.0);
    runner.input.last_cursor = Some(first_position);
    runner.timing.redraw_requested = false;
    runner.timing.redraw_requested_at = None;
    assert!(!runner.timing.redraw_requested);

    let first = runner.route_native_mouse_wheel_with_phase(
        MouseScrollDelta::LineDelta(1.0, -2.0),
        TouchPhase::Moved,
    );
    assert_eq!(first.diagnostic.result, NativePointerRouteResult::Coalesced);
    let first_pending = runner
        .input
        .pending_scroll_container_wheel
        .expect("phaseful ordinary scroll should be pending");
    assert_eq!(first_pending.position, first_position);
    assert_eq!(first_pending.delta, Vector2::new(0.0, 80.0));
    assert_eq!(first_pending.modifiers, PointerModifiers::default());
    let first_sequence = first_pending
        .sequence_range
        .expect("first phaseful sample should carry sequence metadata");
    let first_timestamp = first_pending
        .timestamp
        .expect("first phaseful sample should carry timestamp metadata");
    assert!(runner.core.runtime.bridge().scroll_updates.is_empty());

    runner.input.last_cursor = Some(newest_position);
    runner.input.modifiers = ModifiersState::SHIFT;
    let second = runner.route_native_mouse_wheel_with_phase(
        MouseScrollDelta::PixelDelta(PhysicalPosition::new(-5.0, -10.0)),
        TouchPhase::Moved,
    );
    assert_eq!(
        second.diagnostic.result,
        NativePointerRouteResult::Coalesced
    );
    assert!(runner.core.runtime.bridge().scroll_updates.is_empty());

    let pending = runner
        .input
        .pending_scroll_container_wheel
        .expect("phaseful ordinary burst should remain coalesced");
    assert_eq!(pending.position, newest_position);
    assert_eq!(pending.delta, Vector2::new(0.0, 90.0));
    assert_eq!(
        pending.modifiers,
        PointerModifiers {
            shift: true,
            ..Default::default()
        }
    );
    let newest_timestamp = pending
        .timestamp
        .expect("newest phaseful sample should carry timestamp metadata");
    assert!(newest_timestamp >= first_timestamp);
    let pending_sequence = pending
        .sequence_range
        .expect("coalesced phaseful samples should retain sequence metadata");
    assert_eq!(pending_sequence.start(), first_sequence.start());
    assert_eq!(
        pending_sequence.end().runtime_value(),
        first_sequence.end().runtime_value() + 1
    );
    assert_eq!(runner.core.runtime.bridge().scroll_count, 0);

    runner.flush_pending_scroll_container_wheel(&mut RenderFrameProfile::default());

    assert!(runner.input.pending_scroll_container_wheel.is_none());
    assert_eq!(runner.core.runtime.bridge().scroll_count, 1);
    let update = runner.core.runtime.bridge().scroll_updates[0];
    assert_eq!(update.position, newest_position);
    assert_eq!(update.delta, Vector2::new(0.0, 90.0));
    assert_eq!(update.metadata.modifiers, pending.modifiers);
    assert_eq!(update.metadata.timestamp, Some(newest_timestamp));
    assert_eq!(update.metadata.sequence_range, Some(pending_sequence));
}

#[test]
fn native_phaseful_scroll_container_flushes_before_switching_axis() {
    let mut runner = GenericNativeVelloRunner::new(
        NativeRunOptions::default(),
        GpuWheelScrollBridge::default(),
        Vector2::new(120.0, 40.0),
    );
    runner.rebuild_scene();
    let point = Point::new(40.0, 20.0);
    runner.input.last_cursor = Some(point);
    runner.timing.redraw_requested = true;
    runner.timing.redraw_requested_at = Some(Instant::now());

    let vertical = runner.route_native_mouse_wheel_with_phase(
        MouseScrollDelta::LineDelta(0.0, -1.0),
        TouchPhase::Moved,
    );
    assert_eq!(
        vertical.diagnostic.result,
        NativePointerRouteResult::Coalesced
    );
    let first_pending = runner
        .input
        .pending_scroll_container_wheel
        .expect("vertical phaseful scroll should be pending");
    let first_timestamp = first_pending
        .timestamp
        .expect("vertical phaseful scroll should carry timestamp metadata");
    let first_sequence = first_pending
        .sequence_range
        .expect("vertical phaseful scroll should carry sequence metadata");

    runner.input.modifiers = ModifiersState::SHIFT;
    let horizontal = runner.route_native_mouse_wheel_with_phase(
        MouseScrollDelta::LineDelta(-1.0, 0.0),
        TouchPhase::Moved,
    );
    assert_eq!(
        horizontal.diagnostic.result,
        NativePointerRouteResult::Coalesced
    );

    let bridge = runner.core.runtime.bridge();
    assert_eq!(bridge.scroll_updates.len(), 1);
    assert_eq!(bridge.scroll_updates[0].delta, Vector2::new(0.0, 40.0));
    assert_eq!(
        bridge.scroll_updates[0].metadata.modifiers,
        PointerModifiers::default()
    );
    assert_eq!(
        bridge.scroll_updates[0].metadata.timestamp,
        Some(first_timestamp)
    );
    assert_eq!(
        bridge.scroll_updates[0].metadata.sequence_range,
        Some(first_sequence)
    );

    let pending = runner
        .input
        .pending_scroll_container_wheel
        .expect("horizontal phaseful scroll should become the sole pending item");
    assert_eq!(pending.delta, Vector2::new(40.0, 0.0));
    assert_eq!(
        pending.modifiers,
        PointerModifiers {
            shift: true,
            ..Default::default()
        }
    );
    let second_timestamp = pending
        .timestamp
        .expect("horizontal phaseful scroll should carry timestamp metadata");
    assert!(second_timestamp >= first_timestamp);
    let second_sequence = pending
        .sequence_range
        .expect("horizontal phaseful scroll should carry sequence metadata");
    assert_ne!(second_sequence.start(), first_sequence.start());
    assert_ne!(second_sequence.end(), first_sequence.end());

    runner.flush_pending_scroll_container_wheel(&mut RenderFrameProfile::default());

    let bridge = runner.core.runtime.bridge();
    assert_eq!(bridge.scroll_updates.len(), 2);
    assert_eq!(bridge.scroll_updates[1].delta, Vector2::new(40.0, 0.0));
    assert_eq!(
        bridge.scroll_updates[1].metadata.modifiers,
        PointerModifiers {
            shift: true,
            ..Default::default()
        }
    );
    assert_eq!(
        bridge.scroll_updates[1].metadata.timestamp,
        Some(second_timestamp)
    );
    assert_eq!(
        bridge.scroll_updates[1].metadata.sequence_range,
        Some(second_sequence)
    );
    assert!(runner.input.pending_scroll_container_wheel.is_none());
}

#[test]
fn native_phaseful_scroll_retains_exact_sequence_after_pending_compatibility_flush() {
    let mut runner = GenericNativeVelloRunner::new(
        NativeRunOptions::default(),
        GpuWheelScrollBridge::default(),
        Vector2::new(240.0, 40.0),
    );
    runner.rebuild_scene();
    let point = Point::new(40.0, 20.0);
    runner.input.last_cursor = Some(point);
    runner.timing.redraw_requested = true;
    runner.timing.redraw_requested_at = Some(Instant::now());

    let moved = runner.route_native_mouse_wheel_with_phase(
        MouseScrollDelta::LineDelta(0.0, -0.25),
        TouchPhase::Moved,
    );
    assert_eq!(moved.diagnostic.result, NativePointerRouteResult::Coalesced);
    assert_eq!(runner.core.runtime.bridge().scroll_count, 0);

    let started = runner.route_native_mouse_wheel_with_phase(
        MouseScrollDelta::LineDelta(0.0, -0.25),
        TouchPhase::Started,
    );
    assert_eq!(started.diagnostic.result, NativePointerRouteResult::Routed);
    assert!(runner.input.pending_scroll_container_wheel.is_none());
    assert_eq!(runner.core.runtime.bridge().scroll_count, 2);

    runner.timing.redraw_requested = true;
    runner.timing.redraw_requested_at = Some(Instant::now());
    let moved = runner.route_native_mouse_wheel_with_phase(
        MouseScrollDelta::LineDelta(0.0, 0.25),
        TouchPhase::Moved,
    );
    assert_eq!(moved.diagnostic.result, NativePointerRouteResult::Routed);

    let ended = runner.route_native_mouse_wheel_with_phase(
        MouseScrollDelta::LineDelta(0.0, 0.25),
        TouchPhase::Ended,
    );
    assert_eq!(ended.diagnostic.result, NativePointerRouteResult::Routed);
    assert!(runner.input.pending_scroll_container_wheel.is_none());
    assert_eq!(runner.core.runtime.bridge().scroll_count, 4);

    runner.timing.redraw_requested = true;
    runner.timing.redraw_requested_at = Some(Instant::now());
    let moved = runner.route_native_mouse_wheel_with_phase(
        MouseScrollDelta::LineDelta(0.0, -0.25),
        TouchPhase::Moved,
    );
    assert_eq!(moved.diagnostic.result, NativePointerRouteResult::Unrouted);

    let cancelled = runner.route_native_mouse_wheel_with_phase(
        MouseScrollDelta::LineDelta(0.0, -0.25),
        TouchPhase::Cancelled,
    );
    assert_eq!(
        cancelled.diagnostic.result,
        NativePointerRouteResult::Unrouted
    );
    assert!(runner.input.pending_scroll_container_wheel.is_none());
    assert_eq!(runner.core.runtime.bridge().scroll_count, 4);
    // A fresh admitted sequence can start after the stale terminal.
    runner.route_native_mouse_wheel_with_phase(
        MouseScrollDelta::LineDelta(0.0, -0.25),
        TouchPhase::Started,
    );
    runner.route_native_mouse_wheel_with_phase(
        MouseScrollDelta::LineDelta(0.0, -0.25),
        TouchPhase::Moved,
    );
    let cancelled = runner.route_native_mouse_wheel_with_phase(
        MouseScrollDelta::LineDelta(0.0, -0.25),
        TouchPhase::Cancelled,
    );
    assert_eq!(
        cancelled.diagnostic.result,
        NativePointerRouteResult::Routed
    );
    assert_eq!(runner.core.runtime.bridge().scroll_count, 7);
}

#[derive(Default)]
struct PointerSnapshotShortcutBridge {
    snapshots: Vec<Option<Point>>,
}

impl RuntimeBridge<()> for PointerSnapshotShortcutBridge {
    fn project_surface(&mut self) -> Arc<UiSurface<()>> {
        crate::runtime::test_arc_surface(UiSurface::new(SurfaceNode::container(
            1,
            ContainerPolicy::default(),
            Vec::new(),
        )))
    }

    fn update_with_runtime(
        &mut self,
        _message: (),
        snapshot: crate::runtime::RuntimeUpdateSnapshot,
    ) -> Command<()> {
        self.snapshots.push(snapshot.current_pointer_position());
        Command::none()
    }

    fn host_capabilities(&self) -> RuntimeHostCapabilities<Self, ()> {
        RuntimeHostCapabilities::new().with_input()
    }
}

impl RuntimeInputHost<()> for PointerSnapshotShortcutBridge {
    fn resolve_key_press(
        &mut self,
        _pending_chord: Option<KeyPress>,
        press: KeyPress,
        _focus: FocusSurface,
    ) -> ShortcutResolution<()> {
        if press.key == KeyCode::W {
            ShortcutResolution::action(())
        } else {
            ShortcutResolution::unhandled()
        }
    }
}

#[derive(Default)]
struct FocusRegainedBridge {
    focus_regained_calls: usize,
    reduced_messages: usize,
}

impl RuntimeBridge<()> for FocusRegainedBridge {
    fn project_surface(&mut self) -> Arc<UiSurface<()>> {
        crate::runtime::test_arc_surface(UiSurface::new(SurfaceNode::container(
            1,
            ContainerPolicy::default(),
            Vec::new(),
        )))
    }

    fn reduce_message(&mut self, (): ()) {
        self.reduced_messages += 1;
    }

    fn host_capabilities(&self) -> RuntimeHostCapabilities<Self, ()> {
        RuntimeHostCapabilities::new().with_input()
    }
}

impl RuntimeInputHost<()> for FocusRegainedBridge {
    fn native_focus_regained(&mut self) -> Command<()> {
        self.focus_regained_calls += 1;
        Command::message(())
    }
}

#[test]
fn native_focus_regained_notifies_host_and_routes_its_command() {
    let mut harness =
        NativePointerHarness::new(FocusRegainedBridge::default(), Vector2::new(320.0, 40.0));

    harness.focus_lost();
    harness.focus_regained();

    let bridge = harness.runner.core.runtime.bridge();
    assert_eq!(bridge.focus_regained_calls, 1);
    assert_eq!(bridge.reduced_messages, 1);
}

#[test]
fn initial_native_focus_does_not_report_focus_regained() {
    let mut harness =
        NativePointerHarness::new(FocusRegainedBridge::default(), Vector2::new(320.0, 40.0));

    harness.focus_regained();

    let bridge = harness.runner.core.runtime.bridge();
    assert_eq!(bridge.focus_regained_calls, 0);
    assert_eq!(bridge.reduced_messages, 0);
}

#[test]
fn native_pointer_harness_routes_cursor_and_mouse_to_runner_state() {
    let mut harness = NativePointerHarness::new(demo_bridge(), Vector2::new(320.0, 40.0));
    let button_point = harness
        .runner
        .core
        .runtime
        .layout()
        .rects
        .get(&11)
        .map(|rect| Point::new(rect.min.x + 4.0, rect.min.y + 4.0))
        .expect("button should be laid out");

    harness.cursor_moved_logical(button_point);
    assert_eq!(harness.runner.input.last_cursor, Some(button_point));
    assert!(harness.mouse_pressed(MouseButton::Left).routed);
    assert!(harness.mouse_released(MouseButton::Left).routed);

    assert_eq!(harness.runner.core.runtime.bridge().state.count, 1);
}

#[test]
fn native_text_pointer_affinity_commits_only_for_current_target_and_text_source() {
    let mut runner = GenericNativeVelloRunner::new(
        NativeRunOptions::default(),
        demo_bridge(),
        Vector2::new(320.0, 40.0),
    );
    runner.rebuild_scene();
    runner
        .frame
        .seed_text_input_snapshots_for_current_plan(false, &Default::default());
    let input_rect = runner
        .core
        .runtime
        .layout()
        .rects
        .get(&12)
        .copied()
        .expect("text input should be laid out");
    let position = Point::new(input_rect.min.x + 4.0, input_rect.min.y + 4.0);

    runner.frame.text_renderer.set_native_caret_affinity(
        crate::widgets::WidgetId::from(12_u32),
        CaretAffinity::Upstream,
    );
    runner.core.runtime.set_native_text_pointer_caret(
        crate::widgets::WidgetId::from(12_u32),
        "stale",
        0,
        crate::widgets::NativeCaretAffinity::Downstream,
    );
    runner.core.runtime.dispatch_input(
        crate::widgets::WidgetId::from(12_u32),
        WidgetInput::primary_press(position),
    );
    runner.commit_accepted_native_text_pointer_caret();
    assert_eq!(
        runner
            .frame
            .text_renderer
            .native_caret_affinity(crate::widgets::WidgetId::from(12_u32)),
        CaretAffinity::Upstream
    );

    let source = String::new();
    let caret = 0;
    let affinity = CaretAffinity::Downstream;
    runner.core.runtime.set_native_text_pointer_caret(
        crate::widgets::WidgetId::from(12_u32),
        &source,
        caret,
        match affinity {
            CaretAffinity::Upstream => crate::widgets::NativeCaretAffinity::Upstream,
            CaretAffinity::Downstream => crate::widgets::NativeCaretAffinity::Downstream,
        },
    );
    runner.core.runtime.dispatch_input(
        crate::widgets::WidgetId::from(11_u32),
        WidgetInput::pointer_move(position),
    );
    runner.core.runtime.dispatch_input(
        crate::widgets::WidgetId::from(12_u32),
        WidgetInput::primary_press(position),
    );
    runner.commit_accepted_native_text_pointer_caret();
    assert_eq!(
        runner
            .frame
            .text_renderer
            .native_caret_affinity(crate::widgets::WidgetId::from(12_u32)),
        CaretAffinity::Upstream,
        "a caret staged for a text input must be vetoed when another widget consumes the event"
    );

    runner.core.runtime.set_native_text_pointer_caret(
        crate::widgets::WidgetId::from(12_u32),
        &source,
        caret,
        match affinity {
            CaretAffinity::Upstream => crate::widgets::NativeCaretAffinity::Upstream,
            CaretAffinity::Downstream => crate::widgets::NativeCaretAffinity::Downstream,
        },
    );
    runner.core.runtime.dispatch_input(
        crate::widgets::WidgetId::from(12_u32),
        WidgetInput::primary_press(position),
    );
    runner.commit_accepted_native_text_pointer_caret();
    assert_eq!(
        runner
            .frame
            .text_renderer
            .native_caret_affinity(crate::widgets::WidgetId::from(12_u32)),
        affinity
    );
}

#[test]
fn native_numeric_pointer_uses_shaped_caret_and_keeps_numeric_as_owner() {
    let mut harness =
        NativePointerHarness::new(NativeNumericBridge::default(), Vector2::new(260.0, 60.0));
    focus_native_numeric_with_tab(&mut harness);
    harness.runner.rebuild_scene();
    harness
        .runner
        .frame
        .seed_text_input_snapshots_for_current_plan(false, &Default::default());
    let (position, native_caret, native_affinity, generic_caret) =
        native_numeric_divergence(&mut harness.runner);
    assert_ne!(native_caret, generic_caret);
    assert_eq!(native_affinity, CaretAffinity::Upstream);

    harness.cursor_moved_logical(position);
    let press = harness.mouse_pressed(MouseButton::Left);
    assert!(press.routed, "native Numeric press should be routed");
    harness.runner.rebuild_scene();
    let pressed = native_numeric_paint_input(&harness.runner);
    assert_eq!(pressed.state.caret, native_caret);
    assert_eq!(pressed.state.selection_anchor, native_caret);
    assert_eq!(
        harness
            .runner
            .frame
            .text_renderer
            .native_caret_affinity(crate::widgets::WidgetId::from(NATIVE_NUMERIC_ID)),
        native_affinity
    );

    let before_character = pressed.state.value.clone();
    assert!(
        harness.runner.core.runtime.bridge().interactions.is_empty(),
        "Numeric owns active text editing without emitting a host message while typing"
    );
    let mut expected = before_character
        .chars()
        .take(native_caret)
        .collect::<String>();
    expected.push('X');
    expected.extend(before_character.chars().skip(native_caret));
    let character = harness.runner.core.route_character('X');
    assert!(
        character.routed,
        "focused Numeric should own character input"
    );
    harness.runner.apply_route_outcome(character);
    harness.runner.rebuild_scene();
    let after_character = native_numeric_paint_input(&harness.runner);
    assert_eq!(after_character.state.value, expected);
    assert_eq!(after_character.state.caret, native_caret + 1);
    assert_eq!(after_character.state.selection_anchor, native_caret + 1);
    assert!(
        harness.runner.core.runtime.bridge().interactions.is_empty(),
        "Numeric typing should remain local until the edit is committed"
    );

    let committed = harness.runner.core.route_key_press(
        KeyPress::new(KeyCode::Enter),
        WidgetKey::from_key_code(KeyCode::Enter),
    );
    harness.runner.apply_route_outcome(committed);
    let interactions = &harness.runner.core.runtime.bridge().interactions;
    assert_eq!(
        interactions.as_slice(),
        &[vec![
            (EditPhase::Begin, NATIVE_NUMERIC_SOURCE.to_owned()),
            (EditPhase::Commit, expected.clone()),
        ]],
        "Numeric Enter should emit exactly one mapped edit batch"
    );
}

#[test]
fn native_numeric_pointer_rejects_stale_shaped_source_after_reprojection() {
    let mut harness =
        NativePointerHarness::new(NativeNumericBridge::default(), Vector2::new(260.0, 60.0));
    focus_native_numeric_with_tab(&mut harness);
    harness
        .runner
        .frame
        .seed_text_input_snapshots_for_current_plan(false, &Default::default());
    let (position, _, _, _) = native_numeric_divergence(&mut harness.runner);
    let (_, stale_source, _, stale_affinity) = harness
        .runner
        .frame
        .native_text_pointer_target(position, None)
        .expect("seeded native snapshot should expose the old Numeric source");
    assert_eq!(stale_source, NATIVE_NUMERIC_SOURCE);
    assert!(matches!(
        stale_affinity,
        CaretAffinity::Upstream | CaretAffinity::Downstream
    ));
    harness.cursor_moved_logical(position);

    harness.runner.core.runtime.bridge_mut().value = String::from("WWWW");
    harness
        .runner
        .core
        .refresh_surface_with_scope(crate::runtime::RepaintScope::Surface);
    assert_eq!(
        harness.runner.core.runtime.focused_widget(),
        Some(crate::widgets::WidgetId::from(NATIVE_NUMERIC_ID))
    );
    let generic_caret = generic_numeric_caret_at_value(position, "WWWW");

    let press = harness.mouse_pressed(MouseButton::Left);
    assert!(
        press.routed,
        "stale native press should use generic fallback"
    );
    harness.runner.rebuild_scene();
    let after = native_numeric_paint_input(&harness.runner);
    assert_eq!(after.state.value, "WWWW");
    assert_eq!(after.state.caret, generic_caret);
    assert_eq!(after.state.selection_anchor, generic_caret);
    assert_eq!(
        harness
            .runner
            .frame
            .text_renderer
            .native_caret_affinity(crate::widgets::WidgetId::from(NATIVE_NUMERIC_ID)),
        CaretAffinity::Downstream,
        "stale native affinity must not be committed"
    );
}

#[test]
fn native_text_pointer_affinity_commits_for_pressed_drag_move() {
    let mut runner = GenericNativeVelloRunner::new(
        NativeRunOptions::default(),
        demo_bridge(),
        Vector2::new(320.0, 40.0),
    );
    runner.rebuild_scene();
    runner
        .frame
        .seed_text_input_snapshots_for_current_plan(false, &Default::default());
    let input_rect = runner
        .core
        .runtime
        .layout()
        .rects
        .get(&12)
        .copied()
        .expect("text input should be laid out");
    let press_position = Point::new(input_rect.min.x + 4.0, input_rect.min.y + 4.0);
    let drag_position = Point::new(input_rect.min.x + 24.0, input_rect.min.y + 4.0);
    let widget_id = crate::widgets::WidgetId::from(12_u32);

    runner.core.runtime.set_native_text_pointer_caret(
        widget_id,
        "",
        0,
        crate::widgets::NativeCaretAffinity::Upstream,
    );
    runner
        .core
        .runtime
        .dispatch_input(widget_id, WidgetInput::primary_press(press_position));
    runner.commit_accepted_native_text_pointer_caret();
    assert_eq!(
        runner.frame.text_renderer.native_caret_affinity(widget_id),
        CaretAffinity::Upstream
    );

    runner.core.runtime.set_native_text_pointer_caret(
        widget_id,
        "",
        0,
        crate::widgets::NativeCaretAffinity::Downstream,
    );
    runner
        .core
        .runtime
        .dispatch_input(widget_id, WidgetInput::pointer_move(drag_position));
    runner.commit_accepted_native_text_pointer_caret();
    assert_eq!(
        runner.frame.text_renderer.native_caret_affinity(widget_id),
        CaretAffinity::Downstream
    );
}

#[test]
fn native_text_pointer_drag_uses_exclusive_grapheme_boundaries() {
    let mut bridge = demo_bridge();
    bridge.state.name = String::from("ab");
    let mut harness = NativePointerHarness::new(bridge, Vector2::new(320.0, 40.0));
    harness
        .runner
        .frame
        .seed_text_input_snapshots_for_current_plan(false, &Default::default());
    let input_rect = harness
        .runner
        .core
        .runtime
        .layout()
        .rects
        .get(&12)
        .copied()
        .expect("text input should be laid out");
    let start = (1..input_rect.width() as usize)
        .map(|offset| Point::new(input_rect.min.x + offset as f32, input_rect.min.y + 4.0))
        .find(|position| {
            harness
                .runner
                .frame
                .native_text_pointer_target(*position, None)
                .is_some_and(|(_, _, scalar, _)| scalar == 0)
        })
        .expect("native hit testing should expose the start boundary");
    let after_a = (1..input_rect.width() as usize)
        .map(|offset| Point::new(input_rect.min.x + offset as f32, start.y))
        .find(|position| {
            harness
                .runner
                .frame
                .native_text_pointer_target(*position, None)
                .is_some_and(|(_, _, scalar, _)| scalar == 1)
        })
        .expect("native hit testing should expose the boundary after a");
    harness.cursor_moved_logical(start);
    let _ = harness.mouse_pressed(MouseButton::Left);
    harness.cursor_moved_logical(after_a);

    let widget_id = crate::widgets::WidgetId::from(12_u32);
    let widget = harness
        .runner
        .core
        .runtime
        .surface()
        .find_widget(widget_id)
        .expect("text input should remain projected");
    let input = widget
        .widget()
        .as_any()
        .downcast_ref::<crate::widgets::TextInputWidget>()
        .expect("projected widget should remain a text input");
    assert_eq!(input.state.selection_range(), (0, 1));
    assert_eq!(input.selected_text_slice(), Some("a"));

    let mut combining_bridge = demo_bridge();
    combining_bridge.state.name = String::from("e\u{301}x");
    let mut combining = NativePointerHarness::new(combining_bridge, Vector2::new(320.0, 40.0));
    combining
        .runner
        .frame
        .seed_text_input_snapshots_for_current_plan(false, &Default::default());
    let combining_rect = combining
        .runner
        .core
        .runtime
        .layout()
        .rects
        .get(&12)
        .copied()
        .expect("combining text input should be laid out");
    let combining_start = Point::new(combining_rect.min.x + 1.0, combining_rect.min.y + 4.0);
    let combining_after_cluster = (1..combining_rect.width() as usize)
        .map(|offset| Point::new(combining_rect.min.x + offset as f32, combining_start.y))
        .find(|position| {
            combining
                .runner
                .frame
                .native_text_pointer_target(*position, None)
                .is_some_and(|(_, _, scalar, _)| scalar == 2)
        })
        .expect("native hit testing should expose the boundary after the combining cluster");
    combining.cursor_moved_logical(combining_start);
    let _ = combining.mouse_pressed(MouseButton::Left);
    combining.cursor_moved_logical(combining_after_cluster);

    let combining_widget = combining
        .runner
        .core
        .runtime
        .surface()
        .find_widget(widget_id)
        .expect("combining text input should remain projected");
    let combining_input = combining_widget
        .widget()
        .as_any()
        .downcast_ref::<crate::widgets::TextInputWidget>()
        .expect("projected combining widget should remain a text input");
    assert_eq!(combining_input.state.selection_range(), (0, 2));
    assert_eq!(combining_input.selected_text_slice(), Some("e\u{301}"));
}

#[test]
fn native_text_pointer_affinity_ignores_discarded_hover_and_release_carets() {
    let mut runner = GenericNativeVelloRunner::new(
        NativeRunOptions::default(),
        demo_bridge(),
        Vector2::new(320.0, 40.0),
    );
    runner.rebuild_scene();
    runner
        .frame
        .seed_text_input_snapshots_for_current_plan(false, &Default::default());
    let input_rect = runner
        .core
        .runtime
        .layout()
        .rects
        .get(&12)
        .copied()
        .expect("text input should be laid out");
    let position = Point::new(input_rect.min.x + 4.0, input_rect.min.y + 4.0);
    let widget_id = crate::widgets::WidgetId::from(12_u32);

    runner
        .frame
        .text_renderer
        .set_native_caret_affinity(widget_id, CaretAffinity::Upstream);
    for input in [
        WidgetInput::pointer_move(position),
        WidgetInput::pointer_release(
            position,
            crate::widgets::PointerButton::Primary,
            Default::default(),
        ),
    ] {
        runner.core.runtime.set_native_text_pointer_caret(
            widget_id,
            "",
            0,
            crate::widgets::NativeCaretAffinity::Downstream,
        );
        runner.core.runtime.dispatch_input(widget_id, input);
        runner.commit_accepted_native_text_pointer_caret();
    }

    assert_eq!(
        runner.frame.text_renderer.native_caret_affinity(widget_id),
        CaretAffinity::Upstream
    );
}

#[test]
fn native_pointer_move_delivers_captured_timestamp_and_normalized_modifiers() {
    let mut harness = NativePointerHarness::new(
        NativeMoveMetadataBridge::default(),
        Vector2::new(120.0, 40.0),
    );
    let point = Point::new(8.0, 8.0);

    harness
        .modifiers_changed(ModifiersState::CONTROL | ModifiersState::SHIFT | ModifiersState::ALT);
    harness.cursor_moved_logical(point);

    let samples = &harness.runner.core.runtime.bridge().samples;
    assert_eq!(samples.len(), 1);
    assert_eq!(samples[0].position, point);
    assert_eq!(
        samples[0].modifiers,
        PointerModifiers {
            command: true,
            shift: true,
            alt: true,
        }
    );
    assert!(samples[0].timestamp.is_some());
    assert!(samples[0].sequence_range.is_some());
}

#[test]
fn native_pointer_press_delivers_one_captured_timestamp_to_widget_input() {
    let mut harness =
        NativePointerHarness::new(PressTimestampBridge::default(), Vector2::new(120.0, 40.0));
    let point = Point::new(8.0, 8.0);

    harness.cursor_moved_logical(point);
    assert!(
        harness
            .mouse_pressed_route(MouseButton::Left)
            .outcome
            .routed
    );

    let timestamps = &harness.runner.core.runtime.bridge().timestamps;
    assert_eq!(timestamps.len(), 1);
    assert!(timestamps[0].is_some());
}

#[test]
fn native_pointer_modifiers_changed_delivers_one_captured_timestamp_to_widget_input() {
    let mut harness =
        NativePointerHarness::new(PressTimestampBridge::default(), Vector2::new(120.0, 40.0));
    harness.cursor_moved_logical(Point::new(8.0, 8.0));

    harness.modifiers_changed(ModifiersState::SHIFT);

    let timestamps = &harness.runner.core.runtime.bridge().timestamps;
    assert_eq!(timestamps.len(), 1);
    assert!(timestamps[0].is_some());
}

#[test]
fn native_pointer_double_click_delivers_captured_timestamp_to_widget_input() {
    let mut harness =
        NativePointerHarness::new(PressTimestampBridge::default(), Vector2::new(120.0, 40.0));
    let point = Point::new(8.0, 8.0);

    harness.cursor_moved_logical(point);
    assert!(
        harness
            .mouse_pressed_route(MouseButton::Left)
            .outcome
            .routed
    );
    let second_press = harness.mouse_pressed_route(MouseButton::Left);
    assert!(second_press.outcome.routed);
    assert!(second_press.double_click);

    let timestamps = &harness.runner.core.runtime.bridge().timestamps;
    assert_eq!(timestamps.len(), 2);
    assert!(timestamps.iter().all(Option::is_some));
}

#[test]
fn native_pointer_release_delivers_one_captured_timestamp_to_widget_input() {
    let mut harness =
        NativePointerHarness::new(PressTimestampBridge::default(), Vector2::new(120.0, 40.0));
    let point = Point::new(8.0, 8.0);

    harness.cursor_moved_logical(point);
    assert!(
        harness
            .mouse_released_route(MouseButton::Left)
            .outcome
            .routed
    );

    let timestamps = &harness.runner.core.runtime.bridge().timestamps;
    assert_eq!(timestamps.len(), 1);
    assert!(timestamps[0].is_some());
}

#[test]
fn native_focus_loss_modifier_reset_delivers_no_timestamp_and_clears_native_state() {
    let mut harness =
        NativePointerHarness::new(PressTimestampBridge::default(), Vector2::new(120.0, 40.0));
    harness.cursor_moved_logical(Point::new(8.0, 8.0));
    assert!(
        harness
            .mouse_pressed_route(MouseButton::Left)
            .outcome
            .routed
    );
    harness.modifiers_changed(ModifiersState::ALT);

    let outcome = harness.focus_lost();

    assert!(outcome.needs_scene_rebuild());
    assert!(harness.runner.input.modifiers.is_empty());
    assert_eq!(harness.runner.input.last_cursor, None);
    assert_eq!(harness.runner.core.runtime.pointer_capture(), None);
    let timestamps = &harness.runner.core.runtime.bridge().timestamps;
    assert_eq!(timestamps.len(), 3);
    assert!(timestamps[0].is_some());
    assert!(timestamps[1].is_some());
    assert!(timestamps[2].is_none());
}

#[test]
fn native_keypress_update_snapshot_uses_hover_cursor_without_mouse_press() {
    let mut harness = NativePointerHarness::new(
        PointerSnapshotShortcutBridge::default(),
        Vector2::new(320.0, 40.0),
    );
    let hover = Point::new(88.0, 18.0);

    harness.runner.input.last_cursor = Some(hover);
    harness
        .runner
        .core
        .runtime
        .set_current_pointer_position(None);
    harness.runner.sync_runtime_pointer_from_native_cursor();
    let outcome = harness.runner.core.route_key_press(
        KeyPress::new(KeyCode::W),
        WidgetKey::from_key_code(KeyCode::W),
    );
    harness.runner.apply_route_outcome(outcome);

    assert_eq!(
        harness.runner.core.runtime.bridge().snapshots,
        vec![Some(hover)]
    );
}

#[test]
fn native_pointer_harness_uses_physical_to_logical_cursor_conversion() {
    let mut harness = NativePointerHarness::new(demo_bridge(), Vector2::new(320.0, 40.0));
    harness.runner.window.dpi_scale = crate::theme::DpiScale::new(2.0);

    harness.cursor_moved_physical(PhysicalPosition::new(40.0, 24.0));

    assert_eq!(
        harness.runner.input.last_cursor,
        Some(Point::new(20.0, 12.0))
    );
}

#[test]
fn native_pointer_enter_reasserts_default_cursor_when_cache_is_default() {
    let mut harness = NativePointerHarness::new(demo_bridge(), Vector2::new(320.0, 40.0));
    harness.runner.input.native_cursor = Some(crate::widgets::WidgetCursor::Default);
    harness.runner.input.native_cursor_visible = false;
    let updates_before = harness.runner.input.native_cursor_apply_count;

    harness.cursor_entered();

    assert!(harness.runner.input.native_cursor_visible);
    assert_eq!(
        harness.runner.input.native_cursor,
        Some(crate::widgets::WidgetCursor::Default)
    );
    assert_eq!(
        harness.runner.input.native_cursor_apply_count,
        updates_before + 1,
        "cursor entry must reclaim native cursor ownership even when the cached logical cursor did not change"
    );
}

#[test]
fn native_pointer_first_move_reasserts_default_cursor_when_cache_is_default() {
    let mut harness = NativePointerHarness::new(demo_bridge(), Vector2::new(320.0, 40.0));
    harness.runner.input.native_cursor = Some(crate::widgets::WidgetCursor::Default);
    let updates_before = harness.runner.input.native_cursor_apply_count;

    harness.cursor_moved_logical(Point::new(4.0, 4.0));

    assert_eq!(
        harness.runner.input.native_cursor,
        Some(crate::widgets::WidgetCursor::Default)
    );
    assert!(
        harness.runner.input.native_cursor_apply_count > updates_before,
        "first pointer motion after an absent cursor must not trust a stale native cursor cache"
    );
}

#[test]
fn native_pointer_repeated_hover_move_reasserts_default_cursor_when_cache_is_default() {
    let mut harness = NativePointerHarness::new(demo_bridge(), Vector2::new(320.0, 40.0));
    harness.cursor_moved_logical(Point::new(4.0, 4.0));
    harness.runner.input.native_cursor = Some(crate::widgets::WidgetCursor::Default);
    let updates_before = harness.runner.input.native_cursor_apply_count;

    harness.cursor_moved_logical(Point::new(5.0, 4.0));

    assert_eq!(
        harness.runner.input.native_cursor,
        Some(crate::widgets::WidgetCursor::Default)
    );
    assert!(
        harness.runner.input.native_cursor_apply_count > updates_before,
        "hover motion inside the app must reclaim native cursor ownership even when the cached logical cursor did not change"
    );
}

#[test]
fn native_pointer_harness_drops_mouse_input_without_cursor() {
    let mut harness = NativePointerHarness::new(demo_bridge(), Vector2::new(320.0, 40.0));

    let route = harness.mouse_pressed_route(MouseButton::Left);

    assert!(!route.outcome.routed);
    assert_eq!(route.diagnostic.kind, NativePointerEventKind::MousePress);
    assert_eq!(route.diagnostic.result, NativePointerRouteResult::NoCursor);
    assert_eq!(route.diagnostic.position, None);
    assert_eq!(route.diagnostic.hit_target, None);
    assert_eq!(harness.runner.core.runtime.bridge().state.count, 0);
}

#[test]
fn native_pointer_diagnostics_report_hit_target_and_capture_state() {
    let mut harness = NativePointerHarness::new(demo_bridge(), Vector2::new(320.0, 40.0));
    let button_point = harness
        .runner
        .core
        .runtime
        .layout()
        .rects
        .get(&11)
        .map(|rect| Point::new(rect.min.x + 4.0, rect.min.y + 4.0))
        .expect("button should be laid out");
    harness.cursor_moved_logical(button_point);

    let press = harness.mouse_pressed_route(MouseButton::Left);

    assert!(press.outcome.routed);
    assert_eq!(press.diagnostic.result, NativePointerRouteResult::Routed);
    assert_eq!(press.diagnostic.position, Some(button_point));
    assert_eq!(press.diagnostic.button, Some(PointerButton::Primary));
    assert_eq!(press.diagnostic.hit_target, Some(11));
    assert_eq!(press.diagnostic.captured_widget, None);
    assert!(press.diagnostic.outcome.needs_redraw());

    let release = harness.mouse_released_route(MouseButton::Left);

    assert!(release.outcome.routed);
    assert_eq!(release.diagnostic.result, NativePointerRouteResult::Routed);
    assert_eq!(release.diagnostic.hit_target, Some(11));
    assert_eq!(release.diagnostic.captured_widget, Some(11));
}

#[test]
#[cfg(target_os = "macos")]
fn macos_control_left_is_one_latched_secondary_gesture() {
    let mut harness = NativePointerHarness::new(demo_bridge(), Vector2::new(320.0, 40.0));
    let point = Point::new(12.0, 12.0);
    harness.cursor_moved_logical(point);
    harness.modifiers_changed(
        ModifiersState::CONTROL
            | ModifiersState::SUPER
            | ModifiersState::SHIFT
            | ModifiersState::ALT,
    );

    let press = harness.mouse_pressed_route(MouseButton::Left);
    assert_eq!(press.button, Some(PointerButton::Secondary));
    assert_eq!(
        press.diagnostic.modifiers,
        PointerModifiers {
            command: true,
            shift: true,
            alt: true,
        }
    );
    assert_eq!(
        harness.runner.input.effective_pointer_gesture,
        Some(
            crate::gui_runtime::native_vello::generic_runtime::input::NativePointerGestureLatch {
                physical_button: MouseButton::Left,
                gesture:
                    crate::gui_runtime::native_vello::generic_runtime::input::NativePointerGesture {
                        button: PointerButton::Secondary,
                        consume_control: true,
                    },
            }
        )
    );

    // macOS can report the Control release before the matching mouse-up. The
    // latched effective button must still drive the release.
    harness.modifiers_changed(ModifiersState::SUPER | ModifiersState::SHIFT);
    let release = harness.mouse_released_route(MouseButton::Left);
    assert_eq!(release.button, Some(PointerButton::Secondary));
    assert!(harness.runner.input.effective_pointer_gesture.is_none());
}

#[test]
#[cfg(target_os = "macos")]
fn macos_command_left_without_control_remains_primary() {
    let mut harness = NativePointerHarness::new(demo_bridge(), Vector2::new(320.0, 40.0));
    harness.cursor_moved_logical(Point::new(12.0, 12.0));
    harness.modifiers_changed(ModifiersState::SUPER);

    let press = harness.mouse_pressed_route(MouseButton::Left);
    assert_eq!(press.button, Some(PointerButton::Primary));
    assert!(press.diagnostic.modifiers.command);
    let release = harness.mouse_released_route(MouseButton::Left);
    assert_eq!(release.button, Some(PointerButton::Primary));
}

#[test]
#[cfg(target_os = "macos")]
fn macos_secondary_double_click_uses_converted_button_identity() {
    let mut harness = NativePointerHarness::new(demo_bridge(), Vector2::new(320.0, 40.0));
    harness.cursor_moved_logical(Point::new(12.0, 12.0));
    harness.modifiers_changed(ModifiersState::CONTROL);

    let first_press = harness.mouse_pressed_route(MouseButton::Left);
    assert!(!first_press.double_click);
    let _first_release = harness.mouse_released_route(MouseButton::Left);
    let second_press = harness.mouse_pressed_route(MouseButton::Left);
    assert!(second_press.double_click);
    assert_eq!(second_press.button, Some(PointerButton::Secondary));
    let _second_release = harness.mouse_released_route(MouseButton::Left);
}

#[test]
#[cfg(target_os = "macos")]
fn macos_control_left_release_keeps_latched_button_during_capture() {
    let mut harness = NativePointerHarness::new(demo_bridge(), Vector2::new(320.0, 40.0));
    let button_point = harness
        .runner
        .core
        .runtime
        .layout()
        .rects
        .get(&11)
        .map(|rect| Point::new(rect.min.x + 4.0, rect.min.y + 4.0))
        .expect("button should be laid out");
    harness.cursor_moved_logical(button_point);
    harness.modifiers_changed(ModifiersState::CONTROL);

    let press = harness.mouse_pressed_route(MouseButton::Left);
    assert_eq!(press.button, Some(PointerButton::Secondary));
    assert_eq!(harness.runner.core.runtime.pointer_capture(), Some(11));

    harness.cursor_moved_logical(Point::new(button_point.x + 40.0, button_point.y + 8.0));
    harness.modifiers_changed(ModifiersState::empty());
    let release = harness.mouse_released_route(MouseButton::Left);
    assert_eq!(release.button, Some(PointerButton::Secondary));
    assert!(harness.runner.input.effective_pointer_gesture.is_none());
}

#[test]
#[cfg(target_os = "macos")]
fn macos_control_left_latch_survives_interleaved_physical_buttons() {
    let mut harness = NativePointerHarness::new(demo_bridge(), Vector2::new(320.0, 40.0));
    harness.cursor_moved_logical(Point::new(12.0, 12.0));
    harness.modifiers_changed(ModifiersState::CONTROL);

    let press = harness.mouse_pressed_route(MouseButton::Left);
    assert_eq!(press.button, Some(PointerButton::Secondary));
    assert!(!press.diagnostic.modifiers.command);
    assert!(harness.runner.input.effective_pointer_gesture.is_some());

    for interleaved in [MouseButton::Right, MouseButton::Other(5)] {
        let interleaved_press = harness.mouse_pressed_route(interleaved);
        let interleaved_release = harness.mouse_released_route(interleaved);
        if interleaved == MouseButton::Right {
            assert_eq!(interleaved_press.button, Some(PointerButton::Secondary));
            assert_eq!(interleaved_release.button, Some(PointerButton::Secondary));
        } else {
            assert_eq!(interleaved_press.button, None);
            assert_eq!(interleaved_release.button, None);
        }
        assert!(harness.runner.input.effective_pointer_gesture.is_some());
    }

    harness.modifiers_changed(ModifiersState::empty());
    let release = harness.mouse_released_route(MouseButton::Left);
    assert_eq!(release.button, Some(PointerButton::Secondary));
    assert!(!release.diagnostic.modifiers.command);
    assert!(harness.runner.input.effective_pointer_gesture.is_none());
}

#[test]
#[cfg(not(target_os = "macos"))]
fn non_macos_control_left_remains_primary() {
    let mut harness = NativePointerHarness::new(demo_bridge(), Vector2::new(320.0, 40.0));
    let point = Point::new(12.0, 12.0);
    harness.cursor_moved_logical(point);
    harness.modifiers_changed(ModifiersState::CONTROL);

    let press = harness.mouse_pressed_route(MouseButton::Left);
    assert_eq!(press.button, Some(PointerButton::Primary));
    let release = harness.mouse_released_route(MouseButton::Left);
    assert_eq!(release.button, Some(PointerButton::Primary));
}

#[test]
#[cfg(target_os = "macos")]
fn macos_physical_right_remains_secondary_without_control_consumption() {
    let mut harness = NativePointerHarness::new(demo_bridge(), Vector2::new(320.0, 40.0));
    harness.cursor_moved_logical(Point::new(12.0, 12.0));
    harness.modifiers_changed(ModifiersState::CONTROL | ModifiersState::SHIFT);

    let press = harness.mouse_pressed_route(MouseButton::Right);
    assert_eq!(press.button, Some(PointerButton::Secondary));
    assert_eq!(
        press.diagnostic.modifiers,
        PointerModifiers {
            command: true,
            shift: true,
            alt: false,
        }
    );
    let release = harness.mouse_released_route(MouseButton::Right);
    assert_eq!(release.button, Some(PointerButton::Secondary));
}

#[test]
#[cfg(target_os = "macos")]
fn macos_focus_loss_clears_effective_pointer_gesture_latch() {
    let mut harness = NativePointerHarness::new(demo_bridge(), Vector2::new(320.0, 40.0));
    harness.cursor_moved_logical(Point::new(12.0, 12.0));
    harness.modifiers_changed(ModifiersState::CONTROL);
    let _press = harness.mouse_pressed_route(MouseButton::Left);
    assert!(harness.runner.input.effective_pointer_gesture.is_some());

    let _ = harness.focus_lost();

    assert!(harness.runner.input.effective_pointer_gesture.is_none());
}

#[test]
fn native_pointer_harness_routes_wheel_with_modifiers() {
    let mut harness =
        NativePointerHarness::new(GpuWheelBridge::default(), Vector2::new(320.0, 80.0));
    harness.cursor_moved_logical(Point::new(40.0, 20.0));
    harness.modifiers_changed(ModifiersState::SHIFT);

    let route = harness.mouse_wheel_route(MouseScrollDelta::LineDelta(0.0, -2.0));
    harness
        .runner
        .flush_pending_gpu_surface_wheel(&mut RenderFrameProfile::default());

    assert_eq!(route.outcome.frame_work(), FrameWork::None);
    assert_eq!(route.diagnostic.kind, NativePointerEventKind::MouseWheel);
    assert_eq!(route.diagnostic.result, NativePointerRouteResult::Coalesced);
    assert_eq!(route.diagnostic.hit_target, Some(61));
    assert_eq!(harness.runner.core.runtime.bridge().wheel_count, 1);
    assert_eq!(
        harness.runner.core.runtime.bridge().last_delta,
        Vector2::new(0.0, 80.0)
    );
    assert_eq!(
        harness.runner.core.runtime.bridge().last_modifiers,
        Some(PointerModifiers {
            shift: true,
            ..PointerModifiers::default()
        })
    );
    assert!(
        harness
            .runner
            .core
            .runtime
            .bridge()
            .last_timestamp
            .is_some()
    );
}

#[test]
fn native_phaseful_wheel_dispatches_exact_sample_without_coalescing() {
    let mut harness =
        NativePointerHarness::new(GpuWheelBridge::default(), Vector2::new(320.0, 80.0));
    harness.cursor_moved_logical(Point::new(40.0, 20.0));

    let route = harness.runner.route_native_mouse_wheel_with_phase(
        MouseScrollDelta::LineDelta(0.0, -2.0),
        TouchPhase::Moved,
    );

    assert!(route.outcome.routed);
    assert_eq!(route.diagnostic.result, NativePointerRouteResult::Routed);
    assert!(harness.runner.input.pending_gpu_surface_wheel.is_none());
    assert!(
        harness
            .runner
            .input
            .pending_scroll_container_wheel
            .is_none()
    );
    let bridge = harness.runner.core.runtime.bridge();
    assert_eq!(bridge.wheel_count, 1);
    assert_eq!(bridge.last_delta, Vector2::new(0.0, 80.0));
    assert!(bridge.last_timestamp.is_some());
    assert!(bridge.last_sequence_range.is_some());
}

#[test]
fn native_phaseful_managed_wheel_stays_exact_across_hit_target_changes() {
    let mut harness =
        NativePointerHarness::new(ManagedWheelBridge::default(), Vector2::new(240.0, 80.0));
    let widget_position = Point::new(40.0, 20.0);
    harness.runner.input.last_cursor = Some(widget_position);

    let started = harness.runner.route_native_mouse_wheel_with_phase(
        MouseScrollDelta::LineDelta(0.0, -0.25),
        TouchPhase::Started,
    );
    assert_eq!(started.diagnostic.result, NativePointerRouteResult::Routed);
    assert!(
        harness
            .runner
            .input
            .pending_scroll_container_wheel
            .is_none()
    );
    assert_eq!(harness.runner.core.runtime.bridge().samples.len(), 1);
    assert_eq!(
        harness.runner.core.runtime.bridge().samples[0].phase,
        Some(WheelPhase::Started)
    );

    harness.runner.input.last_cursor = Some(Point::new(200.0, 60.0));
    harness.runner.timing.redraw_requested = true;
    harness.runner.timing.redraw_requested_at = Some(Instant::now());
    let changed = harness.runner.route_native_mouse_wheel_with_phase(
        MouseScrollDelta::PixelDelta(PhysicalPosition::new(0.0, -8.0)),
        TouchPhase::Moved,
    );
    assert_eq!(changed.diagnostic.result, NativePointerRouteResult::Routed);
    assert!(
        harness
            .runner
            .input
            .pending_scroll_container_wheel
            .is_none()
    );
    assert_eq!(harness.runner.core.runtime.bridge().samples.len(), 2);
    assert_eq!(
        harness.runner.core.runtime.bridge().samples[1].delta,
        WheelDelta::Pixels(Vector2::new(0.0, 8.0))
    );
    assert_eq!(
        harness.runner.core.runtime.bridge().samples[1].phase,
        Some(WheelPhase::Changed)
    );

    let ended = harness.runner.route_native_mouse_wheel_with_phase(
        MouseScrollDelta::LineDelta(0.0, -0.25),
        TouchPhase::Ended,
    );
    assert_eq!(ended.diagnostic.result, NativePointerRouteResult::Routed);
    assert!(
        harness
            .runner
            .input
            .pending_scroll_container_wheel
            .is_none()
    );
    assert_eq!(harness.runner.core.runtime.bridge().samples.len(), 3);
    assert_eq!(
        harness.runner.core.runtime.bridge().samples[2].phase,
        Some(WheelPhase::Ended)
    );
}

#[test]
fn native_direct_wheel_preserves_effective_modifiers_and_timestamp() {
    let point = Point::new(40.0, 20.0);
    let delta = Vector2::new(0.0, -2.0);
    let expected_modifiers = PointerModifiers {
        shift: true,
        ..PointerModifiers::default()
    };

    let mut synthetic =
        GenericNativeRuntimeCore::new(ModifierWheelBridge::default(), Vector2::new(120.0, 80.0));
    assert!(
        synthetic
            .route_scroll_with_modifiers(point, delta, expected_modifiers)
            .routed
    );
    assert_eq!(
        synthetic.runtime.bridge().samples,
        vec![(expected_modifiers, None, None)]
    );

    let mut harness =
        NativePointerHarness::new(ModifierWheelBridge::default(), Vector2::new(120.0, 80.0));
    harness.cursor_moved_logical(point);
    harness.modifiers_changed(ModifiersState::SHIFT);

    let route = harness.mouse_wheel_route(MouseScrollDelta::LineDelta(0.0, -2.0));

    assert!(route.outcome.routed);
    assert_eq!(route.diagnostic.result, NativePointerRouteResult::Routed);
    assert_eq!(route.diagnostic.modifiers, expected_modifiers);
    let samples = &harness.runner.core.runtime.bridge().samples;
    assert_eq!(samples.len(), 1);
    assert_eq!(samples[0].0, expected_modifiers);
    assert!(samples[0].1.is_some());
    assert!(samples[0].2.is_some());
}

#[test]
fn canceled_coalesced_wheel_does_not_retain_synthetic_lifecycle_work() {
    let mut harness =
        NativePointerHarness::new(GpuWheelBridge::default(), Vector2::new(320.0, 80.0));
    harness.cursor_moved_logical(Point::new(40.0, 20.0));
    harness.runner.take_pending_frame_work();

    let route = harness.mouse_wheel_route(MouseScrollDelta::LineDelta(0.0, -2.0));
    assert_eq!(route.diagnostic.result, NativePointerRouteResult::Coalesced);
    assert_eq!(route.outcome.frame_work(), FrameWork::None);
    assert!(harness.runner.input.pending_gpu_surface_wheel.is_some());

    harness.runner.apply_route_outcome(route.outcome);
    assert_eq!(harness.runner.timing.pending_frame_work, FrameWork::None);

    harness.focus_lost();

    assert!(harness.runner.input.pending_gpu_surface_wheel.is_none());
    assert_eq!(
        harness.runner.timing.pending_frame_work,
        FrameWork::None,
        "focus-loss cancellation must not leave synthetic wheel work for presentation"
    );
}

#[test]
fn native_pointer_harness_refreshes_scroll_area_wheel_surface_interactively() {
    let mut harness =
        NativePointerHarness::new(ScrollRefreshBridge::default(), Vector2::new(240.0, 40.0));
    harness.cursor_moved_logical(Point::new(12.0, 12.0));

    let route = harness.mouse_wheel_route(MouseScrollDelta::LineDelta(0.0, -2.0));

    assert_eq!(route.diagnostic.result, NativePointerRouteResult::Coalesced);
    assert!(
        harness
            .runner
            .input
            .pending_scroll_container_wheel
            .is_some()
    );
    assert_eq!(harness.runner.core.runtime.bridge().scroll_count, 0);
    harness
        .runner
        .flush_pending_scroll_container_wheel(&mut RenderFrameProfile::default());

    assert_eq!(route.diagnostic.kind, NativePointerEventKind::MouseWheel);
    assert_eq!(harness.runner.core.runtime.bridge().scroll_count, 1);
    assert_eq!(
        harness.runner.core.runtime.bridge().project_count,
        2,
        "native wheel routing should refresh the projected surface on the first interactive frame"
    );
    assert!(!harness.runner.timing.deferred_surface_refresh);
    assert!(!harness.runner.timing.deferred_scene_rebuild);

    harness
        .runner
        .rebuild_deferred_scene_if_needed(&mut RenderFrameProfile::default());
    assert_eq!(
        harness.runner.core.runtime.bridge().project_count,
        2,
        "no extra deferred scene rebuild should be queued after the immediate interactive refresh"
    );
}

#[test]
fn native_wheel_flushes_coalesced_scroll_when_redraw_is_starved() {
    let mut harness =
        NativePointerHarness::new(AppVirtualListBridge::default(), Vector2::new(240.0, 80.0));
    harness.cursor_moved_logical(Point::new(20.0, 20.0));
    harness.runner.timing.redraw_requested = true;
    harness.runner.timing.redraw_requested_at = Some(Instant::now() - Duration::from_millis(20));

    let route = harness.mouse_wheel_route(MouseScrollDelta::LineDelta(0.0, -100.0));

    assert_eq!(route.diagnostic.result, NativePointerRouteResult::Routed);
    assert_eq!(harness.runner.core.runtime.bridge().scroll_count, 1);
    assert!(harness.runner.core.runtime.bridge().project_count > 1);
    assert!(
        harness
            .runner
            .core
            .runtime
            .paint_plan(&Default::default())
            .contains_text("Row 99"),
        "starved wheel redraw should refresh virtual rows immediately"
    );
}

#[test]
fn native_wheel_flushes_stale_coalesced_scroll_before_new_wheel_input() {
    let mut harness =
        NativePointerHarness::new(AppVirtualListBridge::default(), Vector2::new(240.0, 80.0));
    harness.cursor_moved_logical(Point::new(20.0, 20.0));
    harness.runner.timing.redraw_requested = true;
    harness.runner.timing.redraw_requested_at = Some(Instant::now());

    let queued = harness.mouse_wheel_route(MouseScrollDelta::LineDelta(0.0, -20.0));
    assert_eq!(
        queued.diagnostic.result,
        NativePointerRouteResult::Coalesced
    );
    assert_eq!(queued.outcome.frame_work(), FrameWork::None);
    assert_eq!(
        harness.runner.core.runtime.bridge().scroll_count,
        0,
        "fresh pending redraws may coalesce scroll input until paint"
    );

    harness.runner.timing.redraw_requested_at = Some(Instant::now() - Duration::from_millis(20));
    let routed = harness.mouse_wheel_route(MouseScrollDelta::LineDelta(0.0, -20.0));

    assert_eq!(routed.diagnostic.result, NativePointerRouteResult::Routed);
    assert!(
        harness
            .runner
            .input
            .pending_scroll_container_wheel
            .is_none(),
        "new wheel input must not leave an older coalesced scroll delta pending"
    );
    assert!(
        harness.runner.core.runtime.bridge().scroll_count >= 2,
        "stale coalesced scroll should flush before routing the fresh wheel event"
    );
    assert!(
        harness.runner.core.runtime.bridge().window.viewport_start >= 80,
        "ordered wheel deltas should advance the app-owned virtual window"
    );
    let paint = harness.runner.core.runtime.paint_plan(&Default::default());
    assert!(
        paint.text_runs().count() > 0 && paint.contains_text("Row 80"),
        "ordered wheel flushing should keep virtual-list rows rendered after a large jump"
    );
}

#[test]
fn native_pointer_press_flushes_coalesced_scroll_before_click_routing() {
    let mut harness =
        NativePointerHarness::new(AppVirtualListBridge::default(), Vector2::new(240.0, 80.0));
    harness.cursor_moved_logical(Point::new(20.0, 20.0));
    harness.runner.timing.redraw_requested = true;
    harness.runner.timing.redraw_requested_at = Some(Instant::now());

    let queued = harness.mouse_wheel_route(MouseScrollDelta::LineDelta(0.0, -100.0));
    assert_eq!(
        queued.diagnostic.result,
        NativePointerRouteResult::Coalesced
    );
    assert!(
        harness
            .runner
            .input
            .pending_scroll_container_wheel
            .is_some(),
        "fresh wheel input should be pending before the click"
    );
    assert_eq!(harness.runner.core.runtime.bridge().scroll_count, 0);

    let _press = harness.mouse_pressed_route(MouseButton::Left);

    assert!(
        harness
            .runner
            .input
            .pending_scroll_container_wheel
            .is_none(),
        "mouse press should commit pending scroll before hit testing the click"
    );
    assert_eq!(harness.runner.core.runtime.bridge().scroll_count, 1);
    assert!(
        harness.runner.core.runtime.bridge().window.viewport_start >= 80,
        "the coalesced scroll should update the app-owned virtual window before click routing"
    );
    assert!(
        harness
            .runner
            .core
            .runtime
            .paint_plan(&Default::default())
            .contains_text("Row 99"),
        "click routing should see freshly materialized bottom rows"
    );
}

#[test]
fn native_scrollbar_drag_flushes_when_redraw_is_starved() {
    let mut harness =
        NativePointerHarness::new(AppVirtualListBridge::default(), Vector2::new(240.0, 80.0));
    let scroll_rect = harness
        .runner
        .core
        .runtime
        .layout()
        .rects
        .get(&81)
        .copied()
        .expect("virtual list scroll area should be laid out");
    let press = Point::new(scroll_rect.max.x - 2.0, scroll_rect.min.y + 6.0);
    let drag = Point::new(press.x, scroll_rect.max.y - 6.0);

    harness.cursor_moved_logical(press);
    harness.mouse_pressed(MouseButton::Left);
    harness.runner.timing.redraw_requested = true;
    harness.runner.timing.redraw_requested_at = Some(Instant::now() - Duration::from_millis(20));
    harness.cursor_moved_logical(drag);

    assert!(harness.runner.input.pending_scrollbar_drag.is_none());
    assert_eq!(harness.runner.core.runtime.bridge().scroll_count, 1);
    assert!(harness.runner.core.runtime.bridge().project_count > 1);
    assert!(
        harness
            .runner
            .core
            .runtime
            .paint_plan(&Default::default())
            .contains_text("Row 99"),
        "starved scrollbar redraw should refresh virtual rows immediately"
    );
}

#[test]
fn native_scrollbar_drag_flushes_newest_position_and_sample_metadata() {
    let mut runner = GenericNativeVelloRunner::new(
        NativeRunOptions::default(),
        NativeMoveMetadataBridge::default(),
        Vector2::new(120.0, 40.0),
    );
    runner.rebuild_scene();
    let first_timestamp = Some(InputTimestamp::capture());
    let newest_timestamp = Some(InputTimestamp::capture());
    let first_modifiers = PointerModifiers {
        shift: true,
        ..PointerModifiers::default()
    };
    let newest_modifiers = PointerModifiers {
        command: true,
        alt: true,
        ..PointerModifiers::default()
    };
    let first_position = Point::new(8.0, 8.0);
    let newest_position = Point::new(24.0, 8.0);
    let first_sequence = runner
        .input
        .input_sequence_allocator
        .allocate()
        .expect("first scrollbar sample should receive a sequence range");
    let newest_sequence = runner
        .input
        .input_sequence_allocator
        .allocate()
        .expect("newest scrollbar sample should receive a sequence range");

    runner.queue_scrollbar_drag_with_metadata(
        first_position,
        first_modifiers,
        first_timestamp,
        Some(first_sequence),
    );
    runner.queue_scrollbar_drag_with_metadata(
        newest_position,
        newest_modifiers,
        newest_timestamp,
        Some(newest_sequence),
    );

    let pending = runner
        .input
        .pending_scrollbar_drag
        .expect("latest scrollbar sample should remain pending");
    assert_eq!(pending.position, newest_position);
    assert_eq!(pending.modifiers, newest_modifiers);
    assert_eq!(pending.timestamp, newest_timestamp);
    let pending_sequence = pending
        .sequence_range
        .expect("pending scrollbar drag should retain sequence metadata");
    assert_eq!(pending_sequence.start(), first_sequence.start());
    assert_eq!(pending_sequence.end(), newest_sequence.end());

    runner.flush_pending_scrollbar_drag_now();

    assert!(runner.input.pending_scrollbar_drag.is_none());
    let samples = &runner.core.runtime.bridge().samples;
    assert_eq!(samples.len(), 1);
    assert_eq!(samples[0].position, newest_position);
    assert_eq!(samples[0].modifiers, newest_modifiers);
    assert_eq!(samples[0].timestamp, newest_timestamp);
    let delivered_sequence = samples[0]
        .sequence_range
        .expect("flushed scrollbar drag should retain sequence metadata");
    assert_eq!(delivered_sequence.start(), first_sequence.start());
    assert_eq!(delivered_sequence.end(), newest_sequence.end());
}

#[test]
fn native_scrollbar_queue_flushes_original_identity_after_contact_reuse() {
    let mut runner = GenericNativeVelloRunner::new(
        NativeRunOptions::default(),
        NativeTypedPointerBridge::default(),
        Vector2::new(120.0, 40.0),
    );
    runner.rebuild_scene();
    let native_device = DeviceId::dummy();
    let position = Point::new(20.0, 20.0);
    runner.input.last_cursor = Some(position);
    runner.retain_native_mouse_device(native_device, None);

    let _pressed_a = runner.route_native_mouse_input_with_timestamp(
        MouseButton::Left,
        ElementState::Pressed,
        Some(InputTimestamp::capture()),
    );
    assert_eq!(runner.core.runtime.bridge().events.len(), 1);
    let token_a = runner.core.runtime.bridge().events[0]
        .sequence_token()
        .expect("native press should admit a token");
    let (device_a, contact_a) = runner
        .input
        .native_pointer_ingress
        .retain_mouse_contact(native_device)
        .expect("mouse contact should be retained");
    runner.queue_scrollbar_drag_with_metadata(
        Point::new(28.0, 20.0),
        PointerModifiers::default(),
        Some(InputTimestamp::capture()),
        None,
    );
    let pending = runner
        .input
        .pending_scrollbar_drag
        .expect("native move should be queued");
    let identity = pending
        .native_identity
        .expect("queued native identity should be retained atomically");
    assert_eq!(identity.token, token_a);
    assert_eq!(identity.device, device_a);
    assert_eq!(identity.contact, contact_a);
    // A known native stream with no admitted token cannot legacy-route or
    // replace the valid queued A sample.
    runner
        .input
        .native_pointer_ingress
        .clear_token_for_identity(device_a, contact_a);
    runner.queue_scrollbar_drag_with_metadata(
        Point::new(36.0, 20.0),
        PointerModifiers::default(),
        Some(InputTimestamp::capture()),
        None,
    );
    let pending = runner
        .input
        .pending_scrollbar_drag
        .expect("valid A sample remains queued");
    assert_eq!(pending.position, Point::new(28.0, 20.0));
    assert_eq!(runner.core.runtime.bridge().events.len(), 1);

    assert_eq!(
        runner.core.runtime.dispatch_native_pointer_continuation(
            DeviceKind::Mouse,
            device_a,
            contact_a,
            token_a,
            PointerPhase::Ended {
                button: PointerButton::Primary,
            },
            position,
            crate::gui::pointer_ingress::PointerButtons::empty(),
            PointerModifiers::default(),
            None,
            None,
            Some(InputTimestamp::capture()),
            None,
        ),
        PointerIngressDisposition::RoutedWidget(1)
    );
    assert!(matches!(
        runner.core.runtime.bridge().events[1].phase(),
        PointerPhase::Ended { .. }
    ));

    let _pressed_b = runner.route_native_mouse_input_with_timestamp(
        MouseButton::Left,
        ElementState::Pressed,
        Some(InputTimestamp::capture()),
    );
    let token_b = runner.core.runtime.bridge().events[2]
        .sequence_token()
        .expect("reused native contact should admit a fresh token");
    assert_ne!(token_a, token_b);

    runner.flush_pending_scrollbar_drag_now();
    assert_eq!(
        runner.core.runtime.bridge().events.len(),
        3,
        "delayed A move must be stale after B starts"
    );
    assert_eq!(
        runner.core.runtime.dispatch_native_pointer_continuation(
            DeviceKind::Mouse,
            device_a,
            contact_a,
            token_b,
            PointerPhase::Moved,
            Point::new(30.0, 20.0),
            crate::gui::pointer_ingress::PointerButtons::PRIMARY,
            PointerModifiers::default(),
            None,
            None,
            None,
            None,
        ),
        PointerIngressDisposition::RoutedWidget(1),
        "current B must remain routable after stale A flush"
    );
    assert_eq!(
        runner.core.runtime.dispatch_native_pointer_continuation(
            DeviceKind::Mouse,
            device_a,
            contact_a,
            token_a,
            PointerPhase::Ended {
                button: PointerButton::Primary,
            },
            position,
            crate::gui::pointer_ingress::PointerButtons::empty(),
            PointerModifiers::default(),
            None,
            None,
            None,
            None,
        ),
        PointerIngressDisposition::Stale
    );
    let _released_b = runner.route_native_mouse_input_with_timestamp(
        MouseButton::Left,
        ElementState::Released,
        Some(InputTimestamp::capture()),
    );
    assert_eq!(runner.core.runtime.bridge().events.len(), 5);
    assert_eq!(
        runner.core.runtime.dispatch_native_pointer_continuation(
            DeviceKind::Mouse,
            device_a,
            contact_a,
            token_b,
            PointerPhase::Ended {
                button: PointerButton::Primary,
            },
            position,
            crate::gui::pointer_ingress::PointerButtons::empty(),
            PointerModifiers::default(),
            None,
            None,
            None,
            None,
        ),
        PointerIngressDisposition::Stale
    );
    assert_eq!(runner.core.runtime.bridge().events.len(), 5);
}

#[test]
fn native_pointer_harness_exercises_gpu_hover_fast_path_before_press() {
    let mut harness =
        NativePointerHarness::new(GpuWheelBridge::default(), Vector2::new(320.0, 80.0));
    let point = Point::new(40.0, 20.0);

    assert!(harness.runner.can_fast_path_native_hover_move(point));
    harness.cursor_moved_logical(point);
    assert_eq!(harness.runner.input.last_cursor, Some(point));
    assert!(
        harness.runner.frame.composited_base_dirty,
        "native GPU hover fast path should update cached overlay state"
    );

    let pressed = harness.mouse_pressed(MouseButton::Left);

    assert!(
        pressed.routed,
        "press after native GPU hover fast path should still route through the runtime"
    );
}

#[test]
fn native_pointer_harness_focus_loss_clears_native_pointer_state() {
    let mut harness =
        NativePointerHarness::new(GpuWheelBridge::default(), Vector2::new(320.0, 80.0));
    harness.cursor_moved_logical(Point::new(40.0, 20.0));
    assert!(!harness.runner.input.native_cursor_visible);
    harness.modifiers_changed(ModifiersState::ALT);

    let outcome = harness.focus_lost();

    assert!(outcome.routed);
    assert_eq!(harness.runner.input.last_cursor, None);
    assert!(harness.runner.input.native_cursor_visible);
    assert!(harness.runner.input.modifiers.is_empty());
    harness.focus_regained();
}

#[test]
fn native_pointer_focus_loss_clears_retained_widget_hover() {
    let mut harness = NativePointerHarness::new(demo_bridge(), Vector2::new(320.0, 40.0));
    let button_point = harness
        .runner
        .core
        .runtime
        .layout()
        .rects
        .get(&11)
        .map(|rect| Point::new(rect.min.x + 4.0, rect.min.y + 4.0))
        .expect("button should be laid out");

    harness.cursor_moved_logical(button_point);
    assert_eq!(harness.runner.core.runtime.hovered_widget(), Some(11));
    assert!(
        harness
            .runner
            .core
            .runtime
            .surface()
            .find_widget(11)
            .expect("hovered button")
            .widget()
            .common()
            .state
            .hovered
    );

    let outcome = harness.focus_lost();

    assert!(outcome.needs_scene_rebuild());
    assert_eq!(harness.runner.input.last_cursor, None);
    assert_eq!(harness.runner.core.runtime.hovered_widget(), None);
    assert!(
        !harness
            .runner
            .core
            .runtime
            .surface()
            .find_widget(11)
            .expect("previous hovered button")
            .widget()
            .common()
            .state
            .hovered
    );
}

#[test]
fn external_drag_finalize_waits_for_transient_completion() {
    let mut runner = GenericNativeVelloRunner::new(
        NativeRunOptions::default(),
        GpuWheelBridge::default(),
        Vector2::new(240.0, 80.0),
    );
    runner.rebuild_scene();
    runner.input.last_cursor = Some(Point::new(60.0, 20.0));
    runner
        .core
        .runtime
        .execute_command(Command::begin_external_drag_without_completion(
            ExternalDragRequest::files(
                [std::path::PathBuf::from(r"C:\samples\kick.wav")],
                "kick.wav",
            ),
        ));
    let route = runner.route_cursor_left();
    assert_eq!(runner.input.last_cursor, None);
    assert!(route.launch_external_drag);

    let launch_calls = Cell::new(0);
    let rejected = finalize_native_immediate_transient_route(
        ImmediateTransientCompletion::Mismatch,
        route.outcome,
        route.launch_external_drag,
        || {
            launch_calls.set(launch_calls.get() + 1);
            GenericRouteOutcome::default()
        },
    );
    assert!(rejected.is_none());
    assert_eq!(launch_calls.get(), 0);
    assert!(runner.core.runtime.external_drag_armed());

    let mut local_route = GenericRouteOutcome::default();
    local_route.request_scene_rebuild(FrameWorkReason::ExternalDragPreview);
    let accepted = finalize_native_immediate_transient_route(
        ImmediateTransientCompletion::Completed(FrameStageBudgetStatus::Exceeded),
        local_route,
        true,
        || {
            launch_calls.set(launch_calls.get() + 1);
            GenericRouteOutcome::default()
        },
    )
    .expect("completed transient should publish its retained route");
    assert_eq!(launch_calls.get(), 1);
    assert_eq!(accepted.frame_work(), local_route.frame_work());
}

#[test]
fn native_mouse_drag_transfers_capture_and_retires_the_exact_contact_token() {
    use crate::{
        application::{DragSource, button},
        runtime::DragSourcePhase,
    };
    let events = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
    let output = events.clone();
    let bridge = crate::app(())
        .view(|_| {
            button("Drag")
                .filter_mapped(|_| None::<DragSourcePhase>)
                .size(100.0, 40.0)
                .id(1)
                .drag_source(
                    DragSource::new(42u32).on_event_with_revision((), |event| Some(event.phase())),
                )
                .id(10)
        })
        .update(move |_, event| output.borrow_mut().push(event))
        .into_bridge();
    let mut harness = NativePointerHarness::new(bridge, Vector2::new(240.0, 80.0));
    let device = DeviceId::dummy();
    harness.runner.retain_native_mouse_device(device, None);
    harness.cursor_moved_logical(Point::new(20.0, 15.0));
    assert!(harness.mouse_pressed(MouseButton::Left).routed);
    let original = harness
        .runner
        .input
        .native_pointer_ingress
        .contact_token(device, u64::MAX)
        .unwrap();
    assert!(events.borrow().is_empty());
    let physical = PhysicalPosition::new(
        harness.runner.window.dpi_scale.logical_to_physical(35.0) as f64,
        harness.runner.window.dpi_scale.logical_to_physical(15.0) as f64,
    );
    let moved = harness
        .runner
        .route_cursor_moved_with_timestamp(physical, InputTimestamp::capture());
    assert!(moved.outcome.routed);
    assert!(moved.outcome.needs_redraw());
    harness.runner.apply_cursor_moved_route(moved);
    assert_eq!(*events.borrow(), [DragSourcePhase::Started]);
    assert!(harness.runner.core.runtime.drag_session_active());
    assert_eq!(harness.runner.core.runtime.pointer_capture(), None);
    assert_eq!(
        harness
            .runner
            .input
            .native_pointer_ingress
            .contact_token(device, u64::MAX),
        Some(original)
    );
    assert!(harness.mouse_released(MouseButton::Left).routed);
    assert!(!harness.runner.core.runtime.drag_session_active());
    assert_eq!(
        harness
            .runner
            .input
            .native_pointer_ingress
            .contact_token(device, u64::MAX),
        None
    );
    harness.cursor_moved_logical(Point::new(20.0, 15.0));
    assert!(harness.mouse_pressed(MouseButton::Left).routed);
    let next = harness
        .runner
        .input
        .native_pointer_ingress
        .contact_token(device, u64::MAX)
        .unwrap();
    assert_ne!(original, next);
    assert!(harness.mouse_released(MouseButton::Left).routed);
    assert_eq!(
        events
            .borrow()
            .iter()
            .filter(|phase| matches!(phase, DragSourcePhase::Started))
            .count(),
        1
    );
}
