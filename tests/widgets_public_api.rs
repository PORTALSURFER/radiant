//! Public API coverage for `radiant::widgets`.

use radiant::{
    application::IntoView,
    gui::{svg::SvgIcon, types::ImageRgba},
    layout::{
        ContainerKind, ContainerPolicy, LayoutNode, Point, Rect, SlotChild, SlotParams, Vector2,
        layout_tree,
    },
    runtime::{SurfaceNode, UiSurface, WidgetMessageMapper},
    widgets::{
        BadgeWidget, BadgeWidgetParts, ButtonWidget, ButtonWidgetParts, CanvasWidget,
        CanvasWidgetParts, CardWidget, CardWidgetParts, DragHandleWidget, DragHandleWidgetParts,
        EditEvent, EditPhase, EditTransaction, IconButtonWidget, IconButtonWidgetParts,
        ImageWidget, ImageWidgetParts, InteractionProvenance, InteractionSource,
        InteractiveRowWidget, InteractiveRowWidgetParts, KeyboardModifier, KeyboardModifiers,
        KnobEditBatch, KnobMessage, KnobPointerMetadata, KnobState, KnobWidget, ListItemWidget,
        ListItemWidgetParts, NumericAdjustment, NumericCodec, NumericEditSession,
        NumericInputConstructionError, NumericInputEditBatch, NumericInputInteraction,
        NumericInputInteractionBatch, NumericParseResult, NumericScrubActivation,
        NumericScrubAttempt, NumericScrubPolicy, NumericStep, NumericStepAttempt,
        NumericStepDirection, NumericStepModifiers, PointerModifiers, ScrollbarAxis,
        ScrollbarWidget, ScrollbarWidgetParts, SelectableWidget, SelectableWidgetParts,
        SliderEditBatch, SliderMessage, SliderState, SliderWidget, SliderWidgetParts,
        TextInputWidget, TextInputWidgetParts, TextWidget, TextWidgetParts, ToggleWidget,
        ToggleWidgetParts, Widget, WidgetInput, WidgetKey, WidgetOutput, WidgetSizing,
        WidgetSizingParts,
    },
};
use std::{
    cell::{Cell, RefCell},
    fmt::{self, Debug},
    rc::Rc,
    sync::Arc,
};

#[path = "widgets_public_api/composition.rs"]
mod composition;
#[path = "widgets_public_api/construction.rs"]
mod construction;
#[path = "widgets_public_api/dispatch.rs"]
mod dispatch;

fn assert_typed_widget_output<T>(output: Option<WidgetOutput>, expected: T)
where
    T: Clone + Debug + PartialEq + 'static,
{
    let output = output.expect("widget should emit output");
    assert_eq!(output.typed_ref::<T>(), Some(&expected));
    assert_eq!(output.typed_cloned::<T>(), Some(expected));
}

#[test]
fn value_mapping_is_available_through_qualified_interaction_module() {
    let mapping = radiant::widgets::interaction::ValueMapping::linear(0.0..=1.0)
        .expect("valid linear mapping");
    let kind: radiant::widgets::interaction::ValueMappingKind = mapping.kind();
    let error: radiant::widgets::interaction::ValueMappingError =
        radiant::widgets::interaction::ValueMapping::linear(1.0..=1.0)
            .expect_err("equal bounds should be rejected");

    assert_eq!(
        kind,
        radiant::widgets::interaction::ValueMappingKind::Linear
    );
    assert_eq!(
        error,
        radiant::widgets::interaction::ValueMappingError::InvalidRange { min: 1.0, max: 1.0 }
    );
}

#[test]
fn exact_wheel_sample_types_are_qualified_and_preserve_units_and_phases() {
    let line_delta = radiant::widgets::interaction::WheelDelta::lines(Vector2::new(0.0, 1.0))
        .expect("finite line delta");
    let pixel_delta = radiant::widgets::interaction::WheelDelta::pixels(Vector2::new(0.0, 40.0))
        .expect("finite pixel delta");

    assert_eq!(
        line_delta,
        radiant::widgets::interaction::WheelDelta::Lines(Vector2::new(0.0, 1.0))
    );
    assert_eq!(
        pixel_delta,
        radiant::widgets::interaction::WheelDelta::Pixels(Vector2::new(0.0, 40.0))
    );
    assert_eq!(
        line_delta.to_logical_pixels(),
        Some(Vector2::new(
            0.0,
            radiant::widgets::WHEEL_LINE_EQUIVALENCE_PIXELS
        ))
    );

    for phase in [
        radiant::widgets::interaction::WheelPhase::Started,
        radiant::widgets::interaction::WheelPhase::Changed,
        radiant::widgets::interaction::WheelPhase::Ended,
        radiant::widgets::interaction::WheelPhase::Cancelled,
        radiant::widgets::interaction::WheelPhase::Discrete,
    ] {
        let sample = radiant::widgets::interaction::WheelSample::new(
            pixel_delta,
            Some(phase),
            PointerModifiers::default(),
        )
        .expect("finite wheel sample");
        assert_eq!(sample.delta(), pixel_delta);
        assert_eq!(sample.phase(), Some(phase));
    }
    assert_eq!(
        radiant::widgets::interaction::WheelSample::phase_less(
            pixel_delta,
            PointerModifiers::default(),
        )
        .expect("phase-less wheel sample")
        .phase(),
        None
    );
    assert!(
        radiant::widgets::interaction::WheelSample::new(
            radiant::widgets::interaction::WheelDelta::Lines(Vector2::new(f32::NAN, 0.0)),
            Some(radiant::widgets::interaction::WheelPhase::Started),
            PointerModifiers::default(),
        )
        .is_err()
    );
}

#[test]
fn value_format_is_available_through_qualified_and_widgets_root_exports() {
    let qualified = radiant::widgets::interaction::ValueFormat::frequency();
    let root: radiant::widgets::ValueFormat =
        qualified.with_decimal_separator(radiant::widgets::DecimalSeparator::Comma);
    let kind: radiant::widgets::interaction::ValueFormatKind = root.kind();
    let separator: radiant::widgets::DecimalSeparator = root.decimal_separator();
    let error: radiant::widgets::interaction::ValueFormatError =
        radiant::widgets::ValueFormatError::NonFiniteValue;

    assert_eq!(kind, radiant::widgets::ValueFormatKind::Frequency);
    assert_eq!(separator, radiant::widgets::DecimalSeparator::Comma);
    assert_eq!(
        error,
        radiant::widgets::interaction::ValueFormatError::NonFiniteValue
    );

    let mut output = String::new();
    root.write_into(440.0, &mut output)
        .expect("finite value should format");
    assert_eq!(output, "440,00 Hz");
}

#[derive(Clone, Debug, PartialEq)]
struct GenericNumericValue(u32);

#[derive(Debug, PartialEq)]
struct NonCloneNumericValue(i32);

#[derive(Debug, PartialEq)]
enum NumericCodecError {
    WriteFailed,
}

struct NonCloneNumericCodec;

impl NumericCodec<NonCloneNumericValue> for NonCloneNumericCodec {
    type Error = NumericCodecError;

