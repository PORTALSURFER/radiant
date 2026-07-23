use super::PointerPressStamp;
use crate::{
    gui::types::Point,
    runtime::Event,
    widgets::{PointerButton, PointerModifiers},
};
use std::time::{Duration, Instant};

const DOUBLE_CLICK_MAX_INTERVAL: Duration = Duration::from_millis(500);
const DOUBLE_CLICK_MAX_DISTANCE: f32 = 5.0;

pub(in crate::gui_runtime::native_vello) fn pointer_press_event(
    last: Option<PointerPressStamp>,
    now: Instant,
    position: Point,
    button: PointerButton,
    modifiers: PointerModifiers,
) -> Event {
    if last.is_some_and(|last| is_double_click(last, now, position, button)) {
        return Event::PointerDoubleClick {
            position,
            button,
            modifiers,
        };
    }
    Event::PointerPress {
        position,
        button,
        modifiers,
    }
}

pub(super) fn is_double_click(
    last: PointerPressStamp,
    now: Instant,
    position: Point,
    button: PointerButton,
) -> bool {
    if last.button != button || now.duration_since(last.at) > DOUBLE_CLICK_MAX_INTERVAL {
        return false;
    }
    let dx = position.x - last.position.x;
    let dy = position.y - last.position.y;
    (dx * dx + dy * dy) <= DOUBLE_CLICK_MAX_DISTANCE * DOUBLE_CLICK_MAX_DISTANCE
}

#[cfg(test)]
mod tests {
    use super::pointer_press_event;
    use crate::{
        gui::types::Point,
        gui_runtime::native_vello::generic_runtime::PointerPressStamp,
        runtime::Event,
        widgets::{PointerButton, PointerModifiers},
    };
    use std::time::{Duration, Instant};

    #[test]
    fn nearby_repeated_press_routes_as_double_click() {
        let now = Instant::now();
        let position = Point::new(12.0, 18.0);
        let last = PointerPressStamp {
            at: now - Duration::from_millis(120),
            position: Point::new(10.0, 16.0),
            button: PointerButton::Primary,
        };

        assert_eq!(
            pointer_press_event(
                Some(last),
                now,
                position,
                PointerButton::Primary,
                PointerModifiers::default()
            ),
            Event::PointerDoubleClick {
                position,
                button: PointerButton::Primary,
                modifiers: PointerModifiers::default(),
            }
        );
    }

    #[test]
    fn stale_distant_or_different_button_press_routes_as_single_press() {
        let now = Instant::now();
        let position = Point::new(12.0, 18.0);
        let stale = PointerPressStamp {
            at: now - Duration::from_millis(650),
            position,
            button: PointerButton::Primary,
        };
        let distant = PointerPressStamp {
            at: now - Duration::from_millis(120),
            position: Point::new(40.0, 18.0),
            button: PointerButton::Primary,
        };
        let different_button = PointerPressStamp {
            at: now - Duration::from_millis(120),
            position,
            button: PointerButton::Secondary,
        };

        for last in [stale, distant, different_button] {
            assert_eq!(
                pointer_press_event(
                    Some(last),
                    now,
                    position,
                    PointerButton::Primary,
                    PointerModifiers::default()
                ),
                Event::PointerPress {
                    position,
                    button: PointerButton::Primary,
                    modifiers: PointerModifiers::default(),
                }
            );
        }
    }
}
