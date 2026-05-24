use super::{
    OptionItem, OptionItemParts, OptionSelectionState, PreferencePanelParts, PreferencePanelState,
    PreferencePanelVisibility, SummaryField,
};

#[test]
fn option_item_preserves_label_selection_and_value() {
    let option = OptionItem {
        label: String::from("Default"),
        selected: true,
        value: Some(48_000_u32),
    };

    assert_eq!(option.label, "Default");
    assert!(option.selected);
    assert_eq!(option.value, Some(48_000));
}

#[test]
fn option_item_supports_named_selection_parts() {
    let option = OptionItem::from_parts(OptionItemParts {
        label: String::from("Default"),
        selection: OptionSelectionState::Selected,
        value: 48_000_u32,
    });

    assert_eq!(option.label, "Default");
    assert!(option.selected);
    assert_eq!(option.value, 48_000);
    assert_eq!(
        OptionSelectionState::from_selected(false),
        OptionSelectionState::Unselected
    );
}

#[test]
fn summary_field_defaults_to_empty_text() {
    let field = SummaryField::default();

    assert_eq!(field.label, "");
    assert_eq!(field.value_label, "");
}

#[test]
fn preference_panel_state_preserves_visibility_text_toggles_and_auxiliary_label() {
    let panel = PreferencePanelState::from_parts(PreferencePanelParts {
        visibility: PreferencePanelVisibility::Visible,
        primary_text_value: String::from("Default"),
        toggles: [true, false, true],
        auxiliary_label: Some(String::from("Destination")),
    });

    assert!(panel.visible);
    assert_eq!(panel.primary_text_value, "Default");
    assert_eq!(panel.toggles, [true, false, true]);
    assert!(panel.toggle_enabled(0));
    assert!(!panel.toggle_enabled(1));
    assert!(!panel.toggle_enabled(99));
    assert_eq!(panel.auxiliary_label.as_deref(), Some("Destination"));
}

#[test]
fn preference_panel_visibility_round_trips_compatibility_flags() {
    let panel = PreferencePanelState::new(true, "Default", [true, false], None);

    assert_eq!(
        PreferencePanelVisibility::from_visible(false),
        PreferencePanelVisibility::Hidden
    );
    assert!(panel.visible);
}
