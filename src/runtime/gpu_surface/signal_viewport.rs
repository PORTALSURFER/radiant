//! Exact-origin signal viewport coordinates.
//!
//! These types retain the source-frame origin as an integer. Floating-point
//! values are used only for the local fractional part and viewport span; this
//! module does not attempt to recover precision already lost by a legacy
//! floating-point range.

use std::fmt;

const U64_EXCLUSIVE_F64: f64 = 18_446_744_073_709_551_616.0;
const MAX_EXACT_F64_INTEGER: u64 = 1 << 53;

/// A source-frame position with an exact integral frame origin.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GpuSignalPosition {
    frame: u64,
    fraction: f64,
}

/// A source-frame viewport with an exact start origin and a local span.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GpuSignalViewport {
    start: GpuSignalPosition,
    span: f64,
}

/// Error returned when a signal viewport coordinate cannot be represented
/// without silently changing its source-frame meaning.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum GpuSignalViewportError {
    /// A position fraction was not finite or was outside `[0, 1)`.
    InvalidFraction {
        /// Rejected fractional-frame value.
        fraction: f64,
    },
    /// A viewport span was not finite and positive.
    InvalidSpan {
        /// Rejected local frame span.
        span: f64,
    },
    /// A normalized viewport ratio was not finite or outside `[0, 1]`.
    InvalidRatio {
        /// Rejected normalized viewport ratio.
        ratio: f64,
    },
    /// A legacy floating-point range was not finite, non-negative, and ascending.
    InvalidLegacyRange {
        /// Rejected legacy start/end pair.
        range: [f32; 2],
    },
    /// A coordinate operation would move outside the `u64` source-frame domain.
    FrameOutOfBounds,
    /// A local offset would require converting an inexact integer frame delta to `f64`.
    LocalOffsetOutOfExactRange {
        /// Integer distance between the two source origins.
        frame_delta: u64,
    },
}

impl fmt::Display for GpuSignalViewportError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidFraction { fraction } => {
                write!(
                    formatter,
                    "signal position fraction {fraction} must be finite and in [0, 1)"
                )
            }
            Self::InvalidSpan { span } => {
                write!(
                    formatter,
                    "signal viewport span {span} must be finite and positive"
                )
            }
            Self::InvalidRatio { ratio } => {
                write!(
                    formatter,
                    "signal viewport ratio {ratio} must be finite and in [0, 1]"
                )
            }
            Self::InvalidLegacyRange { range } => write!(
                formatter,
                "legacy signal range [{}, {}] must be finite, non-negative, and ascending",
                range[0], range[1]
            ),
            Self::FrameOutOfBounds => {
                formatter.write_str("signal frame position is outside the u64 domain")
            }
            Self::LocalOffsetOutOfExactRange { frame_delta } => write!(
                formatter,
                "signal local offset frame delta {frame_delta} exceeds the exact f64 integer range"
            ),
        }
    }
}

impl std::error::Error for GpuSignalViewportError {}

impl GpuSignalPosition {
    /// Construct a position from an exact source frame and a local fraction.
    pub fn new(frame: u64, fraction: f64) -> Result<Self, GpuSignalViewportError> {
        if !fraction.is_finite() || !(0.0..1.0).contains(&fraction) {
            return Err(GpuSignalViewportError::InvalidFraction { fraction });
        }
        Ok(Self { frame, fraction })
    }

    /// Return the exact integral source frame.
    pub const fn frame(self) -> u64 {
        self.frame
    }

    /// Return the local fraction of one source frame.
    pub const fn fraction(self) -> f64 {
        self.fraction
    }

    /// Translate this position by an exact signed number of source frames.
    pub fn translated_frames(self, delta: i64) -> Result<Self, GpuSignalViewportError> {
        let frame = if delta >= 0 {
            self.frame.checked_add(delta as u64)
        } else {
            self.frame.checked_sub(delta.unsigned_abs())
        }
        .ok_or(GpuSignalViewportError::FrameOutOfBounds)?;
        Ok(Self {
            frame,
            fraction: self.fraction,
        })
    }
}