    fn parse(&self, text: &str) -> NumericParseResult<NonCloneNumericValue> {
        match text {
            "" => NumericParseResult::Incomplete,
            "invalid" => NumericParseResult::Invalid,
            "-1" => NumericParseResult::OutOfRange,
            "7" => NumericParseResult::Valid(NonCloneNumericValue(7)),
            _ => NumericParseResult::Invalid,
        }
    }

    fn format_editable(
        &self,
        value: &NonCloneNumericValue,
        output: &mut dyn fmt::Write,
    ) -> Result<(), Self::Error> {
        write!(output, "{}", value.0).map_err(|_| NumericCodecError::WriteFailed)
    }
}

#[derive(Debug, PartialEq)]
struct NumericAdjustmentTestError;

#[derive(Debug, PartialEq)]
struct LocalAdjustmentError(Rc<Cell<usize>>);

#[derive(Debug, PartialEq)]
struct LocalCodecError(Rc<Cell<usize>>);

struct NonCloneNumericAdjustment;

impl NumericAdjustment<NonCloneNumericValue> for NonCloneNumericAdjustment {
    type Error = NumericAdjustmentTestError;

    fn normalized_to_value(&self, normalized: f32) -> Result<NonCloneNumericValue, Self::Error> {
        Ok(NonCloneNumericValue((normalized * 100.0) as i32))
    }

    fn value_to_normalized(&self, value: &NonCloneNumericValue) -> Result<f32, Self::Error> {
        Ok(value.0 as f32 / 100.0)
    }

    fn step(
        &self,
        value: &NonCloneNumericValue,
        direction: NumericStepDirection,
        step: NumericStep,
    ) -> Result<NonCloneNumericValue, Self::Error> {
        let amount = match step {
            NumericStep::Base => 1,
            NumericStep::Fine => 2,
            NumericStep::Coarse => 10,
        };
        let signed = match direction {
            NumericStepDirection::Decrease => -amount,
            NumericStepDirection::Increase => amount,
        };
        Ok(NonCloneNumericValue(value.0 + signed))
    }

    fn scrub(
        &self,
        value: &NonCloneNumericValue,
        normalized_delta: f32,
        _step: NumericStep,
    ) -> Result<NonCloneNumericValue, Self::Error> {
        Ok(NonCloneNumericValue(
            value.0 + (normalized_delta * 100.0) as i32,
        ))
    }

    fn wheel(
        &self,
        value: &NonCloneNumericValue,
        delta: f32,
        _step: NumericStep,
    ) -> Result<NonCloneNumericValue, Self::Error> {
        Ok(NonCloneNumericValue(value.0 + delta as i32))
    }
}

struct UiLocalNumericCodec(Rc<RefCell<usize>>);

impl NumericCodec<GenericNumericValue> for UiLocalNumericCodec {
    type Error = NumericCodecError;

    fn parse(&self, text: &str) -> NumericParseResult<GenericNumericValue> {
        match text {
            "7" => NumericParseResult::Valid(GenericNumericValue(7)),
            "8" => NumericParseResult::Valid(GenericNumericValue(8)),
            "" => NumericParseResult::Incomplete,
            _ => NumericParseResult::Invalid,
        }
    }

    fn format_editable(
        &self,
        value: &GenericNumericValue,
        output: &mut dyn fmt::Write,
    ) -> Result<(), Self::Error> {
        *self.0.borrow_mut() += 1;
        write!(output, "{}", value.0).map_err(|_| NumericCodecError::WriteFailed)
    }
}

struct UiLocalNumericAdjustment(Rc<RefCell<usize>>);

impl NumericAdjustment<GenericNumericValue> for UiLocalNumericAdjustment {
    type Error = NumericAdjustmentTestError;

    fn normalized_to_value(&self, normalized: f32) -> Result<GenericNumericValue, Self::Error> {
        Ok(GenericNumericValue(normalized as u32))
    }

    fn value_to_normalized(&self, value: &GenericNumericValue) -> Result<f32, Self::Error> {
        *self.0.borrow_mut() += 1;
        Ok(value.0 as f32)
    }

    fn step(
        &self,
        value: &GenericNumericValue,
        direction: NumericStepDirection,
        step: NumericStep,
    ) -> Result<GenericNumericValue, Self::Error> {
        let amount = match step {
            NumericStep::Base => 1,
            NumericStep::Fine => 2,
            NumericStep::Coarse => 10,
        };
        let value = match direction {
            NumericStepDirection::Decrease => value.0.saturating_sub(amount),
            NumericStepDirection::Increase => value.0.saturating_add(amount),
        };
        Ok(GenericNumericValue(value))
    }

    fn scrub(
        &self,
        value: &GenericNumericValue,
        _normalized_delta: f32,
        _step: NumericStep,
    ) -> Result<GenericNumericValue, Self::Error> {
        Ok(value.clone())
    }

    fn wheel(
        &self,
        value: &GenericNumericValue,
        _delta: f32,
        _step: NumericStep,
    ) -> Result<GenericNumericValue, Self::Error> {
        Ok(value.clone())
    }
}

struct LocalErrorNumericCodec;

impl NumericCodec<GenericNumericValue> for LocalErrorNumericCodec {
    type Error = LocalCodecError;

    fn parse(&self, text: &str) -> NumericParseResult<GenericNumericValue> {
        match text {
            "" => NumericParseResult::Incomplete,
            "7" => NumericParseResult::Valid(GenericNumericValue(7)),
            _ => NumericParseResult::Invalid,
        }
    }

    fn format_editable(
        &self,
        value: &GenericNumericValue,
        output: &mut dyn fmt::Write,
    ) -> Result<(), Self::Error> {
        write!(output, "{}", value.0).map_err(|_| LocalCodecError(Rc::new(Cell::new(0))))
    }
}

struct LocalErrorNumericAdjustment;

impl NumericAdjustment<GenericNumericValue> for LocalErrorNumericAdjustment {
    type Error = LocalAdjustmentError;

    fn normalized_to_value(&self, normalized: f32) -> Result<GenericNumericValue, Self::Error> {
        Ok(GenericNumericValue(normalized as u32))
    }

    fn value_to_normalized(&self, value: &GenericNumericValue) -> Result<f32, Self::Error> {
        Ok(value.0 as f32)
    }

    fn step(
        &self,
        value: &GenericNumericValue,
        _direction: NumericStepDirection,
        _step: NumericStep,
    ) -> Result<GenericNumericValue, Self::Error> {
        Ok(value.clone())
    }

    fn scrub(
        &self,
        value: &GenericNumericValue,
        _normalized_delta: f32,
        _step: NumericStep,
    ) -> Result<GenericNumericValue, Self::Error> {
        Ok(value.clone())
    }

    fn wheel(
        &self,
        value: &GenericNumericValue,
        _delta: f32,
        _step: NumericStep,
    ) -> Result<GenericNumericValue, Self::Error> {
        Ok(value.clone())
    }
}

