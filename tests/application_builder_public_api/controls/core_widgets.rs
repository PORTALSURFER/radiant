use super::super::*;
use radiant::runtime::EventMapper;
use radiant::widgets::{
    BadgeMessage, BadgeWidget, ButtonMessage, ButtonWidget, ColorMarkerRunWidget,
    ColorMarkerWidget, DragHandleMessage, DragHandleMetadata, EditEvent, EditPhase,
    FeedbackOverlayWidget, FocusBehavior, IconButtonWidget, InteractionProvenance,
    InteractionSource, KnobEditBatch, MarkerRunWidget, NumericAdjustment, NumericStep,
    NumericStepDirection, PaintBounds, PointerModifiers, SelectableWidget, SliderEditBatch,
    SliderMessage, TextInputWidget, TextWidget, ToggleMessage, ToggleWidget, ValueFormat,
    WidgetInput, WidgetOutput, WidgetProminence, WidgetStyle, WidgetTone,
};
use std::sync::Arc;
use std::{cell::RefCell, rc::Rc};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DomainAdjustmentError {
    UnsupportedAction,
}

#[derive(Clone, Copy)]
struct LinearDomainAdjustment;

impl NumericAdjustment<f32> for LinearDomainAdjustment {
    type Error = DomainAdjustmentError;

    fn normalized_to_value(&self, normalized: f32) -> Result<f32, Self::Error> {
        Ok(10.0 + normalized * 100.0)
    }

    fn value_to_normalized(&self, value: &f32) -> Result<f32, Self::Error> {
        Ok((*value - 10.0) / 100.0)
    }

    fn step(
        &self,
        _value: &f32,
        _direction: NumericStepDirection,
        _step: NumericStep,
    ) -> Result<f32, Self::Error> {
        Err(DomainAdjustmentError::UnsupportedAction)
    }

    fn scrub(
        &self,
        _value: &f32,
        _normalized_delta: f32,
        _step: NumericStep,
    ) -> Result<f32, Self::Error> {
        Err(DomainAdjustmentError::UnsupportedAction)
    }

    fn wheel(&self, _value: &f32, _delta: f32, _step: NumericStep) -> Result<f32, Self::Error> {
        Err(DomainAdjustmentError::UnsupportedAction)
    }
}

#[derive(Clone, Debug, PartialEq)]
enum ToggleMappedMessage {
    ValueChanged {
        checked: bool,
        provenance: InteractionProvenance,
    },
}

#[derive(Clone, Debug, PartialEq)]
enum ButtonMappedMessage {
    Output(ButtonMessage),
}

#[test]
fn mapped_control_accepts_ui_local_capture() {
    use radiant::prelude::{self as ui, IntoView};

    let calls = Rc::new(RefCell::new(0usize));
    let captured = Rc::clone(&calls);
    let surface: UiSurface<()> = ui::button("Local")
        .mapped(move |_| {
            *captured.borrow_mut() += 1;
        })
        .into_surface();

    let widget_id = surface.root().id();
    assert!(
        surface
            .dispatch_widget_output(
                widget_id,
                WidgetOutput::typed(crate::programmatic_button_message())
            )
            .is_some()
    );
    assert_eq!(*calls.borrow(), 1);
}

#[test]
fn typed_toggle_builders_forward_complete_provenance_payloads() {
    use radiant::prelude::{self as ui, IntoView};

    fn map_toggle(message: ToggleMessage) -> ToggleMappedMessage {
        match message {
            ToggleMessage::ValueChanged {
                checked,
                provenance,
            } => ToggleMappedMessage::ValueChanged {
                checked,
                provenance,
            },
        }
    }

    let surface: UiSurface<ToggleMappedMessage> = ui::column([
        ui::toggle("Message with", false)
            .message_with(EventMapper::with_revision(1_u8, map_toggle))
            .id(30),
        ui::toggle("Mapped with", false)
            .mapped_with(EventMapper::with_revision(2_u8, map_toggle))
            .id(31),
        ui::toggle_mapped_with(
            "Free function",
            false,
            EventMapper::with_revision(3_u8, map_toggle),
        )
        .id(32),
    ])
    .into_surface();
    let provenance = InteractionProvenance::Pointer {
        modifiers: PointerModifiers {
            command: true,
            shift: false,
            alt: true,
        },
        timestamp: None,
        sequence_range: None,
    };

    for widget_id in [30, 31, 32] {
        assert_eq!(
            surface.dispatch_widget_output(
                widget_id,
                WidgetOutput::typed(ToggleMessage::ValueChanged {
                    checked: true,
                    provenance,
                }),
            ),
            Some(ToggleMappedMessage::ValueChanged {
                checked: true,
                provenance,
            })
        );
    }
}

