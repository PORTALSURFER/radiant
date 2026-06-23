use super::*;
use crate::host::hide_current_popup_window;
use crate::model::{PopupMode, popup_launch_from_args};
use crate::policy::popup_policy_for_launch;

#[derive(Clone, Debug)]
pub(super) struct PopupState {
    mode: PopupMode,
    pinned: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum PopupMessage {
    TogglePinned,
    Close,
}

pub(super) fn run_popup_window() -> radiant::Result {
    let launch = popup_launch_from_args().unwrap_or_default();
    radiant::app(PopupState {
        mode: launch.mode,
        pinned: false,
    })
    .title("Radiant Floating Popup")
    .size(340, 156)
    .floating_popup()
    .popup_policy(popup_policy_for_launch(launch))
    .view(popup_view)
    .handle_message(update_popup)
    .run()
}

pub(super) fn popup_view(state: &mut PopupState) -> View<PopupMessage> {
    let pinned_badge = if state.pinned {
        badge("Pinned")
            .primary()
            .message(PopupMessage::TogglePinned)
            .key("pinned")
            .size(88.0, 26.0)
    } else {
        badge(state.mode.badge())
            .subtle()
            .message(PopupMessage::TogglePinned)
            .key("pinned")
            .size(88.0, 26.0)
    };

    column([
        row([
            pinned_badge,
            text(state.mode.label()).id(12).height(26.0).fill_width(),
        ])
        .key("header")
        .spacing(8.0)
        .fill_width(),
        text("Drag this title area to move the popup.")
            .key("drag-hint")
            .height(18.0)
            .fill_width(),
        text(state.mode.detail())
            .key("detail")
            .wrap()
            .height(34.0)
            .fill_width(),
        row([
            toggle("Pin", state.pinned)
                .message(|_| PopupMessage::TogglePinned)
                .key("pin")
                .size(82.0, 30.0),
            button("Close")
                .message(PopupMessage::Close)
                .danger()
                .id(18)
                .size(92.0, 30.0),
        ])
        .key("actions")
        .spacing(8.0)
        .fill_width(),
    ])
    .key("popup-root")
    .style(WidgetStyle::default())
    .padding(12.0)
    .spacing(8.0)
    .fill()
}

pub(super) fn update_popup(
    state: &mut PopupState,
    message: PopupMessage,
    context: &mut UiUpdateContext<PopupMessage>,
) {
    match message {
        PopupMessage::TogglePinned => state.pinned = !state.pinned,
        PopupMessage::Close => {
            if !hide_current_popup_window() {
                context.exit();
            }
        }
    }
}

#[cfg(test)]
impl PopupState {
    pub(super) fn new_for_test(mode: PopupMode, pinned: bool) -> Self {
        Self { mode, pinned }
    }

    pub(super) fn set_mode_for_test(&mut self, mode: PopupMode) {
        self.mode = mode;
    }
}