#[test]
fn numeric_input_public_builder_is_generic_and_keeps_lifecycle_types_qualified() {
    let codec = UiLocalNumericCodec(Rc::new(RefCell::new(0)));
    let adjustment = UiLocalNumericAdjustment(Rc::new(RefCell::new(0)));
    let result: Result<
        radiant::application::NumericInputBuilder<
            GenericNumericValue,
            UiLocalNumericCodec,
            UiLocalNumericAdjustment,
        >,
        NumericInputConstructionError<NumericCodecError, NumericAdjustmentTestError>,
    > = radiant::application::numeric_input(GenericNumericValue(7), codec, adjustment);
    let builder = result
        .expect("generic public numeric input should construct")
        .step_modifiers(NumericStepModifiers::new(
            KeyboardModifier::Alt,
            KeyboardModifier::Control,
        ))
        .scrub_policy(radiant::widgets::NumericScrubPolicy::default());
    let _: fn(
        NumericInputEditBatch<GenericNumericValue>,
    ) -> NumericInputEditBatch<GenericNumericValue> = |batch| batch;
    let mut surface: radiant::runtime::UiSurface<NumericInputEditBatch<GenericNumericValue>> =
        builder.on_edit(|batch| batch).id(77).into_surface();
    let bounds = radiant::gui::types::Rect::from_min_size(
        radiant::gui::types::Point::default(),
        radiant::gui::types::Vector2::new(120.0, 28.0),
    );
    assert!(
        surface
            .dispatch_widget_input(77, bounds, WidgetInput::FocusChanged(true))
            .is_none()
    );
    assert!(
        surface
            .dispatch_widget_input(
                77,
                bounds,
                WidgetInput::text_edit(radiant::widgets::TextEditCommand::SelectAll),
            )
            .is_none()
    );
    assert!(
        surface
            .dispatch_widget_input(
                77,
                bounds,
                WidgetInput::text_edit(radiant::widgets::TextEditCommand::InsertText(
                    String::from("8"),
                )),
            )
            .is_none()
    );
    let output = surface
        .dispatch_widget_input(77, bounds, WidgetInput::key_press(WidgetKey::Enter))
        .and_then(|output| output.typed_cloned::<NumericInputEditBatch<GenericNumericValue>>())
        .expect("valid generic numeric edit should emit a typed terminal batch");
    assert_eq!(
        output
            .events()
            .iter()
            .map(|event| event.phase)
            .collect::<Vec<_>>(),
        [EditPhase::Begin, EditPhase::Commit]
    );
}

#[test]
fn numeric_input_on_interaction_keeps_error_order_without_thread_bounds() {
    let builder = radiant::application::numeric_input(
        GenericNumericValue(7),
        LocalErrorNumericCodec,
        LocalErrorNumericAdjustment,
    )
    .expect("local error policies should construct");
    let _: radiant::runtime::UiSurface<()> = builder
        .on_interaction(
            |batch: NumericInputInteractionBatch<
                GenericNumericValue,
                LocalAdjustmentError,
                LocalCodecError,
            >| {
                assert_eq!(batch.len(), 1);
            },
        )
        .id(78)
        .into_surface();
}

#[test]
fn numeric_input_on_interaction_maps_one_complete_text_edit_envelope() {
    type Batch = NumericInputInteractionBatch<
        GenericNumericValue,
        NumericAdjustmentTestError,
        NumericCodecError,
    >;

    let map_calls = Rc::new(Cell::new(0));
    let map_calls_for_mapper = Rc::clone(&map_calls);
    let mut surface: radiant::runtime::UiSurface<Batch> = radiant::application::numeric_input(
        GenericNumericValue(7),
        UiLocalNumericCodec(Rc::new(RefCell::new(0))),
        UiLocalNumericAdjustment(Rc::new(RefCell::new(0))),
    )
    .expect("generic numeric input should construct")
    .on_interaction(move |batch| {
        map_calls_for_mapper.set(map_calls_for_mapper.get() + 1);
        batch
    })
    .id(79)
    .into_surface();
    let bounds = radiant::gui::types::Rect::from_min_size(
        radiant::gui::types::Point::default(),
        radiant::gui::types::Vector2::new(120.0, 28.0),
    );

    assert!(
        surface
            .dispatch_widget_input(79, bounds, WidgetInput::FocusChanged(true))
            .is_none()
    );
    assert!(
        surface
            .dispatch_widget_input(
                79,
                bounds,
                WidgetInput::text_edit(radiant::widgets::TextEditCommand::SelectAll),
            )
            .is_none()
    );
    assert!(
        surface
            .dispatch_widget_input(
                79,
                bounds,
                WidgetInput::text_edit(radiant::widgets::TextEditCommand::InsertText(
                    String::from("8"),
                )),
            )
            .is_none()
    );

    let output = surface
        .dispatch_widget_input(79, bounds, WidgetInput::key_press(WidgetKey::Enter))
        .expect("complete mode should emit one raw interaction output");
    assert!(
        output
            .typed_ref::<NumericInputEditBatch<GenericNumericValue>>()
            .is_none()
    );
    let message = surface
        .dispatch_widget_output(79, output)
        .expect("complete output should map to one host message");
    assert_eq!(map_calls.get(), 1);
    assert_eq!(message.len(), 1);
    let [NumericInputInteraction::Edit(edit)] = message.parts() else {
        panic!("complete TextEdit output should contain one outer Edit");
    };
    assert_eq!(
        edit.events()
            .iter()
            .map(|event| event.phase)
            .collect::<Vec<_>>(),
        [EditPhase::Begin, EditPhase::Commit]
    );
    assert_eq!(edit.events()[0].transaction, edit.events()[1].transaction);
    assert_eq!(edit.events()[0].start_value, GenericNumericValue(7));
    assert_eq!(edit.events()[1].value, GenericNumericValue(8));
}

#[test]
fn numeric_input_on_interaction_maps_complete_keyboard_transaction_shapes() {
    type Batch = NumericInputInteractionBatch<
        GenericNumericValue,
        NumericAdjustmentTestError,
        NumericCodecError,
    >;

    let map_calls = Rc::new(Cell::new(0));
    let map_calls_for_mapper = Rc::clone(&map_calls);
    let mut surface: radiant::runtime::UiSurface<Batch> = radiant::application::numeric_input(
        GenericNumericValue(7),
        UiLocalNumericCodec(Rc::new(RefCell::new(0))),
        UiLocalNumericAdjustment(Rc::new(RefCell::new(0))),
    )
    .expect("generic numeric input should construct")
    .step_modifiers(NumericStepModifiers::new(
        KeyboardModifier::Shift,
        KeyboardModifier::Control,
    ))
    .on_interaction(move |batch| {
        map_calls_for_mapper.set(map_calls_for_mapper.get() + 1);
        batch
    })
    .id(82)
    .into_surface();
    let bounds = radiant::gui::types::Rect::from_min_size(
        radiant::gui::types::Point::default(),
        radiant::gui::types::Vector2::new(120.0, 28.0),
    );

    assert!(
        surface
            .dispatch_widget_input(82, bounds, WidgetInput::FocusChanged(true))
            .is_none()
    );

    let initial = surface
        .dispatch_widget_input(
            82,
            bounds,
            WidgetInput::KeyPress {
                key: WidgetKey::ArrowUp,
                modifiers: KeyboardModifiers::default(),
                repeat: false,
                timestamp: None,
            },
        )
        .expect("complete keyboard initial should emit raw output");
    let initial = surface
        .dispatch_widget_output(82, initial)
        .expect("complete keyboard initial should use the interaction mapper");
    let [NumericInputInteraction::Edit(edit)] = initial.parts() else {
        panic!("keyboard initial should map to one Edit part");
    };
    assert_eq!(
        edit.events()
            .iter()
            .map(|event| event.phase)
            .collect::<Vec<_>>(),
        [EditPhase::Begin, EditPhase::Update]
    );
    assert_eq!(edit.events()[1].value, GenericNumericValue(8));

    let repeat = surface
        .dispatch_widget_input(
            82,
            bounds,
            WidgetInput::KeyPress {
                key: WidgetKey::ArrowUp,
                modifiers: KeyboardModifiers {
                    shift: true,
                    ..KeyboardModifiers::default()
                },
                repeat: true,
                timestamp: None,
            },
        )
        .expect("complete keyboard repeat should emit raw output");
    let repeat = surface
        .dispatch_widget_output(82, repeat)
        .expect("complete keyboard repeat should use the interaction mapper");
    let [NumericInputInteraction::Edit(edit)] = repeat.parts() else {
        panic!("keyboard repeat should map to one Edit part");
    };
    assert_eq!(edit.events().len(), 1);
    assert_eq!(edit.events()[0].phase, EditPhase::Update);
    assert_eq!(edit.events()[0].value, GenericNumericValue(10));

    let release = surface
        .dispatch_widget_input(82, bounds, WidgetInput::key_release(WidgetKey::ArrowUp))
        .expect("matching keyboard release should emit raw output");
    let release = surface
        .dispatch_widget_output(82, release)
        .expect("complete keyboard release should use the interaction mapper");
    let [NumericInputInteraction::Edit(edit)] = release.parts() else {
        panic!("keyboard release should map to one Edit part");
    };
    assert_eq!(edit.events().len(), 1);
    assert_eq!(edit.events()[0].phase, EditPhase::Commit);
    assert_eq!(edit.events()[0].value, GenericNumericValue(10));
    assert_eq!(map_calls.get(), 3);
}

