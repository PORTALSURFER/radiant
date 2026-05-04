//! Options-panel action definitions and paired-picker helpers.

use super::*;
use crate::gui::form::{OptionItem, PairedPickerTarget, PairedPickerValue, PairedStatusPanel};

type PairedPickerValueItem = PairedPickerValue<String, u32>;
type PairedPickerOptionItem = OptionItem<PairedPickerValueItem>;

pub(super) fn paired_device_overview_button_defs(model: &AppModel) -> Vec<(String, UiAction)> {
    let paired_device = model.paired_device_panel();
    vec![
        (
            format!(
                "{}: {}",
                paired_device.primary_group().label,
                paired_device.primary_group().value_label
            ),
            UiAction::OpenPrimaryGroupPicker,
        ),
        (
            format!(
                "{}: {}",
                paired_device.primary_item().label,
                paired_device.primary_item().value_label
            ),
            UiAction::OpenPrimaryItemPicker,
        ),
        (
            format!(
                "{}: {}",
                paired_device.primary_number().label,
                paired_device.primary_number().value_label
            ),
            UiAction::OpenPrimaryNumberPicker,
        ),
        (
            format!(
                "{}: {}",
                paired_device.secondary_group().label,
                paired_device.secondary_group().value_label
            ),
            UiAction::OpenSecondaryGroupPicker,
        ),
        (
            format!(
                "{}: {}",
                paired_device.secondary_item().label,
                paired_device.secondary_item().value_label
            ),
            UiAction::OpenSecondaryItemPicker,
        ),
        (
            format!(
                "{}: {}",
                paired_device.secondary_number().label,
                paired_device.secondary_number().value_label
            ),
            UiAction::OpenSecondaryNumberPicker,
        ),
    ]
}

pub(super) fn legacy_options_panel_button_defs(model: &AppModel) -> Vec<(String, UiAction)> {
    let preferences = model.options_panel.preference_state();
    let input_monitoring_enabled = preferences.toggle_enabled(0);
    let advance_after_rating_enabled = preferences.toggle_enabled(1);
    let destructive_yolo_mode_enabled = preferences.toggle_enabled(2);
    let invert_waveform_scroll_enabled = preferences.toggle_enabled(3);

    vec![
        (
            format!("Auto Rename Identifier: {}", preferences.primary_text_value),
            UiAction::EditDefaultIdentifier,
        ),
        (
            on_off_text("Input Monitor", input_monitoring_enabled),
            UiAction::SetInputMonitoringEnabled {
                enabled: !input_monitoring_enabled,
            },
        ),
        (
            on_off_text("Advance After Rating", advance_after_rating_enabled),
            UiAction::SetAdvanceAfterRatingEnabled {
                enabled: !advance_after_rating_enabled,
            },
        ),
        (
            on_off_text("YOLO Edits", destructive_yolo_mode_enabled),
            UiAction::SetDestructiveYoloMode {
                enabled: !destructive_yolo_mode_enabled,
            },
        ),
        (
            on_off_text("Invert Scroll", invert_waveform_scroll_enabled),
            UiAction::SetInvertWaveformScroll {
                enabled: !invert_waveform_scroll_enabled,
            },
        ),
        (
            format!(
                "Trash Folder: {}",
                preferences.auxiliary_label.as_deref().unwrap_or("Not set")
            ),
            UiAction::PickTrashFolder,
        ),
        (String::from("Open Trash Folder"), UiAction::OpenTrashFolder),
        (String::from("Close"), UiAction::CloseOptionsPanel),
    ]
}

pub(super) fn options_panel_title(model: &AppModel) -> String {
    let paired_device = model.paired_device_panel();
    paired_device
        .active_picker()
        .map(|target| paired_picker_title(paired_device, target))
        .unwrap_or_else(|| String::from("Device Options"))
}

pub(super) fn picker_options(
    model: &AppModel,
    target: PairedPickerTarget,
) -> &[PairedPickerOptionItem] {
    model.paired_device_panel().options_for(target)
}

/// Map one projected paired-picker option into the native action it emits.
pub(super) fn picker_action(value: &PairedPickerValueItem) -> UiAction {
    match value {
        PairedPickerValue::PrimaryGroup(group_id) => UiAction::SetPrimaryGroup {
            group_id: group_id.clone(),
        },
        PairedPickerValue::PrimaryItem(item_name) => UiAction::SetPrimaryItem {
            item_name: item_name.clone(),
        },
        PairedPickerValue::PrimaryNumber(value) => UiAction::SetPrimaryNumber { value: *value },
        PairedPickerValue::SecondaryGroup(group_id) => UiAction::SetSecondaryGroup {
            group_id: group_id.clone(),
        },
        PairedPickerValue::SecondaryItem(item_name) => UiAction::SetSecondaryItem {
            item_name: item_name.clone(),
        },
        PairedPickerValue::SecondaryNumber(value) => UiAction::SetSecondaryNumber { value: *value },
    }
}

/// Return the title text for the active paired-picker target.
fn paired_picker_title(
    paired_device: &PairedStatusPanel<PairedPickerValueItem>,
    target: PairedPickerTarget,
) -> String {
    match target {
        PairedPickerTarget::PrimaryGroup => paired_device.primary_group().label.clone(),
        PairedPickerTarget::PrimaryItem => paired_device.primary_item().label.clone(),
        PairedPickerTarget::PrimaryNumber => paired_device.primary_number().label.clone(),
        PairedPickerTarget::SecondaryGroup => paired_device.secondary_group().label.clone(),
        PairedPickerTarget::SecondaryItem => paired_device.secondary_item().label.clone(),
        PairedPickerTarget::SecondaryNumber => paired_device.secondary_number().label.clone(),
    }
}

fn on_off_text(label: &str, enabled: bool) -> String {
    format!("{label}: {}", if enabled { "On" } else { "Off" })
}