#[test]
fn typed_button_builders_forward_complete_provenance_payloads() {
    use radiant::prelude::{self as ui, IntoView};

    fn map_button(message: ButtonMessage) -> ButtonMappedMessage {
        ButtonMappedMessage::Output(message)
    }

    let surface: UiSurface<ButtonMappedMessage> = ui::column([
        ui::button("Mapped").mapped(map_button).id(34),
        ui::button("Mapped with")
            .mapped_with(EventMapper::with_revision(4_u8, map_button))
            .id(35),
        ui::button_mapped("Free mapped", map_button).id(36),
        ui::button_mapped_with(
            "Free mapped with",
            EventMapper::with_revision(5_u8, map_button),
        )
        .id(37),
        ui::button("Filtered")
            .filter_mapped(|message| {
                message
                    .activation_provenance()
                    .is_some()
                    .then_some(ButtonMappedMessage::Output(message))
            })
            .id(38),
        ui::disclosure_button(false).mapped(map_button).id(39),
    ])
    .into_surface();
    let expected = ButtonMessage::ActivateWithModifiers {
        provenance: InteractionProvenance::Pointer {
            modifiers: PointerModifiers {
                command: true,
                shift: false,
                alt: true,
            },
            timestamp: None,
            sequence_range: None,
        },
    };

    for widget_id in [34, 35, 36, 37, 38, 39] {
        assert_eq!(
            surface.dispatch_widget_output(widget_id, WidgetOutput::typed(expected),),
            Some(ButtonMappedMessage::Output(expected))
        );
    }
}

#[test]
fn concise_toggle_builders_keep_checked_only_host_messages() {
    use radiant::prelude::{self as ui, IntoView};

    let surface: UiSurface<bool> = ui::column([
        ui::toggle("Message", false)
            .message(|checked| checked)
            .id(33),
        ui::toggle_mapped("Mapped", false, |checked| !checked).id(34),
    ])
    .into_surface();

    assert_eq!(
        surface.dispatch_widget_output(
            33,
            WidgetOutput::typed(ToggleMessage::ValueChanged {
                checked: true,
                provenance: InteractionProvenance::Programmatic,
            }),
        ),
        Some(true)
    );
    assert_eq!(
        surface.dispatch_widget_output(
            34,
            WidgetOutput::typed(ToggleMessage::ValueChanged {
                checked: true,
                provenance: InteractionProvenance::Keyboard { timestamp: None },
            }),
        ),
        Some(false)
    );
}

#[test]
fn constant_button_message_accepts_rc_backed_ui_message() {
    use radiant::prelude::{self as ui, IntoView};

    #[derive(Clone)]
    struct UiOnlyMessage(Rc<RefCell<usize>>);

    let state = Rc::new(RefCell::new(7usize));
    let surface: UiSurface<UiOnlyMessage> = ui::button("Local")
        .message(UiOnlyMessage(Rc::clone(&state)))
        .id(29)
        .into_surface();

    let message = surface
        .dispatch_widget_output(
            29,
            WidgetOutput::typed(crate::programmatic_button_message()),
        )
        .expect("button should emit its UI-local message");
    assert!(Rc::ptr_eq(&message.0, &state));
}

#[test]
fn application_text_builders_accept_static_owned_and_shared_content() {
    use radiant::prelude::{self as ui, IntoView};

    let shared: Arc<str> = Arc::from("Shared status");
    let surface: UiSurface<()> = ui::column([
        ui::text("Ready").id(40),
        ui::button(String::from("Owned action")).message(()).id(41),
        ui::badge(Arc::clone(&shared)).message(()).id(42),
        ui::toggle("Enabled", true).message(|_| ()).id(43),
        ui::selectable(Arc::clone(&shared), false)
            .message(|_| ())
            .id(44),
    ])
    .into_surface();

    assert!(
        widget_ref::<TextWidget, _>(&surface, 40, "text")
            .text
            .is_static()
    );
    assert!(
        !widget_ref::<ButtonWidget, _>(&surface, 41, "button")
            .props
            .label
            .is_static()
    );
    assert_eq!(
        widget_ref::<BadgeWidget, _>(&surface, 42, "badge")
            .props
            .label
            .as_str(),
        shared.as_ref()
    );
    assert!(
        widget_ref::<ToggleWidget, _>(&surface, 43, "toggle")
            .props
            .label
            .is_static()
    );
    assert_eq!(
        widget_ref::<SelectableWidget, _>(&surface, 44, "selectable")
            .props
            .label
            .as_str(),
        shared.as_ref()
    );
}

