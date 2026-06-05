use crate::gui::input::KeyCode;
use std::time::{Duration, Instant};

const NAVIGATION_KEY_REPEAT_INTERVAL: Duration = Duration::from_millis(45);

pub(super) fn should_route_keypress(
    key: KeyCode,
    repeat: bool,
    allow_text_deletion_repeat: bool,
    last_navigation_repeat: &mut Option<Instant>,
    now: Instant,
) -> bool {
    if !repeat {
        if matches!(key, KeyCode::ArrowUp | KeyCode::ArrowDown) {
            *last_navigation_repeat = None;
        }
        return true;
    }
    if !matches!(key, KeyCode::ArrowUp | KeyCode::ArrowDown) {
        if allow_text_deletion_repeat && matches!(key, KeyCode::Backspace | KeyCode::Delete) {
            return true;
        }
        return false;
    }
    if last_navigation_repeat
        .is_some_and(|last| now.saturating_duration_since(last) < NAVIGATION_KEY_REPEAT_INTERVAL)
    {
        return false;
    }
    *last_navigation_repeat = Some(now);
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repeated_navigation_keys_are_throttled_without_repeating_other_shortcuts() {
        let start = Instant::now();
        let mut last = None;

        assert!(should_route_keypress(
            KeyCode::ArrowDown,
            false,
            false,
            &mut last,
            start
        ));
        assert!(should_route_keypress(
            KeyCode::ArrowDown,
            true,
            false,
            &mut last,
            start
        ));
        assert!(!should_route_keypress(
            KeyCode::ArrowDown,
            true,
            false,
            &mut last,
            start + Duration::from_millis(30)
        ));
        assert!(should_route_keypress(
            KeyCode::ArrowDown,
            true,
            false,
            &mut last,
            start + Duration::from_millis(50)
        ));
        assert!(!should_route_keypress(
            KeyCode::N,
            true,
            false,
            &mut last,
            start + Duration::from_millis(180)
        ));
    }

    #[test]
    fn repeated_backspace_and_delete_are_allowed_only_for_text_deletion() {
        let start = Instant::now();
        let mut last = None;

        assert!(should_route_keypress(
            KeyCode::Backspace,
            true,
            true,
            &mut last,
            start
        ));
        assert!(should_route_keypress(
            KeyCode::Delete,
            true,
            true,
            &mut last,
            start
        ));
        assert!(!should_route_keypress(
            KeyCode::Backspace,
            true,
            false,
            &mut last,
            start
        ));
        assert!(!should_route_keypress(
            KeyCode::Delete,
            true,
            false,
            &mut last,
            start
        ));
    }
}
