//! Launcher surface composition for the popup window example.

use super::{LauncherMessage, LauncherState};
use crate::model::PopupMode;
use radiant::prelude::*;

pub(crate) fn launcher_view(state: &mut LauncherState) -> View<LauncherMessage> {
    column([
        text("Popup workflow")
            .id(11)
            .key("title")
            .height(32.0)
            .fill_width(),
        text("Open a real popup window, drag its title area, then close it from inside the popup.")
            .key("description")
            .wrap()
            .height(42.0)
            .fill_width(),
        row([
            mode_button(state, PopupMode::DragPreview),
            mode_button(state, PopupMode::Tooltip),
            mode_button(state, PopupMode::CommandPalette),
        ])
        .key("modes")
        .spacing(8.0)
        .fill_width(),
        row([
            button("Open popup")
                .mapped(|_| LauncherMessage::OpenPopup)
                .primary()
                .id(14)
                .key("open")
                .size(132.0, 34.0),
            text(format!("Launches: {}", state.launches))
                .id(15)
                .height(30.0),
            text(state.status.clone())
                .key("status")
                .truncate()
                .height(30.0)
                .fill_width(),
        ])
        .key("actions")
        .spacing(10.0)
        .fill_width(),
        text("Current native runtime opens one window per run; this example prewarms one child-process popup surface per mode as the host-owned multi-window adapter.")
            .key("boundary")
            .wrap()
            .height(48.0)
            .fill_width(),
    ])
    .key("launcher-root")
    .padding(18.0)
    .spacing(12.0)
    .fill()
}

fn mode_button(state: &LauncherState, mode: PopupMode) -> View<LauncherMessage> {
    let builder = button(mode.label())
        .mapped(move |_| LauncherMessage::SelectMode(mode))
        .key(mode.arg())
        .size(148.0, 32.0);
    if state.selected_mode == mode {
        builder.primary()
    } else {
        builder.subtle()
    }
}
