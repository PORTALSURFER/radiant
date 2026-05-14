//! Normal-window launcher for a real floating popup Radiant surface.
//!
//! Run `cargo run --example popup_window`, then use the main window controls to
//! reveal a second borderless popup window. Drag the popup from its title area
//! to reposition it, or hide it from the popup itself. The popup is the same
//! example binary relaunched with `--popup`. After the launcher window starts,
//! it prepares one offscreen popup host for every mode, so the user-facing open
//! path only moves and focuses an already prepared native surface.

use radiant::prelude::*;
use std::time::Duration;

#[path = "popup_window/host.rs"]
mod host;
#[path = "popup_window/model.rs"]
mod model;
#[path = "popup_window/policy.rs"]
mod policy;

use host::{PopupHosts, hide_current_popup_window, open_popup_host, prepare_popup_hosts};
#[cfg(test)]
use model::{POPUP_POSITION, POPUP_PREWARM_POSITION, PopupLaunch};
use model::{PopupMode, popup_launch_from_args};
#[cfg(test)]
use policy::popup_policy;
use policy::popup_policy_for_launch;
#[cfg(test)]
use policy::popup_spec;

struct LauncherState {
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
enum LauncherMessage {
    PreparePopups,
    PopupsPrepared(Box<PreparedPopupHosts>),
    SelectMode(PopupMode),
    OpenPopup,
}

#[derive(Debug)]
struct PreparedPopupHosts {
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

#[derive(Clone, Debug)]
struct PopupState {
    mode: PopupMode,
    pinned: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PopupMessage {
    TogglePinned,
    Close,
}

fn main() -> radiant::Result {
    if popup_launch_from_args().is_some() {
        run_popup_window()
    } else {
        run_launcher_window()
    }
}

fn run_launcher_window() -> radiant::Result {
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

fn run_popup_window() -> radiant::Result {
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
    .update_with(update_popup)
    .run()
}

fn launcher_view(state: &mut LauncherState) -> View<LauncherMessage> {
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

fn update_launcher(
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

fn popup_view(state: &mut PopupState) -> View<PopupMessage> {
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
            text(state.mode.label())
                .id(12)
                .key("title")
                .height(26.0)
                .fill_width(),
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
                .key("close")
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

fn update_popup(
    state: &mut PopupState,
    message: PopupMessage,
    context: &mut UpdateContext<PopupMessage>,
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
mod tests {
    use super::*;
    use radiant::{runtime::UiSurface, widgets::TextWidget};

    #[test]
    fn popup_policy_describes_focused_transient_window() {
        let policy = popup_policy(true);

        assert_eq!(policy.position, Some(POPUP_POSITION));
        assert!(policy.transparent);
        assert!(policy.always_on_top);
        assert!(policy.initially_focused);
        assert!(policy.skip_taskbar);
        assert!(policy.initially_visible);
        assert_eq!(policy.drag_region_height, Some(38.0));
        assert!(!policy.resizable);
    }

    #[test]
    fn popup_policy_can_prepare_rendered_transient_window_offscreen() {
        let policy = popup_policy_for_launch(PopupLaunch {
            mode: PopupMode::Tooltip,
            prewarmed: true,
        });

        assert!(policy.initially_visible);
        assert!(policy.hide_after_first_present);
        assert!(!policy.initially_focused);
        assert_eq!(policy.position, Some(POPUP_PREWARM_POSITION));
        assert!(policy.always_on_top);
        assert!(policy.skip_taskbar);
    }

    #[test]
    fn popup_spec_uses_borderless_popup_window_options() {
        let spec = popup_spec();

        assert!(spec.is_popup());
        assert_eq!(spec.key, "workflow-popup");
        assert_eq!(spec.inner_size(), Some([340.0, 156.0]));
        assert_eq!(
            spec.popup_options().and_then(|popup| popup.position),
            Some(POPUP_POSITION)
        );
        assert!(!spec.native_options().decorations);
        assert!(!spec.drag_and_drop_enabled());
    }

    #[test]
    fn launcher_view_tracks_selected_popup_mode_and_status() {
        let mut state = LauncherState::default();
        state.popups_ready = true;
        update_launcher(
            &mut state,
            LauncherMessage::SelectMode(PopupMode::CommandPalette),
            &mut UpdateContext::default(),
        );

        let view = launcher_view(&mut state).into_surface();

        assert_eq!(text(&view, 11).text, "Popup workflow");
        assert_eq!(state.selected_mode, PopupMode::CommandPalette);
        assert!(state.status.contains("Command palette"));
    }

    #[test]
    fn popup_view_switches_between_modes_and_exposes_close_button() {
        let mut state = PopupState {
            mode: PopupMode::Tooltip,
            pinned: false,
        };
        let tooltip_view = popup_view(&mut state).into_surface();
        assert_eq!(text(&tooltip_view, 12).text, "Tooltip");
        assert!(tooltip_view.find_widget(18).is_some());

        state.mode = PopupMode::CommandPalette;
        let command_view = popup_view(&mut state).into_surface();
        assert_eq!(text(&command_view, 12).text, "Command palette");
    }

    fn text<Message>(surface: &UiSurface<Message>, id: u64) -> &TextWidget {
        surface
            .find_widget(id)
            .expect("text widget should exist")
            .widget()
            .as_any()
            .downcast_ref::<TextWidget>()
            .expect("widget should be text")
    }
}