#[test]
fn application_builder_dense_control_panel_uses_generic_focusable_widgets() {
    use radiant::prelude::{self as ui, IntoView};

    let surface: UiSurface<()> = ui::column([
        ui::row([
            ui::toggle("Enabled", true).message(|_| ()).id(10),
            ui::toggle("Link", false).message(|_| ()).id(11),
        ])
        .id(2)
        .fill_width(),
        ui::grid_with_gaps(
            (0..3).map(|index| {
                ui::column([
                    ui::text(format!("Param {index}"))
                        .id(100 + index)
                        .height(22.0),
                    ui::row([
                        ui::button("-").subtle().message(()).id(200 + index * 2),
                        ui::button("+").primary().message(()).id(201 + index * 2),
                    ]),
                ])
                .id(50 + index)
                .style(WidgetStyle {
                    tone: WidgetTone::Neutral,
                    prominence: WidgetProminence::Subtle,
                })
                .padding(8.0)
                .height(96.0)
            }),
            3,
            8.0,
            8.0,
        )
        .id(3)
        .fill_width(),
    ])
    .id(1)
    .padding(12.0)
    .spacing(10.0)
    .into_surface();
    let layout = layout_tree(
        &surface.layout_node(),
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(480.0, 180.0)),
    );

    let focus_order = surface.keyboard_focus_order();
    assert_eq!(focus_order.len(), 8);
    assert!(focus_order.contains(&10));
    assert!(focus_order.contains(&205));
    assert_eq!(layout.rects[&50].min.y, layout.rects[&51].min.y);
    assert!(layout.rects[&51].min.x > layout.rects[&50].max.x);
    assert_eq!(layout.rects[&50].height(), 96.0);
}

#[test]
fn button_row_groups_app_owned_buttons_with_compact_geometry() {
    use radiant::prelude::{self as ui, IntoView};

    let surface: UiSurface<DemoMessage> = ui::column([ui::button_row([
        ui::button("Apply")
            .primary()
            .message(DemoMessage::Increment)
            .id(20)
            .width(72.0),
        ui::button("Cancel")
            .message(DemoMessage::Increment)
            .id(21)
            .width(68.0),
    ])
    .id(10)])
    .into_surface();
    let layout = layout_tree(
        &surface.layout_node(),
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(180.0, 40.0)),
    );

    assert_eq!(surface.keyboard_focus_order(), vec![20, 21]);
    assert_eq!(layout.rects[&10].height(), 26.0);
    assert_eq!(layout.rects[&20].height(), 24.0);
    assert!((layout.rects[&21].min.x - layout.rects[&20].max.x - 6.0).abs() < 0.01);
    assert_eq!(
        surface.dispatch_widget_output(
            20,
            WidgetOutput::typed(crate::programmatic_button_message())
        ),
        Some(DemoMessage::Increment)
    );
}

#[test]
fn application_builders_expose_padding_style_and_text_policy_helpers() {
    use radiant::prelude::{self as ui, IntoView};

    let surface: UiSurface<()> = ui::column([
        ui::text("Long title").wrap().id(10),
        ui::button("Add").primary().message(()).id(11),
        ui::button("Delete").danger().message(()).id(12),
        ui::checkbox(true).message(|_| ()).id(13),
        ui::text_input("")
            .placeholder("What needs to be done?")
            .message(|_| ())
            .id(14),
        ui::slider(0.4).primary().message(|_| ()).id(15),
    ])
    .id(1)
    .padding(16.0)
    .into_surface();

    let layout = layout_tree(
        &surface.layout_node(),
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(240.0, 160.0)),
    );

    assert_eq!(layout.rects[&10].min.x, 16.0);
    assert_eq!(
        widget_ref::<TextWidget, _>(&surface, 10, "text").wrap,
        radiant::widgets::TextWrap::Word
    );
    let primary = widget_ref::<ButtonWidget, _>(&surface, 11, "button");
    assert_eq!(primary.common.style.tone, WidgetTone::Accent);
    assert_eq!(primary.common.style.prominence, WidgetProminence::Strong);
    assert_eq!(
        widget_ref::<ButtonWidget, _>(&surface, 12, "button")
            .common
            .style
            .tone,
        WidgetTone::Danger
    );
    let toggle = widget_ref::<ToggleWidget, _>(&surface, 13, "toggle");
    assert_eq!(toggle.props.label, "");
    assert!(toggle.state.checked);
    assert_eq!(toggle.common.sizing.preferred, Vector2::new(22.0, 22.0));
    assert_eq!(
        widget_ref::<TextInputWidget, _>(&surface, 14, "text input")
            .props
            .placeholder
            .as_deref(),
        Some("What needs to be done?")
    );
    let slider = surface
        .find_widget(15)
        .expect("slider widget should exist")
        .widget();
    assert_eq!(
        slider.automation_semantics().value_text.as_deref(),
        Some("0.400")
    );
    assert_eq!(slider.common().style.tone, WidgetTone::Accent);
    assert_eq!(slider.common().style.prominence, WidgetProminence::Strong);
    assert_eq!(
        surface.dispatch_widget_output(
            15,
            WidgetOutput::typed(SliderMessage::ValueChanged { value: 0.75 }),
        ),
        Some(())
    );
}

