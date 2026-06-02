use super::*;
use crate::application::{IntoView, labeled_control_control_offset};
use crate::gui::layout_core::StackedLayoutCursor;

#[derive(Clone, Debug, PartialEq)]
enum Message {
    Toggle,
    Select(&'static str),
}

#[test]
fn dropdown_height_tracks_expanded_options() {
    assert_eq!(dropdown_height(false, 3), 24.0);
    assert_eq!(dropdown_height(true, 3), 24.0);
    assert_eq!(dropdown_menu_height(3), 80.0);
}

#[test]
fn dropdown_builder_accepts_toggle_and_options() {
    let _view = dropdown("WASAPI", true)
        .toggle_message(Message::Toggle)
        .option_from_parts(DropdownOptionParts {
            label: "System default".into(),
            selection: DropdownOptionSelection::Unselected,
            message: Message::Select("default"),
        })
        .option_from_parts(DropdownOptionParts {
            label: "WASAPI".into(),
            selection: DropdownOptionSelection::Selected,
            message: Message::Select("wasapi"),
        })
        .build();
}

#[test]
fn dropdown_builder_accepts_options_before_required_toggle() {
    let _view = dropdown("WASAPI", true)
        .option_from_parts(DropdownOptionParts {
            label: "System default".into(),
            selection: DropdownOptionSelection::Unselected,
            message: Message::Select("default"),
        })
        .option_from_parts(DropdownOptionParts {
            label: "WASAPI".into(),
            selection: DropdownOptionSelection::Selected,
            message: Message::Select("wasapi"),
        })
        .toggle_message(Message::Toggle)
        .build();
}

#[test]
fn dropdown_trigger_builds_external_overlay_toggle() {
    let _view = dropdown_trigger("WASAPI", true)
        .toggle_message(Message::Toggle)
        .build();
    let parts = DropdownTriggerParts {
        selected_label: "WASAPI".into(),
        open: true,
        toggle_message: Message::Toggle,
    };

    assert_eq!(parts.selected_label, "WASAPI");
    assert!(parts.open);
}

#[test]
fn dropdown_option_compatibility_constructor_delegates_to_named_parts() {
    let from_parts = DropdownOption::from_parts(DropdownOptionParts {
        label: "WASAPI".into(),
        selection: DropdownOptionSelection::Selected,
        message: Message::Select("wasapi"),
    });
    let positional = DropdownOption::new("WASAPI", true, Message::Select("wasapi"));

    assert_eq!(positional, from_parts);
    assert_eq!(
        DropdownOptionSelection::from_selected(false),
        DropdownOptionSelection::Unselected
    );
}

#[test]
fn dropdown_option_named_selection_constructors_make_state_explicit() {
    let selected = DropdownOption::selected("WASAPI", Message::Select("wasapi"));
    let unselected = DropdownOption::unselected("System default", Message::Select("default"));
    let from_selection = DropdownOption::from_selection(
        "Device default",
        DropdownOptionSelection::Unselected,
        Message::Select("device-default"),
    );

    assert!(selected.selected);
    assert!(!unselected.selected);
    assert_eq!(from_selection.label, "Device default");
    assert!(!from_selection.selected);
}

#[test]
fn dropdown_builder_accepts_explicit_option_selection() {
    let _view = dropdown("WASAPI", true)
        .option_with_selection(
            "System default",
            DropdownOptionSelection::Unselected,
            Message::Select("default"),
        )
        .toggle_message(Message::Toggle)
        .option_with_selection(
            "WASAPI",
            DropdownOptionSelection::Selected,
            Message::Select("wasapi"),
        )
        .build();
}

#[test]
fn dropdown_menu_overlay_below_positions_menu_after_trigger_gap() {
    let _view = dropdown_menu_overlay_below(
        12.0,
        20.0,
        24.0,
        3.0,
        Some(120.0),
        vec![DropdownOption::new(
            "WASAPI",
            true,
            Message::Select("wasapi"),
        )],
    );
    let parts = DropdownMenuOverlayBelowParts::new(
        12.0,
        20.0,
        24.0,
        3.0,
        vec![DropdownOption::new(
            "WASAPI",
            true,
            Message::Select("wasapi"),
        )],
    )
    .width(120.0);

    assert_eq!(parts.x, 12.0);
    assert_eq!(parts.trigger_y, 20.0);
    assert_eq!(parts.trigger_height, 24.0);
    assert_eq!(parts.gap, 3.0);
    assert_eq!(parts.width, Some(120.0));
}

#[test]
fn dropdown_menu_overlay_below_trigger_uses_standard_trigger_height() {
    assert_eq!(dropdown_trigger_height(), dropdown_height(false, 99));

    let _view = dropdown_menu_overlay_below_trigger(
        12.0,
        20.0,
        3.0,
        Some(120.0),
        vec![DropdownOption::new(
            "WASAPI",
            true,
            Message::Select("wasapi"),
        )],
    );
}

#[test]
fn dropdown_menu_overlay_below_labeled_control_uses_standard_control_offset() {
    let frame = dropdown_menu_overlay_below_labeled_control(
        12.0,
        20.0,
        3.0,
        Some(120.0),
        vec![DropdownOption::new(
            "WASAPI",
            true,
            Message::Select("wasapi"),
        )],
    )
    .view_frame_at_size_with_default_theme(crate::gui::types::Vector2::new(240.0, 160.0));

    let menu_text = frame
        .paint_plan
        .first_text_run("WASAPI")
        .expect("dropdown option should be painted");

    assert!(menu_text.rect.min.y >= 20.0 + labeled_control_control_offset());
}

#[test]
fn dropdown_menu_overlay_below_stacked_labeled_control_uses_stack_cursor_offset() {
    let cursor = StackedLayoutCursor::new()
        .advanced(20.0, 7.0)
        .advanced_if(true, 18.0, 3.0);
    let frame = dropdown_menu_overlay_below_stacked_labeled_control(
        12.0,
        8.0,
        cursor,
        3.0,
        Some(120.0),
        vec![DropdownOption::new(
            "WASAPI",
            true,
            Message::Select("wasapi"),
        )],
    )
    .view_frame_at_size_with_default_theme(crate::gui::types::Vector2::new(240.0, 220.0));

    let menu_text = frame
        .paint_plan
        .first_text_run("WASAPI")
        .expect("dropdown option should be painted");

    assert!(menu_text.rect.min.y >= 8.0 + cursor.offset() + labeled_control_control_offset());
}
