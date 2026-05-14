//! Child-process popup host management for the popup example.

#[cfg(all(target_os = "windows", not(test)))]
use super::POPUP_POSITION;
use super::{POPUP_ARG, POPUP_MODE_ARG, POPUP_PREWARM_ARG, PopupMode};
#[cfg(all(target_os = "windows", not(test)))]
use std::io::{BufRead, BufReader};
use std::process::Child;
#[cfg(all(target_os = "windows", not(test)))]
use std::process::Stdio;

#[derive(Debug, Default)]
pub(super) struct PopupHosts {
    drag_preview: PopupHost,
    tooltip: PopupHost,
    command_palette: PopupHost,
}

#[derive(Debug, Default)]
struct PopupHost {
    child: Option<Child>,
    mode: Option<PopupMode>,
}

impl PopupHosts {
    fn host_mut(&mut self, mode: PopupMode) -> &mut PopupHost {
        match mode {
            PopupMode::DragPreview => &mut self.drag_preview,
            PopupMode::Tooltip => &mut self.tooltip,
            PopupMode::CommandPalette => &mut self.command_palette,
        }
    }

    pub(super) fn shutdown(&mut self) {
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
        let mut child = spawn_popup_process(mode, true)?;
        self.finish_prewarm(&mut child);
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
            if show_popup_window(child_id, POPUP_POSITION, true) {
                return Ok(());
            }
            if wait_for_popup_window(child_id, std::time::Duration::from_millis(250))
                && show_popup_window(child_id, POPUP_POSITION, true)
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

    #[cfg(all(target_os = "windows", not(test)))]
    fn finish_prewarm(&self, child: &mut Child) {
        wait_for_first_present_profile(child, std::time::Duration::from_secs(3));
        let process_id = child.id();
        let _ = wait_for_hidden_popup_window(process_id, std::time::Duration::from_secs(2));
    }

    #[cfg(all(not(target_os = "windows"), not(test)))]
    fn finish_prewarm(&self, _child: &mut Child) {}

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
pub(super) fn prepare_popup_hosts(hosts: &mut PopupHosts) -> std::result::Result<(), &'static str> {
    for mode in PopupMode::ALL {
        hosts.host_mut(mode).prepare(mode)?;
    }
    if !hosts.wait_until_ready(std::time::Duration::from_secs(5)) {
        return Err("popup hosts did not initialize");
    }
    Ok(())
}

#[cfg(test)]
pub(super) fn prepare_popup_hosts(hosts: &mut PopupHosts) -> std::result::Result<(), &'static str> {
    for mode in PopupMode::ALL {
        hosts.host_mut(mode).mode = Some(mode);
    }
    Ok(())
}

#[cfg(not(test))]
pub(super) fn open_popup_host(
    hosts: &mut PopupHosts,
    mode: PopupMode,
) -> std::result::Result<(), &'static str> {
    hosts.host_mut(mode).open(mode)
}

#[cfg(test)]
pub(super) fn open_popup_host(
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
    let executable = std::env::current_exe().map_err(|_| "could not resolve current executable")?;
    let mut command = std::process::Command::new(executable);
    command.args(popup_process_args(mode, prewarmed));
    if prewarmed {
        command.env("RADIANT_NATIVE_STARTUP_PROFILE", "1");
        #[cfg(target_os = "windows")]
        command.stderr(Stdio::piped());
    }
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
fn wait_for_hidden_popup_window(process_id: u32, timeout: std::time::Duration) -> bool {
    let start = std::time::Instant::now();
    while start.elapsed() < timeout {
        if popup_window_handle(process_id).is_some_and(|hwnd| !is_popup_window_visible(hwnd)) {
            return true;
        }
        std::thread::sleep(std::time::Duration::from_millis(16));
    }
    false
}

#[cfg(target_os = "windows")]
#[cfg(not(test))]
fn wait_for_first_present_profile(child: &mut Child, timeout: std::time::Duration) -> bool {
    let Some(stderr) = child.stderr.take() else {
        return false;
    };
    let (sender, receiver) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let mut lines = BufReader::new(stderr).lines();
        let ready = lines.any(|line| {
            line.is_ok_and(|line| {
                line.starts_with("[native-vello-startup]") && line.contains("first_present_ms=")
            })
        });
        let _ = sender.send(ready);
    });
    receiver.recv_timeout(timeout).unwrap_or(false)
}

#[cfg(target_os = "windows")]
#[cfg(not(test))]
fn show_popup_window(process_id: u32, position: [f32; 2], focus: bool) -> bool {
    let Some(hwnd) = popup_window_handle(process_id) else {
        return false;
    };
    unsafe {
        use windows_sys::Win32::UI::WindowsAndMessaging::{
            SW_SHOW, SW_SHOWNA, SWP_NOACTIVATE, SWP_NOSIZE, SWP_NOZORDER, SetForegroundWindow,
            SetWindowPos, ShowWindow,
        };

        SetWindowPos(
            hwnd,
            std::ptr::null_mut(),
            position[0].round() as i32,
            position[1].round() as i32,
            0,
            0,
            SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE,
        );
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
fn is_popup_window_visible(hwnd: windows_sys::Win32::Foundation::HWND) -> bool {
    unsafe {
        use windows_sys::Win32::UI::WindowsAndMessaging::IsWindowVisible;

        IsWindowVisible(hwnd) != 0
    }
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
pub(super) fn hide_current_popup_window() -> bool {
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
pub(super) fn hide_current_popup_window() -> bool {
    false
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