#[test]
fn official_numeric_builder_formats_are_display_only_and_default_text_is_stable() {
    use radiant::prelude::{self as ui, IntoView};

    let formatted_slider: UiSurface<SliderEditBatch> = ui::slider(0.25)
        .format(ValueFormat::percent(0))
        .on_edit(|batch| batch)
        .id(50)
        .into_surface();
    let plain_slider: UiSurface<SliderEditBatch> = ui::slider(0.25)
        .on_edit(|batch| batch)
        .id(51)
        .into_surface();
    let formatted_knob: UiSurface<KnobEditBatch> = ui::knob(0.5)
        .format(ValueFormat::frequency())
        .on_edit(|batch| batch)
        .id(52)
        .into_surface();
    let plain_knob: UiSurface<KnobEditBatch> =
        ui::knob(0.5).on_edit(|batch| batch).id(53).into_surface();

    assert_eq!(
        formatted_slider
            .find_widget(50)
            .expect("formatted slider")
            .widget()
            .automation_semantics()
            .value_text
            .as_deref(),
        Some("25%")
    );
    assert_eq!(
        formatted_knob
            .find_widget(52)
            .expect("formatted knob")
            .widget()
            .automation_semantics()
            .value_text
            .as_deref(),
        Some("0.50 Hz")
    );
    assert_eq!(
        plain_slider
            .find_widget(51)
            .expect("plain slider")
            .widget()
            .automation_semantics()
            .value_text
            .as_deref(),
        Some("0.250")
    );
    assert_eq!(
        plain_knob
            .find_widget(53)
            .expect("plain knob")
            .widget()
            .automation_semantics()
            .value_text
            .as_deref(),
        Some("0.500")
    );

    let project_events = |events: &[EditEvent<f32>]| {
        events
            .iter()
            .map(|event| {
                (
                    event.phase,
                    event.start_value,
                    event.value,
                    event.provenance,
                )
            })
            .collect::<Vec<_>>()
    };
    let assert_pointer_transaction = |events: &[EditEvent<f32>]| {
        let begin = events
            .first()
            .expect("edit sequence should contain a begin event");
        assert_eq!(begin.phase, EditPhase::Begin);
        let transaction = begin.transaction;
        let begin_source = begin.provenance.source();
        assert_eq!(begin_source, InteractionSource::Pointer);
        assert!(events.iter().all(|event| {
            event.transaction == transaction && event.provenance.source() == begin_source
        }));
    };

    let bounds = Rect::from_min_size(Point::default(), Vector2::new(120.0, 28.0));
    let mut formatted_slider = formatted_slider;
    let mut plain_slider = plain_slider;
    let slider_sequence = |surface: &mut UiSurface<SliderEditBatch>, widget_id| {
        let mut events = Vec::new();
        for input in [
            WidgetInput::primary_press(Point::new(60.0, 14.0)),
            WidgetInput::pointer_move(Point::new(90.0, 14.0)),
            WidgetInput::primary_release(Point::new(90.0, 14.0)),
        ] {
            let batch = surface
                .dispatch_widget_input(widget_id, bounds, input)
                .expect("official slider output")
                .typed_copied::<SliderEditBatch>()
                .expect("official slider should retain typed output");
            events.extend(batch.events().iter().copied());
        }
        events
    };
    let formatted_slider_events = slider_sequence(&mut formatted_slider, 50);
    let plain_slider_events = slider_sequence(&mut plain_slider, 51);
    assert_pointer_transaction(&formatted_slider_events);
    assert_pointer_transaction(&plain_slider_events);
    assert_eq!(
        formatted_slider_events
            .iter()
            .map(|event| event.phase)
            .collect::<Vec<_>>(),
        [
            EditPhase::Begin,
            EditPhase::Update,
            EditPhase::Update,
            EditPhase::Commit
        ]
    );
    assert_eq!(
        project_events(&formatted_slider_events),
        project_events(&plain_slider_events)
    );

    let knob_bounds = Rect::from_min_size(Point::default(), Vector2::new(40.0, 40.0));
    let mut formatted_knob = formatted_knob;
    let mut plain_knob = plain_knob;
    let knob_sequence = |surface: &mut UiSurface<KnobEditBatch>, widget_id| {
        let mut events = Vec::new();
        for input in [
            WidgetInput::primary_press(Point::new(20.0, 20.0)),
            WidgetInput::pointer_move(Point::new(20.0, 10.0)),
            WidgetInput::primary_release(Point::new(20.0, 10.0)),
        ] {
            let batch = surface
                .dispatch_widget_input(widget_id, knob_bounds, input)
                .expect("official knob output")
                .typed_copied::<KnobEditBatch>()
                .expect("official knob should retain typed output");
            events.extend(batch.events().iter().copied());
        }
        events
    };
    let formatted_knob_events = knob_sequence(&mut formatted_knob, 52);
    let plain_knob_events = knob_sequence(&mut plain_knob, 53);
    assert_pointer_transaction(&formatted_knob_events);
    assert_pointer_transaction(&plain_knob_events);
    assert_eq!(
        formatted_knob_events
            .iter()
            .map(|event| event.phase)
            .collect::<Vec<_>>(),
        [EditPhase::Begin, EditPhase::Update, EditPhase::Commit]
    );
    assert_eq!(
        project_events(&formatted_knob_events),
        project_events(&plain_knob_events)
    );
}