#[test]
fn numeric_input_on_interaction_maps_focus_loss_commit_once() {
    type Batch = NumericInputInteractionBatch<
        GenericNumericValue,
        NumericAdjustmentTestError,
        NumericCodecError,
    >;

    let mut surface: radiant::runtime::UiSurface<Batch> = radiant::application::numeric_input(
        GenericNumericValue(7),
        UiLocalNumericCodec(Rc::new(RefCell::new(0))),
        UiLocalNumericAdjustment(Rc::new(RefCell::new(0))),
    )
    .expect("generic numeric input should construct")
    .on_interaction(|batch| batch)
    .id(80)
    .into_surface();
    let bounds = radiant::gui::types::Rect::from_min_size(
        radiant::gui::types::Point::default(),
        radiant::gui::types::Vector2::new(120.0, 28.0),
    );

    assert!(
        surface
            .dispatch_widget_input(80, bounds, WidgetInput::FocusChanged(true))
            .is_none()
    );
    assert!(
        surface
            .dispatch_widget_input(
                80,
                bounds,
                WidgetInput::text_edit(radiant::widgets::TextEditCommand::SelectAll),
            )
            .is_none()
    );
    assert!(
        surface
            .dispatch_widget_input(
                80,
                bounds,
                WidgetInput::text_edit(radiant::widgets::TextEditCommand::InsertText(
                    String::from("8"),
                )),
            )
            .is_none()
    );

    let output = surface
        .dispatch_widget_input(80, bounds, WidgetInput::FocusChanged(false))
        .expect("valid focus loss should emit one complete output");
    let message = surface
        .dispatch_widget_output(80, output)
        .expect("focus-loss output should map once");
    let [NumericInputInteraction::Edit(edit)] = message.parts() else {
        panic!("focus-loss output should contain one outer Edit");
    };
    assert_eq!(
        edit.events()
            .iter()
            .map(|event| event.phase)
            .collect::<Vec<_>>(),
        [EditPhase::Begin, EditPhase::Commit]
    );
}

#[test]
fn numeric_input_on_interaction_preserves_synthetic_typed_failures() {
    type Batch = NumericInputInteractionBatch<
        GenericNumericValue,
        NumericAdjustmentTestError,
        NumericCodecError,
    >;
    type Interaction =
        NumericInputInteraction<GenericNumericValue, NumericAdjustmentTestError, NumericCodecError>;

    let surface: radiant::runtime::UiSurface<Batch> = radiant::application::numeric_input(
        GenericNumericValue(7),
        UiLocalNumericCodec(Rc::new(RefCell::new(0))),
        UiLocalNumericAdjustment(Rc::new(RefCell::new(0))),
    )
    .expect("generic numeric input should construct")
    .on_interaction(|batch| batch)
    .id(81)
    .into_surface();
    let keyboard = InteractionProvenance::Keyboard { timestamp: None };
    let step = Interaction::step_failed(
        NumericStepAttempt::Initial,
        NumericStepDirection::Increase,
        NumericStep::Base,
        keyboard,
        NumericAdjustmentTestError,
        false,
    );
    let format = Interaction::format_failed(
        NumericStepAttempt::Initial,
        NumericStepDirection::Decrease,
        NumericStep::Fine,
        keyboard,
        NumericCodecError::WriteFailed,
        false,
    );
    let step_batch = Batch::from_interactions(&[step])
        .expect("synthetic step failure should retain its exact envelope");
    let format_batch = Batch::from_interactions(&[format])
        .expect("synthetic format failure should retain its exact envelope");

    let mapped_step = surface
        .dispatch_widget_output(81, WidgetOutput::typed(step_batch))
        .expect("synthetic step failure should use the complete mapper");
    assert_eq!(
        mapped_step.parts()[0].step_error(),
        Some(&NumericAdjustmentTestError)
    );
    assert!(mapped_step.parts()[0].format_error().is_none());

    let mapped_format = surface
        .dispatch_widget_output(81, WidgetOutput::typed(format_batch))
        .expect("synthetic format failure should use the complete mapper");
    assert_eq!(
        mapped_format.parts()[0].format_error(),
        Some(&NumericCodecError::WriteFailed)
    );
    assert!(mapped_format.parts()[0].step_error().is_none());
}

