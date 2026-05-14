use super::super::GenericRouteOutcome;
use std::time::Duration;
use tracing::info;

pub(in crate::gui_runtime::native_vello) fn maybe_log_route_profile(
    reason: &'static str,
    elapsed: Duration,
    outcome: GenericRouteOutcome,
) {
    if !render_profile_enabled() {
        return;
    }
    info!(
        reason,
        event_route_us = elapsed.as_micros(),
        routed = outcome.routed,
        redraw_requested = outcome.redraw_requested,
        repaint_requested = outcome.repaint_requested,
        "radiant native input profile"
    );
}

pub(in crate::gui_runtime::native_vello) fn render_profile_enabled() -> bool {
    std::env::var("RADIANT_NATIVE_RENDER_PROFILE")
        .ok()
        .is_some_and(|value| crate::env_flags::is_truthy(&value))
}