impl GpuSignalViewport {
    /// Construct a viewport from an exact start position and a finite positive span.
    pub fn new(start: GpuSignalPosition, span: f64) -> Result<Self, GpuSignalViewportError> {
        if !span.is_finite() || span <= 0.0 {
            return Err(GpuSignalViewportError::InvalidSpan { span });
        }
        let viewport = Self { start, span };
        viewport.end()?;
        Ok(viewport)
    }

    /// Adapt a legacy `[start, end]` `f32` frame range.
    ///
    /// The resulting viewport reflects exactly the already-rounded legacy
    /// values. It cannot restore an absolute origin that was lost before this
    /// method was called.
    pub fn from_f32_range(range: [f32; 2]) -> Result<Self, GpuSignalViewportError> {
        let start = f64::from(range[0]);
        let end = f64::from(range[1]);
        if !start.is_finite() || !end.is_finite() || start < 0.0 || end <= start {
            return Err(GpuSignalViewportError::InvalidLegacyRange { range });
        }
        let start = position_from_nonnegative_f64(start)?;
        Self::new(start, end - f64::from(range[0]))
    }

    /// Return the exact viewport start position.
    pub const fn start(self) -> GpuSignalPosition {
        self.start
    }

    /// Return the finite positive viewport span in source frames.
    pub const fn span(self) -> f64 {
        self.span
    }

    /// Return the exact-origin viewport end position.
    pub fn end(self) -> Result<GpuSignalPosition, GpuSignalViewportError> {
        add_local_frames(self.start, self.span)
    }

    /// Return the source position at a normalized viewport ratio in `[0, 1]`.
    pub fn position_at(self, ratio: f64) -> Result<GpuSignalPosition, GpuSignalViewportError> {
        if !ratio.is_finite() || !(0.0..=1.0).contains(&ratio) {
            return Err(GpuSignalViewportError::InvalidRatio { ratio });
        }
        add_local_frames(self.start, self.span * ratio)
    }

    /// Return this viewport translated by an exact signed number of source frames.
    pub fn translated_frames(self, delta: i64) -> Result<Self, GpuSignalViewportError> {
        Self::new(self.start.translated_frames(delta)?, self.span)
    }

    /// Change span while retaining the source position at `ratio`.
    ///
    /// The anchor is retained through local floating-point arithmetic; callers
    /// should not treat it as a repair for a legacy rounded absolute range.
    pub fn zoom_anchored(self, ratio: f64, new_span: f64) -> Result<Self, GpuSignalViewportError> {
        if !ratio.is_finite() || !(0.0..=1.0).contains(&ratio) {
            return Err(GpuSignalViewportError::InvalidRatio { ratio });
        }
        if !new_span.is_finite() || new_span <= 0.0 {
            return Err(GpuSignalViewportError::InvalidSpan { span: new_span });
        }
        let anchor = self.position_at(ratio)?;
        let start = add_local_frames(anchor, -(new_span * ratio))?;
        Self::new(start, new_span)
    }

    /// Return `position` relative to this viewport start as a local `f64` offset.
    ///
    /// Integer origins are subtracted before conversion. Deltas beyond `2^53`
    /// are rejected because `f64` could not represent every intervening frame.
    /// The returned local `f64` can still round its fractional component at a
    /// large, but exactly representable, integer delta.
    pub fn local_offset(self, position: GpuSignalPosition) -> Result<f64, GpuSignalViewportError> {
        let frame_delta = position.frame.abs_diff(self.start.frame);
        if frame_delta > MAX_EXACT_F64_INTEGER {
            return Err(GpuSignalViewportError::LocalOffsetOutOfExactRange { frame_delta });
        }
        let frame_offset = if position.frame >= self.start.frame {
            frame_delta as f64
        } else {
            -(frame_delta as f64)
        };
        let fractional_offset = position.fraction - self.start.fraction;
        Ok(frame_offset + fractional_offset)
    }
}

