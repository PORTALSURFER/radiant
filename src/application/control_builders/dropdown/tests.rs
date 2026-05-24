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
