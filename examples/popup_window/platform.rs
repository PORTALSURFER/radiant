//! Platform window helpers for the popup example host.

#[cfg(all(target_os = "windows", not(test)))]
use std::{
    io::{BufRead, BufReader},
    process::Child,
    time::{Duration, Instant},
};

#[cfg(target_os = "windows")]
pub(crate) fn hide_current_popup_window() -> bool {
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
pub(crate) fn hide_current_popup_window() -> bool {
    false
}

#[cfg(target_os = "windows")]
#[cfg(not(test))]
pub(super) fn wait_for_popup_window(process_id: u32, timeout: Duration) -> bool {
    let start = Instant::now();
    while start.elapsed() < timeout {
        if popup_window_handle(process_id).is_some() {
            return true;
        }
        std::thread::sleep(Duration::from_millis(16));
    }
    false
}

#[cfg(target_os = "windows")]
#[cfg(not(test))]
pub(super) fn wait_for_hidden_popup_window(process_id: u32, timeout: Duration) -> bool {
    let start = Instant::now();
    while start.elapsed() < timeout {
        if popup_window_handle(process_id).is_some_and(|hwnd| !is_popup_window_visible(hwnd)) {
            return true;
        }
        std::thread::sleep(Duration::from_millis(16));
    }
    false
}

#[cfg(target_os = "windows")]
#[cfg(not(test))]
pub(super) fn wait_for_visible_popup_window(process_id: u32, timeout: Duration) -> bool {
    let start = Instant::now();
    while start.elapsed() < timeout {
        if popup_window_handle(process_id).is_some_and(is_popup_window_visible) {
            return true;
        }
        std::thread::sleep(Duration::from_millis(16));
    }
    false
}

#[cfg(target_os = "windows")]
#[cfg(not(test))]
pub(super) fn wait_for_first_present_profile(child: &mut Child, timeout: Duration) -> bool {
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
pub(super) fn show_popup_window(process_id: u32, position: [f32; 2], focus: bool) -> bool {
    let Some(hwnd) = popup_window_handle(process_id) else {
        return false;
    };
    move_window(hwnd, position);
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
pub(super) fn hide_popup_window(process_id: u32) -> bool {
    let Some(hwnd) = popup_window_handle(process_id) else {
        return false;
    };
    unsafe {
        use windows_sys::Win32::UI::WindowsAndMessaging::{SW_HIDE, ShowWindow};

        ShowWindow(hwnd, SW_HIDE);
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
#[cfg(not(test))]
fn move_window(hwnd: windows_sys::Win32::Foundation::HWND, position: [f32; 2]) {
    unsafe {
        use windows_sys::Win32::UI::WindowsAndMessaging::{
            SWP_NOACTIVATE, SWP_NOSIZE, SWP_NOZORDER, SetWindowPos,
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
    }
}