#[test]
fn qualified_slider_domain_builder_maps_outputs_and_formats_domain_values() {
    use radiant::prelude::IntoView;

    type DomainMessage = radiant::widgets::interaction::SliderDomainMessage<DomainAdjustmentError>;

    let mut surface: UiSurface<DomainMessage> =
        radiant::application::slider_domain(10.0, LinearDomainAdjustment)
            .expect("finite inverse should construct the domain slider")
            .format(ValueFormat::decimal(1))
            .message(|message| message)
            .id(54)
            .into_surface();

    assert_eq!(
        surface
            .find_widget(54)
            .expect("domain slider should be projected")
            .widget()
            .automation_semantics()
            .value_text
            .as_deref(),
        Some("10.0")
    );

    let bounds = Rect::from_min_size(Point::default(), Vector2::new(120.0, 28.0));
    let output = surface
        .dispatch_widget_input(
            54,
            bounds,
            WidgetInput::primary_press(Point::new(60.0, 14.0)),
        )
        .expect("domain slider should emit a mapped output");
    assert!(output.typed_ref::<SliderEditBatch>().is_none());
    assert_eq!(
        output.typed_cloned::<DomainMessage>(),
        Some(DomainMessage::ValueChanged { value: 60.0 })
    );
    assert_eq!(
        surface.dispatch_widget_output(54, output),
        Some(DomainMessage::ValueChanged { value: 60.0 })
    );
    assert_eq!(
        surface
            .find_widget(54)
            .expect("domain slider should remain projected")
            .widget()
            .automation_semantics()
            .value_text
            .as_deref(),
        Some("60.0")
    );
}

#[test]
fn qualified_slider_edit_builders_forward_complete_batches() {
    use radiant::prelude::{self as ui, IntoView};

    let on_edit: UiSurface<SliderEditBatch> = ui::slider(0.25)
        .on_edit(|batch| batch)
        .id(40)
        .into_surface();
    let free_function: UiSurface<SliderEditBatch> =
        radiant::application::slider_edit_mapped(0.25, |batch| batch)
            .id(41)
            .into_surface();
    let provenance = InteractionProvenance::Keyboard { timestamp: None };
    let begin = EditEvent::begin(0.25, provenance);
    let update = begin.update(0.5, provenance).expect("slider update");
    let batch = SliderEditBatch::from_events(&[begin, update]).expect("slider edit batch");

    for (surface, widget_id) in [(&on_edit, 40), (&free_function, 41)] {
        assert_eq!(
            surface.dispatch_widget_output(widget_id, WidgetOutput::typed(batch)),
            Some(batch)
        );
        assert_eq!(
            batch
                .events()
                .iter()
                .map(|event| event.phase)
                .collect::<Vec<_>>(),
            [EditPhase::Begin, EditPhase::Update]
        );
    }
}

