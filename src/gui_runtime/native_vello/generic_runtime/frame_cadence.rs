//! Timed-frame scheduling policy for native animation and paint-only overlays.

use crate::runtime::RuntimeAnimationActivity;
use std::time::{Duration, Instant};

const TEXT_INPUT_CARET_TARGET_FPS: u32 = 30;

pub(super) fn animation_frame_interval(target_fps: u32) -> Duration {
    let fps = crate::gui_runtime::options::normalize_native_target_fps(target_fps);
    Duration::from_secs_f64(1.0 / f64::from(fps))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum TimedFrameCadence {
    Idle,
    WaitUntil(Instant),
    DrainNow { next_wake: Instant },
}

pub(super) fn timed_frame_cadence(
    now: Instant,
    last_redraw: Instant,
    target_fps: u32,
    active: bool,
) -> TimedFrameCadence {
    if !active {
        return TimedFrameCadence::Idle;
    }
    let interval = animation_frame_interval(target_fps);
    let next_frame = last_redraw.checked_add(interval).unwrap_or(now);
    if now >= next_frame {
        TimedFrameCadence::DrainNow {
            next_wake: now + interval,
        }
    } else {
        TimedFrameCadence::WaitUntil(next_frame)
    }
}

pub(super) fn timed_frame_target_fps(
    native_target_fps: u32,
    animation_activity: RuntimeAnimationActivity,
    needs_text_caret_animation: bool,
) -> u32 {
    let native_target_fps =
        crate::gui_runtime::options::normalize_native_target_fps(native_target_fps);
    if let Some(target_fps) = animation_activity.target_fps() {
        return crate::gui_runtime::options::normalize_native_target_fps(target_fps)
            .min(native_target_fps);
    }
    if animation_activity.needs_animation() {
        return native_target_fps;
    }
    if needs_text_caret_animation {
        return TEXT_INPUT_CARET_TARGET_FPS.min(native_target_fps);
    }
    native_target_fps
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn timed_frame_cadence_stays_idle_without_active_animation() {
        let now = Instant::now();

        assert_eq!(
            timed_frame_cadence(now, now - Duration::from_secs(1), 60, false),
            TimedFrameCadence::Idle
        );
    }

    #[test]
    fn timed_frame_cadence_waits_for_next_frame_deadline() {
        let last_redraw = Instant::now();
        let now = last_redraw + Duration::from_millis(5);
        let expected_next_frame = last_redraw + animation_frame_interval(60);

        assert_eq!(
            timed_frame_cadence(now, last_redraw, 60, true),
            TimedFrameCadence::WaitUntil(expected_next_frame)
        );
    }

    #[test]
    fn timed_frame_cadence_drains_and_schedules_next_wake_when_due() {
        let last_redraw = Instant::now();
        let interval = animation_frame_interval(120);
        let now = last_redraw + interval;

        assert_eq!(
            timed_frame_cadence(now, last_redraw, 120, true),
            TimedFrameCadence::DrainNow {
                next_wake: now + interval
            }
        );
    }

    #[test]
    fn timed_frame_target_fps_uses_activity_cap_without_exceeding_native_policy() {
        assert_eq!(
            timed_frame_target_fps(120, RuntimeAnimationActivity::paint_only_at(24), false),
            24
        );
        assert_eq!(
            timed_frame_target_fps(60, RuntimeAnimationActivity::paint_only_at(240), false),
            60
        );
    }

    #[test]
    fn timed_frame_target_fps_caps_standalone_text_caret_animation() {
        assert_eq!(
            timed_frame_target_fps(120, RuntimeAnimationActivity::idle(), true),
            30
        );
        assert_eq!(
            timed_frame_target_fps(24, RuntimeAnimationActivity::idle(), true),
            24
        );
    }

    #[test]
    fn timed_frame_target_fps_preserves_explicit_animation_caps_with_text_focus() {
        assert_eq!(
            timed_frame_target_fps(120, RuntimeAnimationActivity::paint_only_at(24), true),
            24
        );
        assert_eq!(
            timed_frame_target_fps(120, RuntimeAnimationActivity::frame_messages_at(48), true),
            48
        );
    }
}