#[test]
fn numeric_input_edit_batch_accepts_only_legal_lifecycle_fragments() {
    let provenance = InteractionProvenance::Programmatic;
    let begin = EditEvent::begin(GenericNumericValue(7), provenance);
    let update = begin
        .clone()
        .update(GenericNumericValue(8), provenance)
        .expect("matching source should update");
    let commit = begin
        .clone()
        .commit(GenericNumericValue(9), provenance)
        .expect("matching source should commit");
    let cancel = begin
        .clone()
        .cancel(provenance)
        .expect("matching source should cancel");

    let assert_round_trip = |events: &[EditEvent<GenericNumericValue>]| {
        let batch = NumericInputEditBatch::from_events(events)
            .expect("legal numeric edit fragment should be accepted");
        assert_eq!(batch.events(), events);
        assert_eq!(batch.len(), events.len());
        assert_eq!(batch.transaction(), events[0].transaction);
        assert!(!batch.is_empty());
    };

    assert_round_trip(std::slice::from_ref(&update));
    assert_round_trip(std::slice::from_ref(&commit));
    assert_round_trip(std::slice::from_ref(&cancel));
    assert_round_trip(&[begin.clone(), update.clone()]);
    assert_round_trip(&[begin.clone(), commit.clone()]);
    assert_round_trip(&[begin.clone(), cancel.clone()]);

    assert_eq!(NumericInputEditBatch::<GenericNumericValue>::MAX_EVENTS, 2);
    let terminal_batch = NumericInputEditBatch::terminal(begin.clone(), commit.clone())
        .expect("Begin followed by Commit should remain accepted by terminal");
    assert_eq!(terminal_batch.events(), &[begin.clone(), commit.clone()]);
    assert!(NumericInputEditBatch::terminal(begin.clone(), update.clone()).is_none());

    let other_begin = EditEvent::begin(GenericNumericValue(7), provenance);
    let other_update = other_begin
        .clone()
        .update(GenericNumericValue(8), provenance)
        .expect("matching source should update");
    let other_commit = other_begin
        .clone()
        .commit(GenericNumericValue(9), provenance)
        .expect("matching source should commit");
    let other_cancel = other_begin
        .clone()
        .cancel(provenance)
        .expect("matching source should cancel");

    let assert_rejected = |events: &[EditEvent<GenericNumericValue>]| {
        assert!(
            NumericInputEditBatch::from_events(events).is_none(),
            "illegal numeric edit fragment was accepted"
        );
    };

    assert_rejected(&[]);
    assert_rejected(std::slice::from_ref(&begin));
    assert_rejected(&[begin.clone(), begin.clone()]);
    assert_rejected(&[begin.clone(), other_update.clone()]);
    assert_rejected(&[begin.clone(), other_commit.clone()]);
    assert_rejected(&[begin.clone(), other_cancel.clone()]);
    assert_rejected(&[update.clone(), begin.clone()]);
    assert_rejected(&[commit.clone(), begin.clone()]);
    assert_rejected(&[cancel.clone(), begin.clone()]);
    assert_rejected(&[update.clone(), update.clone()]);
    assert_rejected(&[update.clone(), commit.clone()]);
    assert_rejected(&[update.clone(), cancel.clone()]);
    assert_rejected(&[commit.clone(), update.clone()]);
    assert_rejected(&[commit.clone(), commit.clone()]);
    assert_rejected(&[commit.clone(), cancel.clone()]);
    assert_rejected(&[cancel.clone(), update.clone()]);
    assert_rejected(&[cancel.clone(), commit.clone()]);
    assert_rejected(&[cancel.clone(), cancel.clone()]);
    assert_rejected(&[begin.clone(), update.clone(), commit.clone()]);
}

#[test]
fn numeric_input_interaction_batch_accepts_keyboard_and_text_edit_envelope_shapes() {
    type Interaction =
        NumericInputInteraction<GenericNumericValue, NumericAdjustmentTestError, NumericCodecError>;
    type Batch = NumericInputInteractionBatch<
        GenericNumericValue,
        NumericAdjustmentTestError,
        NumericCodecError,
    >;

    fn assert_clone<T: Clone>() {}

    assert_clone::<Interaction>();
    assert_clone::<Batch>();
    let _: Option<
        radiant::widgets::interaction::NumericInputInteraction<
            GenericNumericValue,
            NumericAdjustmentTestError,
            NumericCodecError,
        >,
    > = None;
    let _: Option<
        radiant::widgets::interaction::NumericInputInteractionBatch<
            GenericNumericValue,
            NumericAdjustmentTestError,
            NumericCodecError,
        >,
    > = None;

    let keyboard = InteractionProvenance::Keyboard { timestamp: None };
    let programmatic = InteractionProvenance::Programmatic;
    let begin = EditEvent::begin(GenericNumericValue(7), keyboard);
    let update = begin
        .clone()
        .update(GenericNumericValue(8), keyboard)
        .expect("matching source should update");
    let commit = update
        .clone()
        .commit(GenericNumericValue(9), keyboard)
        .expect("matching source should commit");
    let cancel = update
        .clone()
        .cancel(keyboard)
        .expect("matching source should cancel");

    let edit = |events: &[EditEvent<GenericNumericValue>]| -> Interaction {
        Interaction::edit(
            NumericInputEditBatch::from_events(events)
                .expect("underlying edit fragment should be legal"),
        )
    };
    let begin_update = edit(&[begin.clone(), update.clone()]);
    let update_edit = edit(std::slice::from_ref(&update));
    let commit_edit = edit(std::slice::from_ref(&commit));
    let cancel_edit = edit(std::slice::from_ref(&cancel));
    let begin_commit = edit(&[begin.clone(), commit.clone()]);
    let begin_cancel = edit(&[begin.clone(), cancel.clone()]);

    let initial_step = Interaction::step_failed(
        NumericStepAttempt::Initial,
        NumericStepDirection::Increase,
        NumericStep::Base,
        keyboard,
        NumericAdjustmentTestError,
        false,
    );
    let initial_format = Interaction::format_failed(
        NumericStepAttempt::Initial,
        NumericStepDirection::Increase,
        NumericStep::Base,
        keyboard,
        NumericCodecError::WriteFailed,
        false,
    );
    assert_eq!(initial_step.step_error(), Some(&NumericAdjustmentTestError));
    assert!(initial_step.format_error().is_none());
    assert_eq!(
        initial_format.format_error(),
        Some(&NumericCodecError::WriteFailed)
    );
    assert!(initial_format.step_error().is_none());

    let repeat_step = Interaction::step_failed(
        NumericStepAttempt::Repeat,
        NumericStepDirection::Increase,
        NumericStep::Fine,
        keyboard,
        NumericAdjustmentTestError,
        true,
    );
    let repeat_format = Interaction::format_failed(
        NumericStepAttempt::Repeat,
        NumericStepDirection::Increase,
        NumericStep::Fine,
        keyboard,
        NumericCodecError::WriteFailed,
        true,
    );
    let rollback = cancel_edit.clone();

    let assert_legal = |parts: &[Interaction]| {
        let batch = Batch::from_interactions(parts)
            .expect("legal keyboard interaction envelope should be accepted");
        assert_eq!(batch.parts(), parts);
        assert_eq!(batch.events(), parts);
        assert_eq!(batch.len(), parts.len());
        assert!(!batch.is_empty());
    };

    assert_eq!(Batch::MAX_INTERACTIONS, 2);
    assert_legal(std::slice::from_ref(&begin_update));
    assert_legal(std::slice::from_ref(&update_edit));
    assert_legal(std::slice::from_ref(&commit_edit));
    assert_legal(std::slice::from_ref(&cancel_edit));
    assert_legal(std::slice::from_ref(&initial_step));
    assert_legal(std::slice::from_ref(&initial_format));
    assert_legal(std::slice::from_ref(&begin_commit));
    assert_legal(std::slice::from_ref(&begin_cancel));
    assert_legal(&[rollback.clone(), repeat_step.clone()]);
    assert_legal(&[rollback.clone(), repeat_format.clone()]);

    let initial_cancelled = Interaction::step_failed(
        NumericStepAttempt::Initial,
        NumericStepDirection::Increase,
        NumericStep::Base,
        keyboard,
        NumericAdjustmentTestError,
        true,
    );
    let repeat_not_cancelled = Interaction::step_failed(
        NumericStepAttempt::Repeat,
        NumericStepDirection::Increase,
        NumericStep::Base,
        keyboard,
        NumericAdjustmentTestError,
        false,
    );
    let programmatic_begin = EditEvent::begin(GenericNumericValue(7), programmatic);
    let programmatic_update = programmatic_begin
        .clone()
        .update(GenericNumericValue(8), programmatic)
        .expect("matching source should update");
    let programmatic_cancel = programmatic_update
        .clone()
        .cancel(programmatic)
        .expect("matching source should cancel");
    let programmatic_begin_update = edit(&[programmatic_begin, programmatic_update]);
    let programmatic_rollback = edit(std::slice::from_ref(&programmatic_cancel));
    let programmatic_edit = edit(std::slice::from_ref(&programmatic_cancel));
    let programmatic_begin_for_terminal = EditEvent::begin(GenericNumericValue(7), programmatic);
    let programmatic_begin_cancel = programmatic_begin_for_terminal
        .clone()
        .cancel(programmatic)
        .expect("matching source should cancel");
    let programmatic_terminal = edit(&[programmatic_begin_for_terminal, programmatic_begin_cancel]);
    let programmatic_failure = Interaction::step_failed(
        NumericStepAttempt::Initial,
        NumericStepDirection::Increase,
        NumericStep::Base,
        programmatic,
        NumericAdjustmentTestError,
        false,
    );
    let mismatched_repeat = Interaction::step_failed(
        NumericStepAttempt::Repeat,
        NumericStepDirection::Increase,
        NumericStep::Fine,
        programmatic,
        NumericAdjustmentTestError,
        true,
    );

    let assert_rejected = |parts: &[Interaction]| {
        assert!(
            Batch::from_interactions(parts).is_none(),
            "illegal keyboard interaction envelope was accepted"
        );
    };

    assert_rejected(&[]);
    assert_rejected(&[rollback.clone(), repeat_step.clone(), repeat_format.clone()]);
    assert_rejected(&[initial_step.clone(), rollback.clone()]);
    assert_rejected(&[repeat_step.clone(), rollback.clone()]);
    assert_rejected(&[initial_step.clone(), initial_format.clone()]);
    assert_rejected(&[repeat_step.clone(), repeat_format.clone()]);
    assert_rejected(std::slice::from_ref(&repeat_step));
    assert_rejected(std::slice::from_ref(&repeat_format));
    assert_rejected(std::slice::from_ref(&initial_cancelled));
    assert_rejected(std::slice::from_ref(&repeat_not_cancelled));
    assert_rejected(&[rollback.clone(), initial_step.clone()]);
    assert_rejected(&[rollback.clone(), initial_format.clone()]);
    assert_rejected(&[update_edit.clone(), repeat_step.clone()]);
    assert_rejected(&[begin_update.clone(), repeat_step.clone()]);
    assert_rejected(&[commit_edit.clone(), repeat_step.clone()]);
    assert_rejected(&[begin_commit.clone(), repeat_step.clone()]);
    assert_rejected(&[begin_cancel.clone(), repeat_step.clone()]);
    assert_rejected(&[begin_update.clone(), update_edit.clone()]);
    assert_rejected(&[rollback.clone(), rollback.clone()]);
    assert_rejected(std::slice::from_ref(&programmatic_edit));
    assert_rejected(std::slice::from_ref(&programmatic_begin_update));
    assert_rejected(std::slice::from_ref(&programmatic_terminal));
    assert_rejected(std::slice::from_ref(&programmatic_failure));
    assert_rejected(&[programmatic_rollback.clone(), repeat_step.clone()]);
    assert_rejected(&[rollback, mismatched_repeat]);
}

