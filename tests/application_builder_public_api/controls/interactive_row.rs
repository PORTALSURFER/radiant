use radiant::prelude::{self as ui, IntoView};
use radiant::widgets::PointerModifiers;
use radiant::{gui::list as gui_list, widgets as widget_api};
use std::{cell::RefCell, rc::Rc};

#[derive(Clone, Debug, PartialEq)]
enum RowMessage {
    Activate,
    Secondary(ui::Point),
    Drop,
}

fn action_row() -> ui::View<RowMessage> {
    let accent = ui::Rgba8::new(80, 160, 220, 255);
    ui::interactive_row_underlay(ui::text_line("Item", 22.0))
        .input_id(44)
        .visual_state(widget_api::InteractiveRowVisualStateParts {
            selected: true,
            ..widget_api::InteractiveRowVisualStateParts::default()
        })
        .dense_chrome_palette(ui::DenseRowPalette::new().selected(accent.with_alpha(96)))
        .leading_marker(ui::DenseRowMarkerStyle::new(
            gui_list::DenseRowMarkerParts::leading(2.0),
            accent,
        ))
        .outline(ui::DenseRowOutlineStyle::new(0.5, accent, 1.5))
        .actions(
            ui::InteractiveRowActions::new()
                .activate(|| RowMessage::Activate)
                .secondary(RowMessage::Secondary)
                .drop(|| RowMessage::Drop),
        )
        .size(160.0, 22.0)
}

fn dense_policy_row() -> ui::View<RowMessage> {
    ui::interactive_row_underlay(ui::text_line("Item", 22.0))
        .dense_row_policy(
            ui::DenseRowPolicy::selectable(true)
                .activation_modifiers()
                .tracked_drag_source(false, false),
        )
        .input_id(45)
        .actions(ui::row_actions().activate(|| RowMessage::Activate))
        .size(160.0, 22.0)
}

#[test]
fn interactive_row_actions_are_available_from_prelude() {
    let secondary = ui::Point::new(8.0, 12.0);

    assert_eq!(
        action_row().view_dispatch_widget_output(
            44,
            ui::WidgetOutput::typed(ui::InteractiveRowMessage::Activate),
        ),
        Some(RowMessage::Activate)
    );
    assert_eq!(
        action_row().view_dispatch_widget_output(
            44,
            ui::WidgetOutput::typed(ui::InteractiveRowMessage::SecondaryActivate {
                position: secondary,
            }),
        ),
        Some(RowMessage::Secondary(secondary))
    );
    assert_eq!(
        action_row().view_dispatch_widget_output(
            44,
            ui::WidgetOutput::typed(ui::InteractiveRowMessage::Drop)
        ),
        Some(RowMessage::Drop)
    );
}

#[test]
fn dense_row_policy_is_available_from_prelude() {
    assert_eq!(
        dense_policy_row().view_dispatch_widget_output(
            45,
            ui::WidgetOutput::typed(ui::InteractiveRowMessage::Activate),
        ),
        Some(RowMessage::Activate)
    );
}

#[test]
fn interactive_row_metadata_is_available_from_prelude() {
    let metadata = ui::InteractiveRowMetadata::default();
    let message = ui::InteractiveRowMessage::Hover {
        position: ui::Point::new(8.0, 12.0),
        metadata,
    };

    assert_eq!(message.input_metadata(), metadata);
    assert_eq!(
        ui::InteractiveRowMessage::Activate.input_metadata(),
        ui::InteractiveRowMetadata::default()
    );
}

#[test]
fn knob_wheel_metadata_and_compatibility_constructor_are_available_from_prelude() {
    let metadata = ui::KnobWheelMetadata {
        modifiers: PointerModifiers {
            shift: true,
            ..PointerModifiers::default()
        },
        ..ui::KnobWheelMetadata::default()
    };
    let gesture = ui::KnobWheelGesture::new_with_metadata(0.25, 0.252, metadata);

    assert_eq!(gesture.input_metadata(), metadata);
    assert_eq!(
        ui::KnobWheelGesture::new(0.25, 0.3).input_metadata(),
        ui::KnobWheelMetadata::default()
    );
}

