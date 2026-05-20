use crate::gui::types::Point;
use std::sync::Arc;

#[cfg(test)]
#[path = "path/tests.rs"]
mod tests;

/// Shared immutable backend-neutral bezier path used by vector paint primitives.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PaintPath {
    commands: Arc<[PaintPathCommand]>,
}

impl PaintPath {
    /// Build a paint path from backend-neutral path commands.
    pub fn new(commands: impl Into<Arc<[PaintPathCommand]>>) -> Self {
        Self {
            commands: commands.into(),
        }
    }

    /// Build an empty paint path.
    pub fn empty() -> Self {
        Self::default()
    }

    /// Return this path's commands in paint order.
    pub fn commands(&self) -> &[PaintPathCommand] {
        &self.commands
    }

    /// Return whether this path has no commands.
    pub fn is_empty(&self) -> bool {
        self.commands.is_empty()
    }
}

impl From<Vec<PaintPathCommand>> for PaintPath {
    fn from(commands: Vec<PaintPathCommand>) -> Self {
        Self::new(commands)
    }
}

impl<const N: usize> From<[PaintPathCommand; N]> for PaintPath {
    fn from(commands: [PaintPathCommand; N]) -> Self {
        Self::new(Arc::from(commands))
    }
}

/// One backend-neutral vector path command.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PaintPathCommand {
    /// Move the current point without drawing.
    MoveTo(Point),
    /// Draw a straight line to the point.
    LineTo(Point),
    /// Draw a quadratic curve.
    QuadTo {
        /// Quadratic control point.
        control: Point,
        /// Curve endpoint.
        to: Point,
    },
    /// Draw a cubic curve.
    CurveTo {
        /// First cubic control point.
        control1: Point,
        /// Second cubic control point.
        control2: Point,
        /// Curve endpoint.
        to: Point,
    },
    /// Close the current subpath.
    Close,
}

/// Affine transform applied while replaying vector paint primitives.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PaintTransform {
    /// Horizontal scale/skew coefficient.
    pub xx: f64,
    /// Vertical skew coefficient.
    pub yx: f64,
    /// Horizontal skew coefficient.
    pub xy: f64,
    /// Vertical scale/skew coefficient.
    pub yy: f64,
    /// Horizontal translation.
    pub dx: f64,
    /// Vertical translation.
    pub dy: f64,
}

impl PaintTransform {
    /// Identity transform.
    pub const IDENTITY: Self = Self {
        xx: 1.0,
        yx: 0.0,
        xy: 0.0,
        yy: 1.0,
        dx: 0.0,
        dy: 0.0,
    };

    /// Build a transform from affine coefficients in `[xx, yx, xy, yy, dx, dy]` order.
    pub const fn new(coefficients: [f64; 6]) -> Self {
        let [xx, yx, xy, yy, dx, dy] = coefficients;
        Self {
            xx,
            yx,
            xy,
            yy,
            dx,
            dy,
        }
    }

    /// Build a translation transform.
    pub const fn translate(dx: f64, dy: f64) -> Self {
        Self {
            dx,
            dy,
            ..Self::IDENTITY
        }
    }

    /// Build a non-uniform scale transform.
    pub const fn scale_non_uniform(x: f64, y: f64) -> Self {
        Self {
            xx: x,
            yy: y,
            ..Self::IDENTITY
        }
    }

    /// Return affine coefficients in `[xx, yx, xy, yy, dx, dy]` order.
    pub const fn coefficients(self) -> [f64; 6] {
        [self.xx, self.yx, self.xy, self.yy, self.dx, self.dy]
    }

    /// Return whether every affine coefficient is finite.
    pub fn is_finite(self) -> bool {
        self.coefficients()
            .into_iter()
            .all(|coefficient| coefficient.is_finite())
    }
}

impl Default for PaintTransform {
    fn default() -> Self {
        Self::IDENTITY
    }
}

/// Rule used to determine the filled area of a vector path.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PaintFillRule {
    /// Fill all regions with non-zero winding.
    NonZero,
    /// Fill all regions with odd winding.
    EvenOdd,
}