#[test]
fn numeric_input_pointer_scrub_policy_and_failures_are_qualified_and_fixed_capacity() {
    type Interaction =
        NumericInputInteraction<GenericNumericValue, NumericAdjustmentTestError, NumericCodecError>;
    type Batch = NumericInputInteractionBatch<
        GenericNumericValue,
        NumericAdjustmentTestError,
        NumericCodecError,
    >;

    fn assert_clone<T: Clone>() {}
    fn assert_debug<T: std::fmt::Debug>() {}
    fn assert_eq_hash<T: Eq + std::hash::Hash>() {}
    assert_clone::<Interaction>();
    assert_clone::<Batch>();

    let qualified = radiant::widgets::interaction::NumericScrubPolicy::default();
    let root: NumericScrubPolicy = qualified;
    assert_eq!(
        root.activation(),
        NumericScrubActivation::PrimaryButtonHorizontalDrag {
            modifier: KeyboardModifier::Alt,
        }
    );
    let explicit = NumericScrubPolicy::new(NumericScrubActivation::PrimaryButtonHorizontalDrag {
        modifier: KeyboardModifier::Command,
    });
    assert_eq!(
        explicit.activation(),
        NumericScrubActivation::PrimaryButtonHorizontalDrag {
            modifier: KeyboardModifier::Command,
        }
    );
    assert_debug::<NumericScrubPolicy>();
    assert_eq_hash::<NumericScrubPolicy>();

    let pointer = InteractionProvenance::Pointer {
        modifiers: PointerModifiers {
            alt: true,
            ..PointerModifiers::default()
        },
        timestamp: None,
        sequence_range: None,
    };
    let begin = EditEvent::begin(GenericNumericValue(7), pointer);
    let update = begin
        .clone()
        .update(GenericNumericValue(8), pointer)
        .expect("pointer source should update");
    let cancel = begin
        .clone()
        .cancel(pointer)
        .expect("pointer source should cancel");
    let edit = |events: &[EditEvent<GenericNumericValue>]| -> Interaction {
        Interaction::edit(
            NumericInputEditBatch::from_events(events)
                .expect("pointer edit fragment should be legal"),
        )
    };
    let begin_update = edit(&[begin.clone(), update.clone()]);
    let update_edit = edit(std::slice::from_ref(&update));
    let cancel_edit = edit(std::slice::from_ref(&cancel));
    let initial_scrub = Interaction::scrub_failed(
        NumericScrubAttempt::Initial,
        0.25,
        NumericStep::Base,
        pointer,
        NumericAdjustmentTestError,
        false,
    );
    let initial_format = Interaction::pointer_format_failed(
        NumericScrubAttempt::Initial,
        0.25,
        NumericStep::Fine,
        pointer,
        NumericCodecError::WriteFailed,
        false,
    );
    let active_scrub = Interaction::scrub_failed(
        NumericScrubAttempt::Update,
        -0.5,
        NumericStep::Coarse,
        pointer,
        NumericAdjustmentTestError,
        true,
    );
    assert_eq!(
        initial_scrub.scrub_error(),
        Some(&NumericAdjustmentTestError)
    );
    assert!(initial_scrub.format_error().is_none());
    assert_eq!(
        initial_format.pointer_format_error(),
        Some(&NumericCodecError::WriteFailed)
    );
    assert!(active_scrub.format_error().is_none());

    assert_eq!(Batch::MAX_INTERACTIONS, 2);
    for parts in [
        vec![begin_update.clone()],
        vec![update_edit.clone()],
        vec![cancel_edit.clone()],
        vec![initial_scrub.clone()],
        vec![initial_format.clone()],
        vec![cancel_edit.clone(), active_scrub.clone()],
    ] {
        let batch = Batch::from_interactions(&parts).expect("pointer envelope should be legal");
        assert_eq!(batch.parts(), parts.as_slice());
    }
    assert!(Batch::from_interactions(&[active_scrub]).is_none());

    let prelude_widgets = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/prelude/widgets.rs"
    ));
    let prelude_controls = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/prelude/application/controls.rs"
    ));
    for source in [prelude_widgets, prelude_controls] {
        assert!(!source.contains("NumericScrubPolicy"));
        assert!(!source.contains("NumericScrubAttempt"));
        assert!(!source.contains("NumericScrubActivation"));
    }
}

