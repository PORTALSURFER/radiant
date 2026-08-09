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
        InteractiveRowWidget, InteractiveRowWidgetParts, KeyboardModifiers, KnobEditBatch,
        KnobMessage, KnobPointerMetadata, KnobState, KnobWidget, ListItemWidget,
        ListItemWidgetParts, NumericAdjustment, NumericCodec, NumericEditSession,
        NumericInputConstructionError, NumericInputEditBatch, NumericParseResult, NumericStep,
        NumericStepDirection, ScrollbarAxis, ScrollbarWidget, ScrollbarWidgetParts,
        SelectableWidget, SelectableWidgetParts, SliderEditBatch, SliderMessage, SliderState,
        SliderWidget, SliderWidgetParts, TextInputWidget, TextInputWidgetParts, TextWidget,
        TextWidgetParts, ToggleWidget, ToggleWidgetParts, Widget, WidgetInput, WidgetKey,
        WidgetOutput, WidgetSizing, WidgetSizingParts,
    },
};
use std::{
    cell::RefCell,
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
    let builder = result.expect("generic public numeric input should construct");
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
fn numeric_input_edit_batch_accepts_only_begin_then_terminal_pairs() {
    let provenance = InteractionProvenance::Programmatic;
    let begin = EditEvent::begin(GenericNumericValue(7), provenance);
    let commit = begin
        .clone()
        .commit(GenericNumericValue(8), provenance)
        .expect("matching source should commit");
    let cancel = begin
        .clone()
        .cancel(provenance)
        .expect("matching source should cancel");

    let commit_batch = radiant::widgets::interaction::NumericInputEditBatch::from_events(&[
        begin.clone(),
        commit.clone(),
    ])
    .expect("Begin followed by Commit should be accepted");
    assert_eq!(
        commit_batch.len(),
        NumericInputEditBatch::<GenericNumericValue>::MAX_EVENTS
    );
    assert_eq!(commit_batch.events()[0].phase, EditPhase::Begin);
    assert_eq!(commit_batch.events()[1].phase, EditPhase::Commit);

    let cancel_batch = NumericInputEditBatch::from_events(&[begin.clone(), cancel.clone()])
        .expect("Begin followed by Cancel should be accepted");
    assert_eq!(cancel_batch.events()[0].phase, EditPhase::Begin);
    assert_eq!(cancel_batch.events()[1].phase, EditPhase::Cancel);

    assert!(NumericInputEditBatch::<GenericNumericValue>::from_events(&[]).is_none());
    assert!(NumericInputEditBatch::from_events(std::slice::from_ref(&begin)).is_none());
    assert!(NumericInputEditBatch::from_events(&[begin.clone(), begin.clone()]).is_none());

    let update = begin
        .clone()
        .update(GenericNumericValue(9), provenance)
        .expect("matching source should update");
    assert!(NumericInputEditBatch::from_events(&[begin.clone(), update]).is_none());
    assert!(NumericInputEditBatch::from_events(&[commit.clone(), begin.clone()]).is_none());
    assert!(
        NumericInputEditBatch::from_events(&[begin.clone(), commit.clone(), cancel.clone(),])
            .is_none()
    );

    let other_begin = EditEvent::begin(GenericNumericValue(7), provenance);
    let other_commit = other_begin
        .clone()
        .commit(GenericNumericValue(8), provenance)
        .expect("matching source should commit");
    assert!(NumericInputEditBatch::from_events(&[begin, other_commit]).is_none());
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
        assert!(!source.contains("NumericInputConstructionError"));
        assert!(!source.contains("NumericCodec"));
        assert!(!source.contains("NumericAdjustment"));
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
