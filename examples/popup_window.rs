//! Normal-window launcher for a real floating popup Radiant surface.
//!
//! Run `cargo run --example popup_window`, then use the main window controls to
//! reveal a second borderless popup window. Drag the popup from its title area
//! to reposition it, or hide it from the popup itself. The popup is the same
//! example binary relaunched with `--popup`, then prewarmed hidden per mode so
//! the user-facing open path only reveals an already prepared native surface.

use radiant::prelude::*;
use std::{env, process::Child};

const POPUP_ARG: &str = "--popup";
const POPUP_MODE_ARG: &str = "--popup-mode";
const POPUP_PREWARM_ARG: &str = "--popup-prewarm";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PopupMode {
    DragPreview,
    Tooltip,
    CommandPalette,
}

impl PopupMode {
    const ALL: [Self; 3] = [Self::DragPreview, Self::Tooltip, Self::CommandPalette];

    fn arg(self) -> &'static str {
        match self {
            Self::DragPreview => "drag-preview",
            Self::Tooltip => "tooltip",
            Self::CommandPalette => "command-palette",
        }
    }

    fn from_arg(value: &str) -> Self {
        match value {
            "tooltip" => Self::Tooltip,
            "command-palette" => Self::CommandPalette,
            _ => Self::DragPreview,
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::DragPreview => "Drag preview",
            Self::Tooltip => "Tooltip",
            Self::CommandPalette => "Command palette",
        }
    }

    fn detail(self) -> &'static str {
        match self {
            Self::DragPreview => "Audio Clip 04 is being dragged outside the main window.",
            Self::Tooltip => "Transient help can use the same borderless popup surface.",
            Self::CommandPalette => "Popup windows can host focused command surfaces too.",
        }
    }

    fn badge(self) -> &'static str {
        match self {
            Self::DragPreview => "Dragging",
            Self::Tooltip => "Hint",
            Self::CommandPalette => "Commands",
        }
    }
}

struct LauncherState {
    selected_mode: PopupMode,
    launches: usize,
    status: String,
    popup_hosts: PopupHosts,
}

impl Default for LauncherState {
    fn default() -> Self {
        Self {
            selected_mode: PopupMode::DragPreview,
            launches: 0,
            status: String::from("Preparing popup surfaces."),
            popup_hosts: PopupHosts::default(),
        }
    }
}

impl LauncherState {
    fn with_prewarmed_popups() -> Self {
        let mut state = Self::default();
        state.status = match prepare_popup_hosts(&mut state.popup_hosts) {
            Ok(()) => String::from("Ready to open any popup instantly."),
            Err(error) => format!("Popup prep failed: {error}"),
        };
        state
    }
}

#[derive(Debug, Default)]
struct PopupHosts {
    drag_preview: PopupHost,
    tooltip: PopupHost,
    command_palette: PopupHost,
}