#[test]
fn numeric_input_policy_and_batch_types_are_not_in_the_common_prelude() {
    let prelude_controls = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/prelude/application/controls.rs"
    ));
    let prelude_widgets = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/prelude/widgets.rs"
    ));
    for source in [prelude_controls, prelude_widgets] {
        assert!(!source.contains("numeric_input"));
        assert!(!source.contains("NumericInputBuilder"));
        assert!(!source.contains("NumericInputEditBatch"));
        assert!(!source.contains("NumericInputInteraction"));
        assert!(!source.contains("NumericInputInteractionBatch"));
        assert!(!source.contains("NumericInputConstructionError"));
        assert!(!source.contains("NumericScrubPolicy"));
        assert!(!source.contains("NumericScrubAttempt"));
        assert!(!source.contains("NumericScrubActivation"));
        assert!(!source.contains("NumericCodec"));
        assert!(!source.contains("NumericAdjustment"));
        assert!(!source.contains("NumericStepAttempt"));
    }
}

#[test]
fn numeric_edit_session_can_be_named_with_a_non_clone_type() {
    let _: Option<NumericEditSession<NonCloneNumericValue>> = None;
}

#[test]
fn numeric_edit_session_is_available_through_qualified_and_widgets_root_exports() {
    let provenance = radiant::widgets::InteractionProvenance::Keyboard { timestamp: None };
    let qualified = radiant::widgets::interaction::NumericEditSession::begin(
        GenericNumericValue(7),
        "7",
        provenance,
    );
    let mut root: NumericEditSession<GenericNumericValue> = qualified;

    root.replace_draft("1e");
    assert_eq!(root.draft(), "1e");
    assert_eq!(root.begin_event().value, GenericNumericValue(7));
}

#[test]
fn numeric_edit_session_accepts_a_generic_domain_value_without_numeric_policy() {
    let provenance = radiant::widgets::InteractionProvenance::Programmatic;
    let session = NumericEditSession::begin(GenericNumericValue(7), "invalid", provenance);
    let event = match session.commit(GenericNumericValue(u32::MAX), provenance) {
        Ok(event) => event,
        Err(_) => panic!("matching source should commit a caller-certified value"),
    };

    assert_eq!(event.value, GenericNumericValue(u32::MAX));
    assert_eq!(event.phase, radiant::widgets::EditPhase::Commit);

    let prelude_widgets = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/prelude/widgets.rs"
    ));
    assert!(!prelude_widgets.contains("NumericEditSession"));
}

#[test]
fn numeric_codec_is_qualified_generic_and_supports_non_clone_domain_values() {
    let codec = NonCloneNumericCodec;
    let _: &dyn radiant::widgets::interaction::NumericCodec<
        NonCloneNumericValue,
        Error = NumericCodecError,
    > = &codec;
    let _: radiant::widgets::interaction::NumericParseResult<NonCloneNumericValue> =
        codec.parse("");

    assert_eq!(codec.parse(""), NumericParseResult::Incomplete);
    assert_eq!(codec.parse("invalid"), NumericParseResult::Invalid);
    assert_eq!(codec.parse("-1"), NumericParseResult::OutOfRange);
    assert_eq!(
        codec.parse("7"),
        NumericParseResult::Valid(NonCloneNumericValue(7))
    );

    let value = NonCloneNumericValue(42);
    let mut output = String::new();
    codec
        .format_editable(&value, &mut output)
        .expect("caller-owned writer should receive canonical text");
    assert_eq!(output, "42");

    struct FailingWriter;
    impl fmt::Write for FailingWriter {
        fn write_str(&mut self, _: &str) -> fmt::Result {
            Err(fmt::Error)
        }
    }

    assert_eq!(
        codec.format_editable(&value, &mut FailingWriter),
        Err(NumericCodecError::WriteFailed)
    );

    let prelude_widgets = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/prelude/widgets.rs"
    ));
    assert!(!prelude_widgets.contains("NumericCodec"));
    assert!(!prelude_widgets.contains("NumericParseResult"));
}

#[test]
fn numeric_adjustment_is_qualified_generic_and_supports_non_clone_domain_values() {
    let adjustment = NonCloneNumericAdjustment;
    let _: &dyn radiant::widgets::interaction::NumericAdjustment<
        NonCloneNumericValue,
        Error = NumericAdjustmentTestError,
    > = &adjustment;

    let _: radiant::widgets::interaction::NumericStep = NumericStep::Base;
    let _: radiant::widgets::interaction::NumericStepDirection = NumericStepDirection::Decrease;
    let _: &dyn radiant::widgets::NumericAdjustment<
        NonCloneNumericValue,
        Error = NumericAdjustmentTestError,
    > = &adjustment;

    assert_eq!(NumericStep::Base, NumericStep::Base);
    assert_eq!(NumericStep::Fine, NumericStep::Fine);
    assert_eq!(NumericStep::Coarse, NumericStep::Coarse);
    assert_eq!(
        NumericStepDirection::Decrease,
        NumericStepDirection::Decrease
    );
    assert_eq!(
        NumericStepDirection::Increase,
        NumericStepDirection::Increase
    );
    assert_eq!(
        adjustment.value_to_normalized(&NonCloneNumericValue(25)),
        Ok(0.25)
    );
    assert_eq!(
        adjustment.step(
            &NonCloneNumericValue(10),
            NumericStepDirection::Increase,
            NumericStep::Coarse,
        ),
        Ok(NonCloneNumericValue(20))
    );

    let prelude_widgets = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/prelude/widgets.rs"
    ));
    assert!(!prelude_widgets.contains("NumericAdjustment"));
    assert!(!prelude_widgets.contains("NumericStep"));
}

#[test]
fn keyboard_modifier_payload_is_qualified_and_not_in_prelude() {
    let qualified = radiant::widgets::interaction::KeyboardModifiers {
        command: true,
        control: true,
        shift: true,
        alt: false,
    };
    let root: KeyboardModifiers = qualified;

    assert!(root.command);
    assert!(root.control);
    assert!(root.shift);
    assert!(!root.alt);
    assert!(!KeyboardModifiers::default().command);

    let prelude_widgets = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/prelude/widgets.rs"
    ));
    assert!(!prelude_widgets.contains("KeyboardModifiers"));
}

