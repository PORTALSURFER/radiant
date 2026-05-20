//! Platform window helpers for the popup example host.

#[path = "platform/readiness.rs"]
mod readiness;

#[cfg(target_os = "windows")]
#[cfg(not(test))]
pub(super) use readiness::{
    wait_for_first_present_profile, wait_for_hidden_popup_window, wait_for_popup_window,
    wait_for_visible_popup_window,
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
pub(super) fn focus_popup_window(process_id: u32) -> bool {
    let Some(hwnd) = popup_window_handle(process_id) else {
        return false;
    };
    unsafe {
        use windows_sys::Win32::UI::WindowsAndMessaging::{
            SW_SHOW, SetForegroundWindow, ShowWindow,
        };

        ShowWindow(hwnd, SW_SHOW);
        SetForegroundWindow(hwnd);
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
pub(super) fn is_popup_window_visible(hwnd: windows_sys::Win32::Foundation::HWND) -> bool {
    unsafe {
        use windows_sys::Win32::UI::WindowsAndMessaging::IsWindowVisible;

        IsWindowVisible(hwnd) != 0
    }
}

#[cfg(target_os = "windows")]
#[cfg(not(test))]
pub(super) fn popup_window_handle(process_id: u32) -> Option<windows_sys::Win32::Foundation::HWND> {
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