fn position_from_nonnegative_f64(value: f64) -> Result<GpuSignalPosition, GpuSignalViewportError> {
    if !value.is_finite() || !(0.0..U64_EXCLUSIVE_F64).contains(&value) {
        return Err(GpuSignalViewportError::FrameOutOfBounds);
    }
    GpuSignalPosition::new(value.trunc() as u64, value.fract())
}

fn add_local_frames(
    position: GpuSignalPosition,
    delta: f64,
) -> Result<GpuSignalPosition, GpuSignalViewportError> {
    if !delta.is_finite() {
        return Err(GpuSignalViewportError::FrameOutOfBounds);
    }
    let whole = delta.trunc();
    if whole >= U64_EXCLUSIVE_F64 || whole <= -U64_EXCLUSIVE_F64 {
        return Err(GpuSignalViewportError::FrameOutOfBounds);
    }
    let mut frame = if whole >= 0.0 {
        position.frame.checked_add(whole as u64)
    } else {
        position.frame.checked_sub((-whole) as u64)
    }
    .ok_or(GpuSignalViewportError::FrameOutOfBounds)?;

    let local_fraction = position.fraction + (delta - whole);
    let (fraction, carry) = if local_fraction >= 1.0 {
        (local_fraction - 1.0, 1)
    } else if local_fraction < 0.0 {
        (local_fraction + 1.0, -1)
    } else {
        (local_fraction, 0)
    };
    if !(0.0..1.0).contains(&fraction) {
        // A negative subnormal residual at a zero fraction can round to 1.0
        // after borrowing. Reject it rather than silently turning it into a
        // whole-frame move.
        return Err(GpuSignalViewportError::FrameOutOfBounds);
    }
    frame = if carry > 0 {
        frame.checked_add(1)
    } else if carry < 0 {
        frame.checked_sub(1)
    } else {
        Some(frame)
    }
    .ok_or(GpuSignalViewportError::FrameOutOfBounds)?;
    GpuSignalPosition::new(frame, fraction)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_geometry_is_stable_at_large_origins() {
        let near =
            GpuSignalViewport::new(GpuSignalPosition::new(1 << 24, 0.125).unwrap(), 64.0).unwrap();
        let far =
            GpuSignalViewport::new(GpuSignalPosition::new(1 << 40, 0.125).unwrap(), 64.0).unwrap();

        for ratio in [0.0, 0.125, 0.5, 1.0] {
            let near_offset = near.local_offset(near.position_at(ratio).unwrap()).unwrap();
            let far_offset = far.local_offset(far.position_at(ratio).unwrap()).unwrap();
            assert_eq!(near_offset, far_offset);
        }
    }

    #[test]
    fn pan_and_zoom_keep_fractional_anchor_local() {
        let viewport =
            GpuSignalViewport::new(GpuSignalPosition::new(1 << 40, 0.25).unwrap(), 80.0).unwrap();
        let panned = viewport.translated_frames(-17).unwrap();
        assert_eq!(panned.start().fraction(), 0.25);

        let ratio = 0.375;
        let anchor = viewport.position_at(ratio).unwrap();
        let zoomed = viewport.zoom_anchored(ratio, 20.0).unwrap();
        let zoomed_anchor = zoomed.position_at(ratio).unwrap();
        assert_eq!(
            zoomed.local_offset(anchor).unwrap(),
            zoomed.local_offset(zoomed_anchor).unwrap()
        );
        assert_eq!(
            viewport
                .local_offset(GpuSignalPosition::new((1 << 40) - 1, 0.75).unwrap())
                .unwrap(),
            -0.5
        );
    }

    #[test]
    fn integer_translation_checks_both_source_edges() {
        assert_eq!(
            GpuSignalPosition::new(4, 0.5)
                .unwrap()
                .translated_frames(-4)
                .unwrap()
                .frame(),
            0
        );
        assert_eq!(
            GpuSignalPosition::new(u64::MAX - 3, 0.5)
                .unwrap()
                .translated_frames(3)
                .unwrap()
                .frame(),
            u64::MAX
        );
        assert_eq!(
            GpuSignalPosition::new(0, 0.0)
                .unwrap()
                .translated_frames(-1),
            Err(GpuSignalViewportError::FrameOutOfBounds)
        );
        assert_eq!(
            GpuSignalPosition::new(u64::MAX, 0.0)
                .unwrap()
                .translated_frames(1),
            Err(GpuSignalViewportError::FrameOutOfBounds)
        );
    }

    #[test]
    fn large_integral_deltas_preserve_the_local_fraction() {
        let positive = add_local_frames(
            GpuSignalPosition::new(7, 0.25).unwrap(),
            (1_u64 << 53) as f64,
        )
        .unwrap();
        assert_eq!(
            positive,
            GpuSignalPosition::new((1 << 53) + 7, 0.25).unwrap()
        );

        let negative = add_local_frames(
            GpuSignalPosition::new((1 << 53) + 7, 0.75).unwrap(),
            -((1_u64 << 53) as f64),
        )
        .unwrap();
        assert_eq!(negative, GpuSignalPosition::new(7, 0.75).unwrap());
    }

    #[test]
    fn fractional_carries_are_checked_at_source_edges() {
        assert_eq!(
            add_local_frames(GpuSignalPosition::new(u64::MAX - 1, 0.75).unwrap(), 0.5),
            Ok(GpuSignalPosition::new(u64::MAX, 0.25).unwrap())
        );
        assert_eq!(
            add_local_frames(GpuSignalPosition::new(1, 0.25).unwrap(), -0.5),
            Ok(GpuSignalPosition::new(0, 0.75).unwrap())
        );
        assert_eq!(
            add_local_frames(GpuSignalPosition::new(u64::MAX, 0.75).unwrap(), 0.5),
            Err(GpuSignalViewportError::FrameOutOfBounds)
        );
        assert_eq!(
            add_local_frames(GpuSignalPosition::new(1, 0.0).unwrap(), -f64::MIN_POSITIVE),
            Err(GpuSignalViewportError::FrameOutOfBounds)
        );
    }

    #[test]
    fn end_and_local_offset_reject_unrepresentable_ranges() {
        assert_eq!(
            GpuSignalViewport::new(GpuSignalPosition::new(u64::MAX, 0.75).unwrap(), 0.5),
            Err(GpuSignalViewportError::FrameOutOfBounds)
        );
        let viewport =
            GpuSignalViewport::new(GpuSignalPosition::new(0, 0.0).unwrap(), 1.0).unwrap();
        assert_eq!(
            viewport.local_offset(GpuSignalPosition::new((1 << 53) + 1, 0.0).unwrap()),
            Err(GpuSignalViewportError::LocalOffsetOutOfExactRange {
                frame_delta: (1 << 53) + 1
            })
        );
    }

    #[test]
    fn malformed_ranges_are_rejected() {
        assert!(matches!(
            GpuSignalPosition::new(0, f64::NAN),
            Err(GpuSignalViewportError::InvalidFraction { fraction }) if fraction.is_nan()
        ));
        assert_eq!(
            GpuSignalViewport::new(GpuSignalPosition::new(0, 0.0).unwrap(), 0.0),
            Err(GpuSignalViewportError::InvalidSpan { span: 0.0 })
        );
        assert_eq!(
            GpuSignalViewport::from_f32_range([2.0, 2.0]),
            Err(GpuSignalViewportError::InvalidLegacyRange { range: [2.0, 2.0] })
        );
    }

    #[test]
    fn ratio_edges_match_start_and_end() {
        let viewport =
            GpuSignalViewport::new(GpuSignalPosition::new(12, 0.25).unwrap(), 5.5).unwrap();
        assert_eq!(viewport.position_at(0.0).unwrap(), viewport.start());
        assert_eq!(viewport.position_at(1.0).unwrap(), viewport.end().unwrap());
        assert_eq!(
            viewport.position_at(1.1),
            Err(GpuSignalViewportError::InvalidRatio { ratio: 1.1 })
        );
    }
}
