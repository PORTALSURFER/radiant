use super::*;

const NAVIGATION_KEY_REPEAT_INTERVAL: Duration = Duration::from_millis(45);

pub(super) fn should_route_keypress(
    key: crate::gui::input::KeyCode,
    repeat: bool,
    last_navigation_repeat: &mut Option<Instant>,
    now: Instant,
) -> bool {
    if !repeat {
        if matches!(
            key,
            crate::gui::input::KeyCode::ArrowUp | crate::gui::input::KeyCode::ArrowDown
        ) {
            *last_navigation_repeat = None;
        }
        return true;
    }
    if !matches!(
        key,
        crate::gui::input::KeyCode::ArrowUp | crate::gui::input::KeyCode::ArrowDown
    ) {
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
    use crate::gui::input::KeyCode;

    #[test]
    fn repeated_navigation_keys_are_throttled_without_repeating_other_shortcuts() {
        let start = Instant::now();
        let mut last = None;

        assert!(should_route_keypress(
            KeyCode::ArrowDown,
            false,
            &mut last,
            start
        ));
        assert!(should_route_keypress(
            KeyCode::ArrowDown,
            true,
            &mut last,
            start
        ));
        assert!(!should_route_keypress(
            KeyCode::ArrowDown,
            true,
            &mut last,
            start + Duration::from_millis(30)
        ));
        assert!(should_route_keypress(
            KeyCode::ArrowDown,
            true,
            &mut last,
            start + Duration::from_millis(50)
        ));
        assert!(!should_route_keypress(
            KeyCode::N,
            true,
            &mut last,
            start + Duration::from_millis(180)
        ));
    }
}
