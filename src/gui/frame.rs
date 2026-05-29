//! Frame feedback primitives shared by runtime bridges and render backends.

use std::time::{Duration, Instant};

#[cfg(test)]
#[path = "frame/tests.rs"]
mod tests;

/// Primitive counts produced while building a frame.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FrameBuildCounts {
    /// Number of generated shape primitives.
    pub primitive_count: usize,
    /// Number of generated text runs.
    pub text_run_count: usize,
}

/// Rebuild work performed while building a frame.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FrameRebuildFlags {
    /// Whether this redraw included a layout-driven static rebuild.
    pub layout_rebuild: bool,
    /// Whether this redraw rebuilt any static scene content.
    pub static_rebuild: bool,
    /// Whether this redraw rebuilt any state-overlay scene content.
    pub state_overlay_rebuild: bool,
    /// Whether this redraw rebuilt any motion-overlay scene content.
    pub motion_overlay_rebuild: bool,
}

/// Animation follow-up requested by frame construction.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FrameAnimationRequest {
    /// Whether runtime should keep animating while idle.
    pub needs_animation: bool,
}

/// Timing measurements captured while building and presenting a frame.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FrameBuildTiming {
    /// End-to-end frame time in microseconds for the redraw pass.
    pub frame_total_us: u32,
    /// Presentation duration in microseconds for the redraw pass.
    pub present_us: u32,
    /// Frame-time budget used to classify redraw jank.
    pub frame_budget_us: u32,
    /// Whether the frame exceeded the configured frame-time budget.
    pub jank: bool,
}

/// Presentation outcome for a frame redraw.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FramePresentResult {
    /// Whether the redraw produced a successful surface present.
    pub presented: bool,
    /// Whether a present was expected but not completed for this redraw.
    pub missed_present: bool,
}

/// Frame-level feedback from renderer to host bridge.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FrameBuildResult {
    /// Primitive counts produced by the frame.
    pub counts: FrameBuildCounts,
    /// Rebuild work performed by the frame.
    pub rebuilds: FrameRebuildFlags,
    /// Animation continuation requested by the frame.
    pub animation: FrameAnimationRequest,
    /// Frame timing measurements.
    pub timing: FrameBuildTiming,
    /// Surface presentation outcome.
    pub presentation: FramePresentResult,
}

/// Thresholds and sampling cadence for application-facing frame timing logs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FrameCadenceConfig {
    /// Delta at or above which a frame is classified as a warning spike.
    pub warn_threshold: Duration,
    /// Delta at or above which a frame is classified as an error spike.
    pub error_threshold: Duration,
    /// Emit a periodic sample every N frames when no spike is observed.
    pub periodic_report_every: u64,
}

impl FrameCadenceConfig {
    /// Build a frame-cadence configuration.
    pub const fn new(
        warn_threshold: Duration,
        error_threshold: Duration,
        periodic_report_every: u64,
    ) -> Self {
        Self {
            warn_threshold,
            error_threshold,
            periodic_report_every,
        }
    }
}

/// Classification for one observed UI frame delta.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FrameCadenceKind {
    /// First observed frame; there is no previous timestamp for a delta.
    Started,
    /// Frame delta crossed the configured error threshold.
    ErrorSpike,
    /// Frame delta crossed the configured warning threshold.
    WarnSpike,
    /// Periodic non-spike report requested by the configured cadence.
    Periodic,
    /// Normal frame with no report requested.
    Normal,
}

impl FrameCadenceKind {
    /// Returns true when this classification should normally be logged.
    pub const fn should_report(self) -> bool {
        !matches!(self, Self::Normal)
    }

    /// Stable severity string for logs and diagnostics.
    pub const fn severity(self) -> Option<&'static str> {
        match self {
            Self::ErrorSpike => Some("error"),
            Self::WarnSpike => Some("warn"),
            Self::Started | Self::Periodic | Self::Normal => None,
        }
    }
}

/// Result of one frame-cadence observation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FrameCadenceReport {
    /// Monotonic frame index, starting at one.
    pub frame_index: u64,
    /// Delta from the previous frame, if one exists.
    pub delta: Option<Duration>,
    /// Largest observed frame delta so far.
    pub max_delta: Duration,
    /// Classification for this observation.
    pub kind: FrameCadenceKind,
}

impl FrameCadenceReport {
    /// Returns true when this observation should normally be logged.
    pub const fn should_report(self) -> bool {
        self.kind.should_report()
    }
}

/// Small reusable monitor for application-facing UI frame cadence diagnostics.
#[derive(Clone, Debug)]
pub struct FrameCadenceMonitor {
    frame_index: u64,
    last_frame_at: Option<Instant>,
    max_delta: Duration,
}

impl Default for FrameCadenceMonitor {
    fn default() -> Self {
        Self {
            frame_index: 0,
            last_frame_at: None,
            max_delta: Duration::ZERO,
        }
    }
}

impl FrameCadenceMonitor {
    /// Create a new monitor with no observed frames.
    pub fn new() -> Self {
        Self::default()
    }

    /// Record a frame using the current instant.
    pub fn record_now(&mut self, config: FrameCadenceConfig) -> FrameCadenceReport {
        self.record_at(Instant::now(), config)
    }

    /// Record a frame at a caller-provided timestamp.
    pub fn record_at(&mut self, now: Instant, config: FrameCadenceConfig) -> FrameCadenceReport {
        self.frame_index = self.frame_index.saturating_add(1);
        let delta = self
            .last_frame_at
            .replace(now)
            .map(|last| now.saturating_duration_since(last));
        self.report(delta, config)
    }

    /// Record a known frame delta directly.
    ///
    /// This is useful for deterministic tests and host integrations that already
    /// measure frame deltas outside Radiant.
    pub fn record_delta(
        &mut self,
        delta: Option<Duration>,
        config: FrameCadenceConfig,
    ) -> FrameCadenceReport {
        self.frame_index = self.frame_index.saturating_add(1);
        self.report(delta, config)
    }

    fn report(
        &mut self,
        delta: Option<Duration>,
        config: FrameCadenceConfig,
    ) -> FrameCadenceReport {
        if let Some(delta) = delta {
            self.max_delta = self.max_delta.max(delta);
        }
        FrameCadenceReport {
            frame_index: self.frame_index,
            delta,
            max_delta: self.max_delta,
            kind: classify_frame_cadence(self.frame_index, delta, config),
        }
    }
}

fn classify_frame_cadence(
    frame_index: u64,
    delta: Option<Duration>,
    config: FrameCadenceConfig,
) -> FrameCadenceKind {
    let Some(delta) = delta else {
        return FrameCadenceKind::Started;
    };
    if delta >= config.error_threshold {
        FrameCadenceKind::ErrorSpike
    } else if delta >= config.warn_threshold {
        FrameCadenceKind::WarnSpike
    } else if config.periodic_report_every > 0
        && frame_index.is_multiple_of(config.periodic_report_every)
    {
        FrameCadenceKind::Periodic
    } else {
        FrameCadenceKind::Normal
    }
}
