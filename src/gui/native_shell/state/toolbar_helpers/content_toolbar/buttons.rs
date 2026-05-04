//! Content toolbar action-button helpers.

use super::super::super::*;

pub(in crate::gui::native_shell::state) fn content_action_buttons(
    layout: &ShellLayout,
    style: &StyleTokens,
    model: &AppModel,
    toolbar: &ContentToolbarLayout,
) -> Vec<ActionButton> {
    let _ = layout;
    if toolbar.action_slots.iter().all(|rect| rect.width() <= 1.0) {
        return Vec::new();
    }
    let mut buttons = Vec::new();
    if toolbar.action_slots[0].width() > 1.0 {
        buttons.push(ActionButton {
            rect: toolbar.action_slots[0],
            label: String::from("Random"),
            icon: Some(ShellSvgIcon::Dice),
            enabled: true,
            active: model.browser_actions.random_navigation_enabled,
            action: UiAction::ToggleRandomNavigationMode,
            text_color: if model.browser_actions.random_navigation_enabled {
                style.highlight_cyan
            } else {
                style.text_primary
            },
        });
    }
    if toolbar.action_slots[1].width() > 1.0 {
        buttons.push(ActionButton {
            rect: toolbar.action_slots[1],
            label: String::from("Cleanup"),
            icon: Some(ShellSvgIcon::Filter),
            enabled: true,
            active: model.browser_actions.duplicate_cleanup_active,
            action: UiAction::ToggleBrowserDuplicateCleanupMode,
            text_color: if model.browser_actions.duplicate_cleanup_active {
                style.highlight_orange
            } else {
                style.text_primary
            },
        });
    }
    if toolbar.action_slots[2].width() > 1.0 {
        buttons.push(ActionButton {
            rect: toolbar.action_slots[2],
            label: model.browser_chrome.pill_editor_label.clone(),
            icon: None,
            enabled: true,
            active: model.browser_actions.pill_editor_open(),
            action: UiAction::ToggleBrowserPillEditor,
            text_color: if model.browser_actions.pill_editor_open() {
                style.highlight_cyan
            } else {
                style.text_primary
            },
        });
    }
    buttons
}