#[test]
fn qualified_knob_edit_builders_forward_complete_batches() {
    use radiant::prelude::{self as ui, IntoView};

    let mut on_edit: UiSurface<KnobEditBatch> =
        ui::knob(0.5).on_edit(|batch| batch).id(42).into_surface();
    let mut free_function: UiSurface<KnobEditBatch> =
        radiant::application::knob_edit_mapped(0.5, |batch| batch)
            .id(43)
            .into_surface();
    let bounds = Rect::from_min_size(Point::default(), Vector2::new(40.0, 40.0));

    for (surface, widget_id) in [(&mut on_edit, 42), (&mut free_function, 43)] {
        let output = surface
            .dispatch_widget_input(
                widget_id,
                bounds,
                WidgetInput::primary_press(Point::new(20.0, 20.0)),
            )
            .expect("official Knob builder should emit a typed batch")
            .typed_copied::<KnobEditBatch>()
            .expect("official Knob builder should retain typed output");
        assert_eq!(
            output
                .events()
                .iter()
                .map(|event| event.phase)
                .collect::<Vec<_>>(),
            [EditPhase::Begin]
        );
    }
}

#[test]
fn passive_badge_is_prelude_accessible_and_does_not_emit_messages() {
    use radiant::prelude::{self as ui, IntoView};

    let surface: UiSurface<()> = ui::passive_badge("KEEP")
        .style(WidgetStyle {
            tone: WidgetTone::Warning,
            prominence: WidgetProminence::Subtle,
        })
        .id(22)
        .into_surface();

    let badge = widget_ref::<BadgeWidget, _>(&surface, 22, "badge");
    assert_eq!(badge.props.label, "KEEP");
    assert_eq!(badge.common.style.tone, WidgetTone::Warning);
    assert_eq!(badge.common.style.prominence, WidgetProminence::Subtle);
    assert_eq!(
        surface.dispatch_widget_output(22, WidgetOutput::typed(BadgeMessage::Activate)),
        None
    );
}

#[test]
fn button_builder_can_filter_secondary_activation_and_map_drag() {
    use radiant::prelude::{self as ui, IntoView};

    let surface: UiSurface<&'static str> = ui::button("Name")
        .click_or_drag("sort", |drag| match drag {
            DragHandleMessage::Started { .. } => "drag-start",
            DragHandleMessage::Moved { .. } => "drag-move",
            DragHandleMessage::Ended { .. } => "drag-end",
            DragHandleMessage::DoubleActivate { .. } => "drag-double",
            DragHandleMessage::Cancelled { .. } => "drag-cancel",
        })
        .id(27)
        .into_surface();

    assert_eq!(
        surface.dispatch_widget_output(
            27,
            WidgetOutput::typed(crate::programmatic_button_message())
        ),
        Some("sort")
    );
    assert_eq!(
        surface.dispatch_widget_output(
            27,
            WidgetOutput::typed(ButtonMessage::SecondaryActivate {
                position: ui::Point::new(1.0, 2.0)
            }),
        ),
        None
    );
    assert_eq!(
        surface.dispatch_widget_output(
            27,
            WidgetOutput::typed(ButtonMessage::Drag(DragHandleMessage::Moved {
                position: ui::Point::new(3.0, 4.0),
                metadata: DragHandleMetadata::empty(),
            })),
        ),
        Some("drag-move")
    );
}

#[test]
fn constant_button_message_maps_all_enabled_button_outputs() {
    use radiant::prelude::{self as ui, IntoView};

    let surface: UiSurface<&'static str> = ui::button("Run")
        .secondary_clicks()
        .draggable()
        .message("run")
        .id(28)
        .into_surface();

    assert_eq!(
        surface.dispatch_widget_output(
            28,
            WidgetOutput::typed(crate::programmatic_button_message())
        ),
        Some("run")
    );
    assert_eq!(
        surface.dispatch_widget_output(
            28,
            WidgetOutput::typed(ButtonMessage::ActivateWithModifiers {
                provenance: radiant::widgets::InteractionProvenance::Pointer {
                    modifiers: Default::default(),
                    timestamp: None,
                    sequence_range: None,
                },
            }),
        ),
        Some("run")
    );
    assert_eq!(
        surface.dispatch_widget_output(
            28,
            WidgetOutput::typed(ButtonMessage::SecondaryActivate {
                position: ui::Point::new(1.0, 2.0),
            }),
        ),
        Some("run")
    );
    assert_eq!(
        surface.dispatch_widget_output(
            28,
            WidgetOutput::typed(ButtonMessage::Drag(DragHandleMessage::Moved {
                position: ui::Point::new(3.0, 4.0),
                metadata: DragHandleMetadata::empty(),
            })),
        ),
        Some("run")
    );
    assert_eq!(
        surface.dispatch_widget_output(28, WidgetOutput::typed(BadgeMessage::Activate)),
        None
    );
}

