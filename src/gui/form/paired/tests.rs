use super::{PairedPickerTarget, PairedPickerValue, PairedStatusPanel};
use crate::gui::form::OptionItem;

#[test]
fn paired_picker_models_cover_primary_and_secondary_fields() {
    let target = PairedPickerTarget::SecondaryNumber;
    let value: PairedPickerValue<String, u32> = PairedPickerValue::PrimaryNumber(Some(48_000));

    assert_eq!(target, PairedPickerTarget::SecondaryNumber);
    assert_eq!(value, PairedPickerValue::PrimaryNumber(Some(48_000)));
}

#[test]
fn paired_status_panel_returns_options_for_target() {
    let panel = PairedStatusPanel {
        active_picker: Some(PairedPickerTarget::SecondaryNumber),
        secondary_number_options: vec![OptionItem {
            label: String::from("Default"),
            selected: true,
            value: PairedPickerValue::<String, u32>::SecondaryNumber(None),
        }],
        ..PairedStatusPanel::default()
    };

    assert_eq!(
        panel.active_picker(),
        Some(PairedPickerTarget::SecondaryNumber)
    );
    assert_eq!(
        panel.options_for(PairedPickerTarget::SecondaryNumber)[0].label,
        "Default"
    );
}
