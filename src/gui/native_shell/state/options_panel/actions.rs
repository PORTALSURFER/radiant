//! Options-panel action definitions and button-label formatting.

use super::*;

pub(super) fn options_panel_button_defs(model: &AppModel) -> [(&'static str, UiAction); 7] {
    [
        (
            "Input Monitor",
            UiAction::SetInputMonitoringEnabled {
                enabled: !model.options_panel.input_monitoring_enabled,
            },
        ),
        (
            "Advance After Rating",
            UiAction::SetAdvanceAfterRatingEnabled {
                enabled: !model.options_panel.advance_after_rating_enabled,
            },
        ),
        (
            "YOLO Edits",
            UiAction::SetDestructiveYoloMode {
                enabled: !model.options_panel.destructive_yolo_mode_enabled,
            },
        ),
        (
            "Invert Scroll",
            UiAction::SetInvertWaveformScroll {
                enabled: !model.options_panel.invert_waveform_scroll_enabled,
            },
        ),
        ("Set Trash Folder", UiAction::PickTrashFolder),
        ("Open Trash Folder", UiAction::OpenTrashFolder),
        ("Close", UiAction::CloseOptionsPanel),
    ]
}

pub(super) fn options_panel_button_text(label: &str, model: &AppModel) -> String {
    match label {
        "Input Monitor" => on_off_text(label, model.options_panel.input_monitoring_enabled),
        "Advance After Rating" => {
            on_off_text(label, model.options_panel.advance_after_rating_enabled)
        }
        "YOLO Edits" => on_off_text(label, model.options_panel.destructive_yolo_mode_enabled),
        "Invert Scroll" => on_off_text(label, model.options_panel.invert_waveform_scroll_enabled),
        "Set Trash Folder" => format!(
            "Trash Folder: {}",
            model
                .options_panel
                .trash_folder_label
                .as_deref()
                .unwrap_or("Not set")
        ),
        _ => String::from(label),
    }
}

fn on_off_text(label: &str, enabled: bool) -> String {
    format!("{label}: {}", if enabled { "On" } else { "Off" })
}
