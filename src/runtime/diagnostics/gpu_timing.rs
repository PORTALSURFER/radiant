use std::time::Duration;

/// The terminal result of one aggregate GPU interval for a successfully
/// presented native frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FrameGpuTimingOutcome {
    /// The backend returned an aggregate GPU execution duration.
    Available {
        /// Aggregate GPU execution duration.
        duration: Duration,
    },
    /// The frame was presented, but the backend could not return a duration.
    Unavailable {
        /// Bounded reason that no GPU duration was available.
        reason: FrameGpuTimingUnavailableReason,
    },
}

impl FrameGpuTimingOutcome {
    /// Construct an available GPU timing outcome.
    pub const fn available(duration: Duration) -> Self {
        Self::Available { duration }
    }

    /// Construct an unavailable GPU timing outcome.
    pub const fn unavailable(reason: FrameGpuTimingUnavailableReason) -> Self {
        Self::Unavailable { reason }
    }

    /// Return the measured duration when one was available.
    pub const fn duration(self) -> Option<Duration> {
        match self {
            Self::Available { duration } => Some(duration),
            Self::Unavailable { .. } => None,
        }
    }
}

/// Why a successfully presented frame has no aggregate GPU duration.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FrameGpuTimingUnavailableReason {
    /// The present contained no attributable GPU command interval.
    NoWork,
    /// The selected adapter/device did not expose the required timestamp
    /// query capability.
    Unsupported,
    /// The backend's bounded pending capacity was full.
    CapacityRefused,
    /// The asynchronous readback did not map successfully.
    MappingFailed,
    /// Timestamp values could not be converted into a finite duration.
    ConversionFailed,
}

/// Correlated aggregate GPU timing for one successful native present.
///
/// The interval covers frame-owned GPU work from the first attributable GPU
/// command through final composition. CPU-side present and display/scanout
/// are excluded. Delivery is asynchronous and independent of
/// [`FrameProfile`](crate::runtime::diagnostics::FrameProfile)
/// delivery. The generic native primary and auxiliary runners can emit a
/// per-window sample when that window's frame profiling and the GPU-timing
/// observer are enabled. Auxiliary samples preserve their window identity
/// through the existing parent handoff, while lifecycle and generation fences
/// discard stale or otherwise invalid completions.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FrameGpuTimingSample {
    /// Stable native-window identity for the presenting runner.
    pub window_identity: u64,
    /// Monotonic successful-presentation sequence for that window.
    pub frame_sequence: u64,
    /// Terminal result of the aggregate GPU interval.
    pub outcome: FrameGpuTimingOutcome,
}

impl FrameGpuTimingSample {
    /// Construct one correlated GPU timing sample.
    pub const fn new(
        window_identity: u64,
        frame_sequence: u64,
        outcome: FrameGpuTimingOutcome,
    ) -> Self {
        Self {
            window_identity,
            frame_sequence,
            outcome,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{FrameGpuTimingOutcome, FrameGpuTimingSample, FrameGpuTimingUnavailableReason};
    use std::time::Duration;

    #[test]
    fn available_sample_preserves_correlation_and_duration() {
        let outcome = FrameGpuTimingOutcome::available(Duration::from_nanos(37));
        let sample = FrameGpuTimingSample::new(3, 19, outcome);

        assert_eq!(
            sample,
            FrameGpuTimingSample {
                window_identity: 3,
                frame_sequence: 19,
                outcome,
            }
        );
        assert_eq!(sample.outcome.duration(), Some(Duration::from_nanos(37)));
    }

    #[test]
    fn unavailable_outcomes_remain_explicit_terminal_reasons() {
        let reasons = [
            FrameGpuTimingUnavailableReason::NoWork,
            FrameGpuTimingUnavailableReason::Unsupported,
            FrameGpuTimingUnavailableReason::CapacityRefused,
            FrameGpuTimingUnavailableReason::MappingFailed,
            FrameGpuTimingUnavailableReason::ConversionFailed,
        ];

        for reason in reasons {
            let outcome = FrameGpuTimingOutcome::unavailable(reason);
            assert_eq!(outcome.duration(), None);
            assert_eq!(outcome, FrameGpuTimingOutcome::Unavailable { reason });
        }
    }
}
