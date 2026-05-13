//! Timer duration helpers.

use std::time::{Duration, Instant};

pub(super) fn due_in(delay: Duration) -> Instant {
    let now = Instant::now();
    now.checked_add(delay).unwrap_or(now)
}

pub(in crate::application::runtime) fn min_timer_delay() -> Duration {
    Duration::from_millis(1)
}