#[test]
fn dynamic_button_mappers_keep_secondary_and_filtered_behavior() {
    use radiant::prelude::{self as ui, IntoView};

    let mapped: UiSurface<&'static str> = ui::button("Mapped")
        .mapped(|message| {
            if message.is_activate() {
                "activate"
            } else {
                "other"
            }
        })
        .id(29)
        .into_surface();
    assert_eq!(
        mapped.dispatch_widget_output(
            29,
            WidgetOutput::typed(ButtonMessage::SecondaryActivate {
                position: ui::Point::new(1.0, 2.0),
            }),
        ),
        Some("other")
    );

    let filtered: UiSurface<&'static str> = ui::button("Filtered")
        .filter_mapped(|message| message.is_activate().then_some("activate"))
        .id(30)
        .into_surface();
    assert_eq!(
        filtered.dispatch_widget_output(
            30,
            WidgetOutput::typed(crate::programmatic_button_message())
        ),
        Some("activate")
    );
    assert_eq!(
        filtered.dispatch_widget_output(
            30,
            WidgetOutput::typed(ButtonMessage::SecondaryActivate {
                position: ui::Point::new(1.0, 2.0),
            }),
        ),
        None
    );
}

#[test]
fn icon_button_builder_supports_message_and_passive_apps() {
    use radiant::prelude::{self as ui, IntoView};

    let message_surface: UiSurface<DemoMessage> = ui::disclosure_button(true)
        .message(DemoMessage::Increment)
        .id(31)
        .into_surface();
    assert!(
        !widget_ref::<IconButtonWidget, _>(&message_surface, 31, "icon button")
            .common
            .state
            .active
    );
    assert_eq!(
        message_surface.dispatch_widget_output(
            31,
            WidgetOutput::typed(crate::programmatic_button_message())
        ),
        Some(DemoMessage::Increment)
    );
    assert_eq!(
        message_surface.dispatch_widget_output(
            31,
            WidgetOutput::typed(ButtonMessage::SecondaryActivate {
                position: ui::Point::new(1.0, 2.0),
            }),
        ),
        Some(DemoMessage::Increment)
    );

    let passive_surface: UiSurface<DemoState> = ui::close_button().passive().id(32).into_surface();
    assert!(passive_surface.find_widget(32).is_some());
}

#[test]
fn color_marker_is_prelude_accessible_and_passive() {
    use radiant::prelude::{self as ui, IntoView};

    let color = ui::Rgba8::new(20, 40, 60, 255);
    let surface: UiSurface<()> = ui::color_marker(Some(color))
        .side(6)
        .inset(2)
        .align(ui::ColorMarkerAlign::Left)
        .view()
        .id(23)
        .into_surface();

    let marker = widget_ref::<ColorMarkerWidget, _>(&surface, 23, "color marker");
    assert_eq!(marker.props.color, Some(color));
    assert_eq!(marker.props.side, 6);
    assert_eq!(marker.props.inset, 2);
    assert_eq!(marker.props.align, ui::ColorMarkerAlign::Left);
    assert_eq!(
        surface.dispatch_widget_output(23, WidgetOutput::typed(())),
        None
    );
}

#[test]
fn selectable_builder_supports_color_marker_adornment() {
    use radiant::prelude::{self as ui, IntoView};

    let color = ui::Rgba8::new(20, 120, 80, 255);
    let surface: UiSurface<()> = ui::selectable("Ready", true)
        .color_marker_props(
            ui::ColorMarkerProps::new(Some(color))
                .side(6)
                .inset(2)
                .align(ui::ColorMarkerAlign::Left),
        )
        .message(|_| ())
        .id(28)
        .into_surface();

    let selectable = widget_ref::<SelectableWidget, _>(&surface, 28, "selectable");
    let marker = selectable.props.color_marker.expect("selectable marker");
    assert_eq!(marker.color, Some(color));
    assert_eq!(marker.side, 6);
    assert_eq!(marker.inset, 2);
    assert_eq!(marker.align, ui::ColorMarkerAlign::Left);

    let ordered_surface: UiSurface<()> = ui::selectable("Queued", false)
        .color_marker_side(7)
        .color_marker_inset(1)
        .color_marker(Some(color))
        .message(|_| ())
        .id(29)
        .into_surface();
    let ordered = widget_ref::<SelectableWidget, _>(&ordered_surface, 29, "selectable");
    let ordered_marker = ordered.props.color_marker.expect("selectable marker");
    assert_eq!(ordered_marker.color, Some(color));
    assert_eq!(ordered_marker.side, 7);
    assert_eq!(ordered_marker.inset, 1);
}

