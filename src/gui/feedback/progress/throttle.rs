#[cfg(test)]
#[path = "throttle/tests.rs"]
mod tests;

use std::time::{Duration, Instant};

/// Coalesces frequent fractional progress updates for responsive UI delivery.
#[derive(Clone, Debug)]
pub struct ProgressUpdateGate {
    min_interval: Duration,
    min_delta: f32,
    max_fraction: f32,
    last_sent_at: Option<Instant>,
    last_fraction: f32,
}

/// Runs accepted progress updates through a caller-provided reporting callback.
#[derive(Clone, Debug)]
pub struct ThrottledProgressReporter<Report> {
    gate: ProgressUpdateGate,
    report: Report,
}

impl ProgressUpdateGate {
    /// Create a gate that accepts the first update, then requires both elapsed
    /// time and fractional progress before accepting another non-terminal update.
    pub fn new(min_interval: Duration, min_delta: f32) -> Self {
        Self {
            min_interval,
            min_delta: min_delta.max(0.0),
            max_fraction: 1.0,
            last_sent_at: None,
            last_fraction: 0.0,
        }
    }

    /// Set the maximum reported fraction after input clamping.
    pub fn with_max_fraction(mut self, max_fraction: f32) -> Self {
        self.max_fraction = max_fraction.clamp(0.0, 1.0);
        self
    }

    /// Return the clamped fraction when this update should be emitted.
    pub fn accept(&mut self, fraction: f32) -> Option<f32> {
        self.accept_at(fraction, Instant::now())
    }

    /// Return the clamped fraction when this timestamped update should be emitted.
    pub fn accept_at(&mut self, fraction: f32, now: Instant) -> Option<f32> {
        let fraction = fraction.clamp(0.0, self.max_fraction);
        if !self.should_accept(fraction, now) {
            return None;
        }
        self.last_sent_at = Some(now);
        self.last_fraction = fraction;
        Some(fraction)
    }

    fn should_accept(&self, fraction: f32, now: Instant) -> bool {
        if fraction >= self.max_fraction {
            return true;
        }
        let Some(last_sent_at) = self.last_sent_at else {
            return true;
        };
        if fraction <= self.last_fraction {
            return false;
        }
        now.duration_since(last_sent_at) >= self.min_interval
            && fraction - self.last_fraction >= self.min_delta
    }
}

impl<Report> ThrottledProgressReporter<Report>
where
    Report: FnMut(f32),
{
    /// Create a reporter from an explicit progress gate and callback.
    pub fn new(gate: ProgressUpdateGate, report: Report) -> Self {
        Self { gate, report }
    }

    /// Report progress using the current time.
    pub fn report(&mut self, fraction: f32) {
        self.report_at(fraction, Instant::now());
    }

    /// Report progress using a supplied timestamp.
    ///
    /// This is primarily useful for deterministic tests and manually driven
    /// worker loops.
    pub fn report_at(&mut self, fraction: f32, now: Instant) {
        let Some(fraction) = self.gate.accept_at(fraction, now) else {
            return;
        };
        (self.report)(fraction);
    }

    /// Return the underlying gate and callback.
    pub fn into_parts(self) -> (ProgressUpdateGate, Report) {
        (self.gate, self.report)
    }
}