#[test]
fn numeric_step_modifiers_are_public_qualified_and_not_in_prelude() {
    fn assert_copy<T: Copy>() {}
    fn assert_debug<T: std::fmt::Debug>() {}
    fn assert_eq_hash<T: Eq + std::hash::Hash>() {}

    assert_copy::<KeyboardModifier>();
    assert_copy::<NumericStepModifiers>();
    assert_debug::<KeyboardModifier>();
    assert_debug::<NumericStepModifiers>();
    assert_eq_hash::<KeyboardModifier>();
    assert_eq_hash::<NumericStepModifiers>();

    let qualified_modifier: radiant::widgets::interaction::KeyboardModifier =
        radiant::widgets::interaction::KeyboardModifier::Alt;
    let root_modifier: radiant::widgets::KeyboardModifier = qualified_modifier;
    let qualified_policy = radiant::widgets::interaction::NumericStepModifiers::new(
        radiant::widgets::interaction::KeyboardModifier::Shift,
        radiant::widgets::interaction::KeyboardModifier::Command,
    );
    let root_policy: radiant::widgets::NumericStepModifiers = qualified_policy;

    assert_eq!(root_modifier, KeyboardModifier::Alt);
    assert_eq!(root_policy.fine(), KeyboardModifier::Shift);
    assert_eq!(root_policy.coarse(), KeyboardModifier::Command);
    assert_eq!(
        radiant::widgets::NumericStepModifiers::MACOS_DEFAULT,
        root_policy
    );
    assert_eq!(
        radiant::widgets::NumericStepModifiers::WINDOWS_LINUX_DEFAULT.coarse(),
        KeyboardModifier::Control
    );
    assert_eq!(
        root_policy.select_step(KeyboardModifiers {
            command: true,
            control: false,
            shift: true,
            alt: false,
        }),
        NumericStep::Fine
    );
    assert_eq!(
        root_policy.select_step(KeyboardModifiers {
            command: true,
            control: false,
            shift: false,
            alt: false,
        }),
        NumericStep::Coarse
    );
    assert_eq!(
        root_policy.select_step(KeyboardModifiers::default()),
        NumericStep::Base
    );

    let prelude_widgets = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/prelude/widgets.rs"
    ));
    assert!(!prelude_widgets.contains("KeyboardModifier"));
    assert!(!prelude_widgets.contains("NumericStepModifiers"));
}

#[test]
fn key_release_widget_input_constructor_is_public() {
    assert_eq!(
        WidgetInput::key_release(WidgetKey::ArrowDown),
        WidgetInput::KeyRelease {
            key: WidgetKey::ArrowDown,
            modifiers: KeyboardModifiers::default(),
            timestamp: None,
        }
    );
}

#[test]
fn widget_output_exposes_typed_and_custom_value_helpers() {
    let copied = WidgetOutput::typed(42_u8);
    assert_eq!(copied.typed_ref::<u8>(), Some(&42));
    assert_eq!(copied.typed_copied::<u8>(), Some(42));
    assert_eq!(copied.custom_copied::<u8>(), Some(42));

    let cloned = WidgetOutput::custom(String::from("activated"));
    assert_eq!(
        cloned.custom_ref::<String>().map(String::as_str),
        Some("activated")
    );
    assert_eq!(
        cloned.typed_cloned::<String>(),
        Some(String::from("activated"))
    );
    assert_eq!(
        cloned.custom_cloned::<String>(),
        Some(String::from("activated"))
    );
}

#[test]
fn widget_output_supports_ui_local_payloads_and_clone_identity() {
    let payload = Rc::new(RefCell::new(3usize));
    let output = WidgetOutput::typed(Rc::clone(&payload));

    assert_eq!(output, output.clone());
    assert_ne!(output, WidgetOutput::typed(Rc::clone(&payload)));
    assert!(Rc::ptr_eq(
        output.typed_ref::<Rc<RefCell<usize>>>().expect("payload"),
        &payload,
    ));
}

#[test]
fn slider_public_contract_keeps_bare_struct_literals_and_concise_output() {
    let bounds = Rect::from_min_size(Point::default(), Vector2::new(120.0, 28.0));
    let template = SliderWidget::new(43, 0.25, WidgetSizing::fixed(Vector2::new(120.0, 28.0)));
    let SliderWidget { common, props, .. } = template;
    let mut slider = SliderWidget {
        common,
        props,
        state: SliderState { value: 0.25 },
    };

    let output = Widget::handle_input(
        &mut slider,
        bounds,
        WidgetInput::primary_press(Point::new(60.0, 14.0)),
    );
    let batch = output
        .and_then(|output| output.typed_copied::<SliderMessage>())
        .expect("bare public SliderWidget should emit its concise message");
    assert_eq!(batch, SliderMessage::ValueChanged { value: 0.5 });

    assert_eq!(
        Widget::handle_pointer_capture_cancelled(&mut slider, bounds),
        None
    );
}

#[test]
fn slider_runtime_constructor_owns_typed_edit_lifecycle() {
    let bounds = Rect::from_min_size(Point::default(), Vector2::new(120.0, 28.0));
    let mut surface: UiSurface<SliderEditBatch> = UiSurface::new(SurfaceNode::slider_edits_mapped(
        44,
        0.25,
        WidgetSizing::fixed(Vector2::new(120.0, 28.0)),
        |batch| batch,
    ));

    let output = surface
        .dispatch_widget_input(
            44,
            bounds,
            WidgetInput::primary_press(Point::new(60.0, 14.0)),
        )
        .and_then(|output| output.typed_copied::<SliderEditBatch>())
        .expect("official Slider constructor should use the retained typed adapter");
    assert_eq!(
        output
            .events()
            .iter()
            .map(|event| event.phase)
            .collect::<Vec<_>>(),
        [EditPhase::Begin, EditPhase::Update]
    );
}

#[test]
fn knob_public_contract_keeps_legacy_shape_and_official_typed_lifecycle() {
    let bounds = Rect::from_min_size(Point::default(), Vector2::new(40.0, 40.0));
    let template = KnobWidget::new(45, 0.5);
    let KnobWidget { common, props, .. } = template;
    let mut knob = KnobWidget {
        common,
        props,
        state: KnobState {
            value: 0.5,
            gesture_origin: None,
            fine_adjustment: false,
        },
    };
    let output = Widget::handle_input(
        &mut knob,
        bounds,
        WidgetInput::primary_press(Point::new(20.0, 20.0)),
    );
    assert_eq!(
        output.and_then(|output| output.typed_copied::<KnobMessage>()),
        Some(KnobMessage::GestureStarted {
            value: 0.5,
            metadata: KnobPointerMetadata::default(),
        })
    );

    let mut surface: UiSurface<KnobEditBatch> = UiSurface::new(SurfaceNode::knob_edits_mapped(
        46,
        0.5,
        WidgetSizing::fixed(Vector2::new(40.0, 40.0)),
        |batch| batch,
    ));
    let output = surface
        .dispatch_widget_input(
            46,
            bounds,
            WidgetInput::primary_press(Point::new(20.0, 20.0)),
        )
        .and_then(|output| output.typed_copied::<KnobEditBatch>())
        .expect("official Knob constructor should emit the typed lifecycle");
    assert_eq!(
        output
            .events()
            .iter()
            .map(|event| event.phase)
            .collect::<Vec<_>>(),
        [EditPhase::Begin]
    );
}

#[test]
fn widget_paint_primitives_helper_captures_builtin_widget_paint() {
    let widget = ButtonWidget::new(42, "Paint", WidgetSizing::fixed(Vector2::new(96.0, 28.0)));
    let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(96.0, 28.0));

    let primitives = widget.paint_primitives_with_defaults(bounds);

    assert!(
        primitives
            .iter()
            .any(|primitive| primitive.fill_polygon().is_some()),
        "button chrome should be captured without app-local paint buffer setup"
    );
    assert!(
        primitives
            .iter()
            .any(|primitive| primitive.text_run().is_some()),
        "button label should be captured without app-local paint buffer setup"
    );
}
