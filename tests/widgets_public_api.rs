//! Public API coverage for `radiant::widgets`.

use radiant::{
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
        InteractiveRowWidget, InteractiveRowWidgetParts, KnobEditBatch, KnobMessage,
        KnobPointerMetadata, KnobState, KnobWidget, ListItemWidget, ListItemWidgetParts,
        NumericEditSession, ScrollbarAxis, ScrollbarWidget, ScrollbarWidgetParts, SelectableWidget,
        SelectableWidgetParts, SliderEditBatch, SliderMessage, SliderState, SliderWidget,
        SliderWidgetParts, TextInputWidget, TextInputWidgetParts, TextWidget, TextWidgetParts,
        ToggleWidget, ToggleWidgetParts, Widget, WidgetInput, WidgetKey, WidgetOutput,
        WidgetSizing, WidgetSizingParts,
    },
};
use std::{cell::RefCell, fmt::Debug, rc::Rc, sync::Arc};

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

struct NonCloneNumericValue;

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