#[test]
fn marker_run_is_prelude_accessible_and_passive() {
    use radiant::prelude::{self as ui, IntoView};

    let color = ui::Rgba8::new(80, 180, 90, 255);
    let surface: UiSurface<()> = ui::marker_run(Some(color), 3)
        .side(5)
        .gap(4)
        .inset(2)
        .view()
        .id(24)
        .into_surface();

    let markers = widget_ref::<MarkerRunWidget, _>(&surface, 24, "marker run");
    assert_eq!(markers.props.color, Some(color));
    assert_eq!(markers.props.count, 3);
    assert_eq!(markers.props.side, 5);
    assert_eq!(markers.props.gap, 4);
    assert_eq!(markers.props.inset, 2);
    assert_eq!(
        surface.dispatch_widget_output(24, WidgetOutput::typed(())),
        None
    );
}

#[test]
fn marker_run_colors_is_prelude_accessible_and_passive() {
    use radiant::prelude::{self as ui, IntoView};

    let first = ui::Rgba8::new(80, 180, 90, 255);
    let second = ui::Rgba8::new(180, 90, 80, 255);
    let surface: UiSurface<()> = ui::marker_run_colors([first, second])
        .side(5)
        .gap(4)
        .inset(2)
        .view()
        .id(25)
        .into_surface();

    let markers = widget_ref::<ColorMarkerRunWidget, _>(&surface, 25, "marker run colors");
    assert_eq!(markers.props.colors, vec![first, second]);
    assert_eq!(markers.props.side, 5);
    assert_eq!(markers.props.gap, 4);
    assert_eq!(markers.props.inset, 2);
    assert_eq!(
        surface.dispatch_widget_output(25, WidgetOutput::typed(())),
        None
    );
}

#[test]
fn feedback_overlay_is_prelude_accessible_and_passive() {
    use radiant::prelude::{self as ui, IntoView};

    let background = ui::Rgba8::new(20, 24, 28, 90);
    let fill = ui::Rgba8::new(180, 190, 200, 120);
    let edge = ui::Rgba8::new(60, 200, 120, 220);
    let surface: UiSurface<()> = ui::feedback_overlay()
        .background(background)
        .progress(0.5, fill)
        .edge(
            edge,
            2.0,
            ui::BorderSides {
                top: true,
                bottom: true,
                left: false,
                right: false,
            },
        )
        .view()
        .id(26)
        .into_surface();

    let overlay = widget_ref::<FeedbackOverlayWidget, _>(&surface, 26, "feedback overlay");
    assert_eq!(overlay.props.background, Some(background));
    assert_eq!(overlay.props.progress.expect("progress").fraction, 0.5);
    assert_eq!(overlay.props.progress.expect("progress").color, fill);
    assert_eq!(overlay.props.edge.expect("edge").color, edge);
    assert_eq!(
        surface.dispatch_widget_output(26, WidgetOutput::typed(())),
        None
    );
}

#[test]
fn interactive_row_builder_exposes_drag_source_configuration() {
    use radiant::prelude::{self as ui, IntoView};
    use radiant::widgets::InteractiveRowWidget;

    let surface: UiSurface<DemoMessage> = ui::interactive_row()
        .draggable()
        .drag_active(true)
        .drag_source(true)
        .drag_source_motion(true)
        .suppress_hover(true)
        .clear_hover_on_sync()
        .mapped(|_| DemoMessage::Increment)
        .id(25)
        .into_surface();

    let row = widget_ref::<InteractiveRowWidget, _>(&surface, 25, "interactive row");
    assert!(row.props.draggable);
    assert!(row.props.drag_active);
    assert!(row.props.drag_source);
    assert!(row.props.drag_source_motion);
    assert!(row.props.suppress_hover);
    assert!(row.props.clear_hover_on_sync);
}

#[test]
fn interactive_row_builder_can_create_custom_input_layer_widget() {
    use radiant::prelude as ui;

    let row = ui::interactive_row()
        .draggable()
        .drag_active(true)
        .drag_source(true)
        .drag_source_motion(true)
        .activation_modifiers()
        .custom_paint_hit_target()
        .widget();

    assert!(row.props.draggable);
    assert!(row.props.drag_active);
    assert!(row.props.drag_source);
    assert!(row.props.drag_source_motion);
    assert!(row.props.activation_modifiers);
    assert_eq!(row.common.focus, FocusBehavior::None);
    assert_eq!(row.common.paint.bounds, PaintBounds::ClipToRect);
    assert!(!row.common.paint.paints_focus);
    assert!(!row.common.paint.paints_state_layers);
}
