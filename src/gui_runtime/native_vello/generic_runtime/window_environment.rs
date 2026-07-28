#[cfg(target_os = "macos")]
use crate::runtime::WindowEnvironmentChange;
use winit::{
    dpi::{PhysicalPosition, PhysicalSize},
    window::Window,
};

/// The small set of monitor facts needed to detect a real monitor transition.
///
/// Winit can temporarily report no current monitor while a window is being
/// moved. The runner therefore retains the last `Some` fingerprint instead of
/// treating that transient state as a monitor change.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct MonitorFingerprint {
    pub(super) name: Option<String>,
    pub(super) position: PhysicalPosition<i32>,
    pub(super) size: PhysicalSize<u32>,
    pub(super) scale_factor_bits: u64,
}

impl MonitorFingerprint {
    pub(super) fn from_facts(
        name: Option<String>,
        position: PhysicalPosition<i32>,
        size: PhysicalSize<u32>,
        scale_factor: f64,
    ) -> Self {
        Self {
            name,
            position,
            size,
            scale_factor_bits: scale_factor.to_bits(),
        }
    }
}

pub(super) fn current_monitor_fingerprint(window: &Window) -> Option<MonitorFingerprint> {
    let monitor = window.current_monitor()?;
    Some(MonitorFingerprint::from_facts(
        monitor.name(),
        monitor.position(),
        monitor.size(),
        monitor.scale_factor(),
    ))
}

/// The process-global accessibility display facts that affect a window.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) struct AccessibilityDisplaySnapshot {
    pub(super) increase_contrast: bool,
    pub(super) reduce_motion: bool,
}

/// Return the semantic causes represented by an accessibility snapshot delta.
#[cfg(target_os = "macos")]
pub(super) fn accessibility_display_changes(
    previous: AccessibilityDisplaySnapshot,
    next: AccessibilityDisplaySnapshot,
) -> impl Iterator<Item = WindowEnvironmentChange> {
    [
        (previous.increase_contrast != next.increase_contrast)
            .then_some(WindowEnvironmentChange::ColorSchemeOrContrast),
        (previous.reduce_motion != next.reduce_motion)
            .then_some(WindowEnvironmentChange::MotionPreference),
    ]
    .into_iter()
    .flatten()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn monitor_fingerprint_includes_stable_monitor_facts() {
        let base = MonitorFingerprint::from_facts(
            Some("Built-in".into()),
            PhysicalPosition::new(0, 0),
            PhysicalSize::new(1920, 1080),
            2.0,
        );
        assert_ne!(
            base,
            MonitorFingerprint::from_facts(
                Some("External".into()),
                PhysicalPosition::new(0, 0),
                PhysicalSize::new(1920, 1080),
                2.0,
            )
        );
        assert_ne!(
            base,
            MonitorFingerprint::from_facts(
                Some("Built-in".into()),
                PhysicalPosition::new(1920, 0),
                PhysicalSize::new(1920, 1080),
                2.0,
            )
        );
        assert_ne!(
            base,
            MonitorFingerprint::from_facts(
                Some("Built-in".into()),
                PhysicalPosition::new(0, 0),
                PhysicalSize::new(1920, 1080),
                1.0,
            )
        );
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn accessibility_snapshot_delta_only_reports_changed_causes() {
        let previous = AccessibilityDisplaySnapshot {
            increase_contrast: false,
            reduce_motion: true,
        };
        let next = AccessibilityDisplaySnapshot {
            increase_contrast: true,
            reduce_motion: true,
        };
        assert_eq!(
            accessibility_display_changes(previous, next).collect::<Vec<_>>(),
            vec![WindowEnvironmentChange::ColorSchemeOrContrast]
        );
    }
}
