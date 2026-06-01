use super::*;

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