#[derive(Debug, Default)]
struct PopupHost {
    child: Option<Child>,
    mode: Option<PopupMode>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum LauncherMessage {
    SelectMode(PopupMode),
    OpenPopup,
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
    radiant::app(LauncherState::with_prewarmed_popups())
        .title("Radiant Popup Launcher")
        .size(520, 300)
        .min_size(440, 260)
        .view(launcher_view)
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
    .popup_policy(popup_policy(!launch.prewarmed))
    .view(popup_view)
    .update_with(update_popup)
    .run()
}

fn popup_policy(initially_visible: bool) -> NativePopupOptions {
    NativePopupOptions::default()
        .position(460.0, 280.0)
        .transparent(true)
        .always_on_top(true)
        .initially_focused(initially_visible)
        .skip_taskbar(true)
        .initially_visible(initially_visible)
        .drag_region_height(38.0)
}

#[cfg(test)]
fn popup_spec() -> WindowSpec {
    WindowSpec::popup("workflow-popup", "Radiant Floating Popup")
        .logical_size(340.0, 156.0)
        .popup_policy(popup_policy(true))
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
                .message(LauncherMessage::OpenPopup)
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
        .message(LauncherMessage::SelectMode(mode))
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
        LauncherMessage::SelectMode(mode) => {
            state.selected_mode = mode;
            state.status = format!("Ready to open {} instantly.", mode.label());
        }
        LauncherMessage::OpenPopup => {
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct PopupLaunch {
    mode: PopupMode,
    prewarmed: bool,
}

impl Default for PopupLaunch {
    fn default() -> Self {
        Self {
            mode: PopupMode::DragPreview,
            prewarmed: false,
        }
    }
}

fn popup_launch_from_args() -> Option<PopupLaunch> {
    let mut args = env::args().skip(1);
    let mut popup = false;
    let mut prewarmed = false;
    let mut mode = PopupMode::DragPreview;
    while let Some(arg) = args.next() {
        if arg == POPUP_ARG {
            popup = true;
        } else if arg == POPUP_PREWARM_ARG {
            prewarmed = true;
        } else if arg == POPUP_MODE_ARG
            && let Some(value) = args.next()
        {
            mode = PopupMode::from_arg(value.as_str());
        }
    }
    popup.then_some(PopupLaunch { mode, prewarmed })
}

impl PopupHosts {
    fn host_mut(&mut self, mode: PopupMode) -> &mut PopupHost {
        match mode {
            PopupMode::DragPreview => &mut self.drag_preview,
            PopupMode::Tooltip => &mut self.tooltip,
            PopupMode::CommandPalette => &mut self.command_palette,
        }
    }

    fn shutdown(&mut self) {
        for mode in PopupMode::ALL {
            self.host_mut(mode).shutdown();
        }
    }

    #[cfg(not(test))]
    fn wait_until_ready(&mut self, timeout: std::time::Duration) -> bool {
        let deadline = std::time::Instant::now() + timeout;
        PopupMode::ALL.into_iter().all(|mode| {
            let now = std::time::Instant::now();
            if now >= deadline {
                return false;
            }
            self.host_mut(mode).wait_until_ready(deadline - now)
        })
    }
}

impl PopupHost {
    #[cfg(not(test))]
    fn prepare(&mut self, mode: PopupMode) -> std::result::Result<(), &'static str> {
        self.reap_finished_child();
        if self.mode == Some(mode) && self.child.is_some() {
            return Ok(());
        }

        self.shutdown();
        let child = spawn_popup_process(mode, true)?;
        self.mode = Some(mode);
        self.child = Some(child);
        Ok(())
    }

    #[cfg(not(test))]
    fn open(&mut self, mode: PopupMode) -> std::result::Result<(), &'static str> {
        self.prepare(mode)?;

        #[cfg(target_os = "windows")]
        {
            let child_id = self
                .child
                .as_ref()
                .map(Child::id)
                .ok_or("popup host is not running")?;
            if show_popup_window(child_id, true) {
                return Ok(());
            }
            if wait_for_popup_window(child_id, std::time::Duration::from_millis(250))
                && show_popup_window(child_id, true)
            {
                return Ok(());
            }
        }

        self.shutdown();
        self.child = Some(spawn_popup_process(mode, false)?);
        self.mode = Some(mode);
        Ok(())
    }

    fn shutdown(&mut self) {
        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
        self.mode = None;
    }

    #[cfg(not(test))]
    fn reap_finished_child(&mut self) {
        let finished = self
            .child
            .as_mut()
            .and_then(|child| child.try_wait().ok())
            .flatten()
            .is_some();
        if finished {
            self.child = None;
            self.mode = None;
        }
    }

    #[cfg(not(test))]
    fn wait_until_ready(&self, timeout: std::time::Duration) -> bool {
        let Some(process_id) = self.child.as_ref().map(Child::id) else {
            return false;
        };

        #[cfg(target_os = "windows")]
        {
            wait_for_popup_window(process_id, timeout)
        }

        #[cfg(not(target_os = "windows"))]
        {
            let _ = timeout;
            true
        }
    }
}

#[cfg(not(test))]
fn prepare_popup_hosts(hosts: &mut PopupHosts) -> std::result::Result<(), &'static str> {
    for mode in PopupMode::ALL {
        hosts.host_mut(mode).prepare(mode)?;
    }
    if !hosts.wait_until_ready(std::time::Duration::from_secs(5)) {
        return Err("popup hosts did not initialize");
    }
    Ok(())
}

#[cfg(test)]
fn prepare_popup_hosts(hosts: &mut PopupHosts) -> std::result::Result<(), &'static str> {
    for mode in PopupMode::ALL {
        hosts.host_mut(mode).mode = Some(mode);
    }
    Ok(())
}

#[cfg(not(test))]
fn open_popup_host(
    hosts: &mut PopupHosts,
    mode: PopupMode,
) -> std::result::Result<(), &'static str> {
    hosts.host_mut(mode).open(mode)
}

#[cfg(test)]
fn open_popup_host(
    hosts: &mut PopupHosts,
    mode: PopupMode,
) -> std::result::Result<(), &'static str> {
    hosts.host_mut(mode).mode = Some(mode);
    Ok(())
}

#[cfg(not(test))]
fn spawn_popup_process(
    mode: PopupMode,
    prewarmed: bool,
) -> std::result::Result<Child, &'static str> {
    let executable = env::current_exe().map_err(|_| "could not resolve current executable")?;
    let mut command = std::process::Command::new(executable);
    command.args(popup_process_args(mode, prewarmed));
    command.spawn().map_err(|_| "could not start popup process")
}

fn popup_process_args(mode: PopupMode, prewarmed: bool) -> Vec<&'static str> {
    let mut args = vec![POPUP_ARG, POPUP_MODE_ARG, mode.arg()];
    if prewarmed {
        args.push(POPUP_PREWARM_ARG);
    }
    args
}

