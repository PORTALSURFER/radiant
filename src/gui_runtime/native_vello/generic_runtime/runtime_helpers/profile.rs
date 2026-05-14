use super::super::GenericRouteOutcome;
use std::sync::OnceLock;
use std::time::Duration;
use tracing::info;

const RENDER_PROFILE_ENV: &str = "RADIANT_NATIVE_RENDER_PROFILE";
static RENDER_PROFILE_ENABLED: OnceLock<bool> = OnceLock::new();

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
    *RENDER_PROFILE_ENABLED.get_or_init(|| {
        std::env::var(RENDER_PROFILE_ENV)
            .ok()
            .is_some_and(|value| parse_render_profile_enabled(&value))
    })
}

fn parse_render_profile_enabled(value: &str) -> bool {
    crate::env_flags::is_truthy(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_profile_flag_uses_shared_truthy_parser() {
        assert!(parse_render_profile_enabled("1"));
        assert!(parse_render_profile_enabled("true"));
        assert!(!parse_render_profile_enabled(""));
        assert!(!parse_render_profile_enabled("false"));
    }
}
