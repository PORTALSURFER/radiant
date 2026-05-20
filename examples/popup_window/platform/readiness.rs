//! Popup readiness polling for the popup example host.

#[cfg(all(target_os = "windows", not(test)))]
use std::{
    io::{BufRead, BufReader},
    process::Child,
    time::{Duration, Instant},
};

#[cfg(all(target_os = "windows", not(test)))]
use super::{is_popup_window_visible, popup_window_handle};

#[cfg(all(target_os = "windows", not(test)))]
pub(in crate::host) fn wait_for_popup_window(process_id: u32, timeout: Duration) -> bool {
    wait_until(timeout, || popup_window_handle(process_id).is_some())
}

#[cfg(all(target_os = "windows", not(test)))]
pub(in crate::host) fn wait_for_hidden_popup_window(process_id: u32, timeout: Duration) -> bool {
    wait_until(timeout, || {
        popup_window_handle(process_id).is_some_and(|hwnd| !is_popup_window_visible(hwnd))
    })
}

#[cfg(all(target_os = "windows", not(test)))]
pub(in crate::host) fn wait_for_visible_popup_window(process_id: u32, timeout: Duration) -> bool {
    wait_until(timeout, || {
        popup_window_handle(process_id).is_some_and(is_popup_window_visible)
    })
}

#[cfg(all(target_os = "windows", not(test)))]
pub(in crate::host) fn wait_for_first_present_profile(
    child: &mut Child,
    timeout: Duration,
) -> bool {
    let Some(stderr) = child.stderr.take() else {
        return false;
    };
    let (sender, receiver) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let mut lines = BufReader::new(stderr).lines();
        let ready = lines.any(|line| line.is_ok_and(|line| is_first_present_profile_line(&line)));
        let _ = sender.send(ready);
    });
    receiver.recv_timeout(timeout).unwrap_or(false)
}

#[cfg(all(target_os = "windows", not(test)))]
fn wait_until(timeout: Duration, mut ready: impl FnMut() -> bool) -> bool {
    let start = Instant::now();
    while start.elapsed() < timeout {
        if ready() {
            return true;
        }
        std::thread::sleep(Duration::from_millis(16));
    }
    false
}

#[cfg_attr(not(all(target_os = "windows", not(test))), allow(dead_code))]
fn is_first_present_profile_line(line: &str) -> bool {
    line.starts_with("[native-vello-startup]") && line.contains("first_present_ms=")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_present_profile_line_requires_native_startup_prefix_and_metric() {
        assert!(is_first_present_profile_line(
            "[native-vello-startup] first_present_ms=14.5 total_ms=30.0"
        ));
        assert!(!is_first_present_profile_line(
            "[native-vello-startup] surface_created_ms=10.0"
        ));
        assert!(!is_first_present_profile_line(
            "[other] first_present_ms=14.5"
        ));
    }
}