#[cfg(target_os = "windows")]
#[cfg(not(test))]
fn wait_for_popup_window(process_id: u32, timeout: std::time::Duration) -> bool {
    let start = std::time::Instant::now();
    while start.elapsed() < timeout {
        if popup_window_handle(process_id).is_some() {
            return true;
        }
        std::thread::sleep(std::time::Duration::from_millis(16));
    }
    false
}

#[cfg(target_os = "windows")]
#[cfg(not(test))]
fn show_popup_window(process_id: u32, focus: bool) -> bool {
    let Some(hwnd) = popup_window_handle(process_id) else {
        return false;
    };
    unsafe {
        use windows_sys::Win32::UI::WindowsAndMessaging::{
            SW_SHOW, SW_SHOWNA, SetForegroundWindow, ShowWindow,
        };

        let command = if focus { SW_SHOW } else { SW_SHOWNA };
        ShowWindow(hwnd, command);
        if focus {
            SetForegroundWindow(hwnd);
        }
    }
    true
}

#[cfg(target_os = "windows")]
#[cfg(not(test))]
fn popup_window_handle(process_id: u32) -> Option<windows_sys::Win32::Foundation::HWND> {
    use windows_sys::Win32::Foundation::{FALSE, HWND, LPARAM, TRUE};
    use windows_sys::Win32::UI::WindowsAndMessaging::{EnumWindows, GetWindowThreadProcessId};

    struct Search {
        process_id: u32,
        hwnd: HWND,
    }

    unsafe extern "system" fn enum_window(hwnd: HWND, lparam: LPARAM) -> i32 {
        let search = unsafe { &mut *(lparam as *mut Search) };
        let mut window_process_id = 0;
        unsafe {
            GetWindowThreadProcessId(hwnd, &mut window_process_id);
        }
        if window_process_id == search.process_id {
            search.hwnd = hwnd;
            return FALSE;
        }
        TRUE
    }

    let mut search = Search {
        process_id,
        hwnd: std::ptr::null_mut(),
    };
    unsafe {
        EnumWindows(Some(enum_window), &mut search as *mut Search as LPARAM);
    }
    (!search.hwnd.is_null()).then_some(search.hwnd)
}

#[cfg(target_os = "windows")]
fn hide_current_popup_window() -> bool {
    unsafe {
        use windows_sys::Win32::UI::WindowsAndMessaging::{
            GetForegroundWindow, SW_HIDE, ShowWindow,
        };

        let hwnd = GetForegroundWindow();
        if hwnd.is_null() {
            return false;
        }
        ShowWindow(hwnd, SW_HIDE);
        true
    }
}

#[cfg(not(target_os = "windows"))]
fn hide_current_popup_window() -> bool {
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use radiant::{runtime::UiSurface, widgets::TextWidget};

    #[test]
    fn popup_policy_describes_focused_transient_window() {
        let policy = popup_policy(true);

        assert_eq!(policy.position, Some([460.0, 280.0]));
        assert!(policy.transparent);
        assert!(policy.always_on_top);
        assert!(policy.initially_focused);
        assert!(policy.skip_taskbar);
        assert!(policy.initially_visible);
        assert_eq!(policy.drag_region_height, Some(38.0));
        assert!(!policy.resizable);
    }

    #[test]
    fn popup_policy_can_prepare_hidden_transient_window() {
        let policy = popup_policy(false);

        assert!(!policy.initially_visible);
        assert!(!policy.initially_focused);
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
            Some([460.0, 280.0])
        );
        assert!(!spec.native_options().decorations);
        assert!(!spec.drag_and_drop_enabled());
    }

    #[test]
    fn popup_process_args_mark_prewarmed_hosts() {
        assert_eq!(
            popup_process_args(PopupMode::Tooltip, true),
            vec![
                POPUP_ARG,
                POPUP_MODE_ARG,
                PopupMode::Tooltip.arg(),
                POPUP_PREWARM_ARG
            ]
        );
        assert_eq!(
            popup_process_args(PopupMode::Tooltip, false),
            vec![POPUP_ARG, POPUP_MODE_ARG, PopupMode::Tooltip.arg()]
        );
    }

    #[test]
    fn popup_hosts_prepare_all_modes_without_replacing_on_selection() {
        let mut hosts = PopupHosts::default();

        prepare_popup_hosts(&mut hosts).expect("test prewarm should succeed");

        assert_eq!(hosts.drag_preview.mode, Some(PopupMode::DragPreview));
        assert_eq!(hosts.tooltip.mode, Some(PopupMode::Tooltip));
        assert_eq!(hosts.command_palette.mode, Some(PopupMode::CommandPalette));
    }

    #[test]
    fn launcher_view_tracks_selected_popup_mode_and_status() {
        let mut state = LauncherState::default();
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