#[test]
fn knob_keyboard_metadata_and_compatibility_constructor_are_available_from_prelude() {
    let metadata = ui::KnobKeyboardMetadata::default();
    let gesture = ui::KnobKeyboardGesture::new_with_metadata(0.25, 0.35, metadata);

    assert_eq!(gesture.input_metadata(), metadata);
    assert_eq!(
        ui::KnobKeyboardGesture::new(0.25, 0.3).input_metadata(),
        ui::KnobKeyboardMetadata::default()
    );
}

#[test]
fn knob_pointer_metadata_and_message_accessor_are_available_from_prelude() {
    let metadata = ui::KnobPointerMetadata {
        modifiers: PointerModifiers {
            command: true,
            ..PointerModifiers::default()
        },
        ..ui::KnobPointerMetadata::default()
    };
    let message = ui::KnobMessage::GestureStarted {
        value: 0.25,
        metadata,
    };

    assert_eq!(message.pointer_gesture_metadata(), Some(metadata));
    assert_eq!(
        ui::KnobPointerMetadata::empty(),
        ui::KnobPointerMetadata::default()
    );
    assert_eq!(
        ui::KnobMessage::Reset { value: 0.25 }.pointer_gesture_metadata(),
        None
    );
}

#[test]
fn local_interactive_row_actions_accept_ui_only_capture() {
    #[derive(Clone, Debug, PartialEq)]
    struct UiOnlyMessage(Rc<RefCell<usize>>);

    let calls = Rc::new(RefCell::new(0usize));
    let captured = Rc::clone(&calls);
    let message_calls = Rc::clone(&calls);
    let secondary_calls = Rc::clone(&calls);
    let drag_calls = Rc::clone(&calls);
    let drop_calls = Rc::clone(&calls);
    let view = ui::interactive_row_underlay(ui::text_line("Local", 22.0))
        .input_id(46)
        .actions_local(
            ui::InteractiveRowLocalActions::new()
                .activate(move || {
                    *captured.borrow_mut() += 1;
                    UiOnlyMessage(Rc::clone(&message_calls))
                })
                .secondary(move |_| {
                    *secondary_calls.borrow_mut() += 1;
                    UiOnlyMessage(Rc::clone(&secondary_calls))
                })
                .drag(move |_| {
                    *drag_calls.borrow_mut() += 1;
                    UiOnlyMessage(Rc::clone(&drag_calls))
                })
                .drop(move || {
                    *drop_calls.borrow_mut() += 1;
                    UiOnlyMessage(Rc::clone(&drop_calls))
                }),
        )
        .size(160.0, 22.0);

    let surface = view.into_surface();
    let message = surface
        .dispatch_widget_output(
            46,
            ui::WidgetOutput::typed(ui::InteractiveRowMessage::Activate),
        )
        .expect("local row action should dispatch");
    assert_eq!(*calls.borrow(), 1);
    assert_eq!(*message.0.borrow(), 1);

    let secondary = surface.dispatch_widget_output(
        46,
        ui::WidgetOutput::typed(ui::InteractiveRowMessage::SecondaryActivate {
            position: ui::Point::new(8.0, 10.0),
        }),
    );
    assert!(secondary.is_some());
    let drag = surface.dispatch_widget_output(
        46,
        ui::WidgetOutput::typed(ui::InteractiveRowMessage::Drag(
            ui::DragHandleMessage::moved(ui::Point::new(9.0, 11.0)),
        )),
    );
    assert!(drag.is_some());
    let drop = surface
        .dispatch_widget_output(46, ui::WidgetOutput::typed(ui::InteractiveRowMessage::Drop));
    assert!(drop.is_some());
    assert_eq!(*calls.borrow(), 4);
}
