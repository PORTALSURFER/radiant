use super::*;
use crate::host::{PopupHosts, open_popup_host, prepare_popup_hosts};
use crate::model::PopupMode;

pub(super) struct LauncherState {
    selected_mode: PopupMode,
    launches: usize,
    status: String,
    popup_hosts: PopupHosts,
    popups_ready: bool,
}

impl Default for LauncherState {
    fn default() -> Self {
        Self {
            selected_mode: PopupMode::DragPreview,
            launches: 0,
            status: String::from("Preparing popup surfaces."),
            popup_hosts: PopupHosts::default(),
            popups_ready: false,
        }
    }
}

impl LauncherState {
    fn install_prepared_popups(&mut self, prepared: PreparedPopupHosts) {
        self.popup_hosts = prepared.hosts;
        self.popups_ready = prepared.ready;
        self.status = match prepared.result {
            Ok(()) => String::from("Ready to open any popup instantly."),
            Err(error) => format!("Popup prep failed: {error}"),
        };
    }
}

#[derive(Debug)]
pub(super) enum LauncherMessage {
    PreparePopups,
    PopupsPrepared(Box<PreparedPopupHosts>),
    SelectMode(PopupMode),
    OpenPopup,
}

#[derive(Debug)]
pub(super) struct PreparedPopupHosts {
    hosts: PopupHosts,
    result: std::result::Result<(), &'static str>,
    ready: bool,
}

fn prepare_popup_hosts_for_install() -> PreparedPopupHosts {
    let mut hosts = PopupHosts::default();
    let result = prepare_popup_hosts(&mut hosts);
    let ready = result.is_ok();
    PreparedPopupHosts {
        hosts,
        result,
        ready,
    }
}

pub(super) fn run_launcher_window() -> radiant::Result {
    radiant::app(LauncherState::default())
        .title("Radiant Popup Launcher")
        .size(520, 300)
        .min_size(440, 260)
        .view(launcher_view)
        .on_startup(|_state, context| {
            context.after(Duration::from_millis(50), LauncherMessage::PreparePopups);
        })
        .on_shutdown(|state| {
            state.popup_hosts.shutdown();
            None
        })
        .on_close_requested(|state| {
            state.popup_hosts.shutdown();
            true
        })
        .update_with(update_launcher)
        .run()
}

pub(super) fn launcher_view(state: &mut LauncherState) -> View<LauncherMessage> {
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

pub(super) fn update_launcher(
    state: &mut LauncherState,
    message: LauncherMessage,
    context: &mut UpdateContext<LauncherMessage>,
) {
    match message {
        LauncherMessage::PreparePopups => {
            state.status = String::from("Preparing popup surfaces.");
            context.spawn(
                "popup-window-prewarm",
                prepare_popup_hosts_for_install,
                |prepared| LauncherMessage::PopupsPrepared(Box::new(prepared)),
            );
        }
        LauncherMessage::PopupsPrepared(prepared) => {
            state.install_prepared_popups(*prepared);
        }
        LauncherMessage::SelectMode(mode) => {
            state.selected_mode = mode;
            state.status = if state.popups_ready {
                format!("Ready to open {} instantly.", mode.label())
            } else {
                String::from("Preparing popup surfaces.")
            };
        }
        LauncherMessage::OpenPopup => {
            if !state.popups_ready {
                state.status = String::from("Popup surfaces are still preparing.");
                context.request_repaint();
                return;
            }
            state.launches += 1;
            match open_popup_host(&mut state.popup_hosts, state.selected_mode) {
                Ok(()) => {
                    state.status = format!("Opened {}.", state.selected_mode.label());
                }
                Err(error) => {
                    state.status = format!("Popup failed: {error}");
                }
            }
        }
    }
    context.request_repaint();
}

#[cfg(test)]
impl LauncherState {
    pub(super) fn mark_popups_ready_for_test(&mut self) {
        self.popups_ready = true;
    }

    pub(super) fn selected_mode(&self) -> PopupMode {
        self.selected_mode
    }

    pub(super) fn status(&self) -> &str {
        &self.status
    }
}
