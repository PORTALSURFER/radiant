#[cfg(all(target_os = "windows", not(test)))]
use super::platform;
#[cfg(all(target_os = "windows", not(test)))]
use crate::model::POPUP_PREWARM_POSITION;
#[cfg(not(test))]
use std::process::Child;

#[cfg(all(target_os = "windows", not(test)))]
pub(super) fn finish_prewarm_child(child: &mut Child) {
    platform::wait_for_first_present_profile(child, std::time::Duration::from_secs(3));
    let process_id = child.id();
    let _ = platform::wait_for_hidden_popup_window(process_id, std::time::Duration::from_secs(2));
    park_visible_offscreen_show_path(process_id);
}

#[cfg(all(not(target_os = "windows"), not(test)))]
pub(super) fn finish_prewarm_child(_child: &mut Child) {}

#[cfg(all(target_os = "windows", not(test)))]
fn park_visible_offscreen_show_path(process_id: u32) {
    for step in offscreen_visible_prime_steps() {
        if !offscreen_visible_prime_step(process_id, step) {
            return;
        }
    }
}

#[cfg(all(target_os = "windows", not(test)))]
fn offscreen_visible_prime_step(process_id: u32, step: PopupPrimeStep) -> bool {
    match step {
        PopupPrimeStep::Show { focus } => {
            platform::show_popup_window(process_id, POPUP_PREWARM_POSITION, focus)
        }
        PopupPrimeStep::WaitVisible => platform::wait_for_visible_popup_window(
            process_id,
            std::time::Duration::from_millis(250),
        ),
        PopupPrimeStep::Hide => platform::hide_popup_window(process_id),
        PopupPrimeStep::WaitHidden => platform::wait_for_hidden_popup_window(
            process_id,
            std::time::Duration::from_millis(250),
        ),
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PopupPrimeStep {
    Show { focus: bool },
    WaitVisible,
    Hide,
    WaitHidden,
}

fn offscreen_visible_prime_steps() -> [PopupPrimeStep; 6] {
    [
        PopupPrimeStep::Show { focus: false },
        PopupPrimeStep::WaitVisible,
        PopupPrimeStep::Hide,
        PopupPrimeStep::WaitHidden,
        PopupPrimeStep::Show { focus: false },
        PopupPrimeStep::WaitVisible,
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn offscreen_visible_prime_steps_end_with_non_focused_ready_park() {
        assert_eq!(
            offscreen_visible_prime_steps(),
            [
                PopupPrimeStep::Show { focus: false },
                PopupPrimeStep::WaitVisible,
                PopupPrimeStep::Hide,
                PopupPrimeStep::WaitHidden,
                PopupPrimeStep::Show { focus: false },
                PopupPrimeStep::WaitVisible,
            ]
        );
    }
}
