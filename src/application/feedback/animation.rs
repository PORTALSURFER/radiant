use std::time::Duration;

use crate::{
    animation::{
        Animatable, AnimationPaintContext, AnimationTarget, AnimationValues, FeedbackAnimation,
    },
    gui::types::{Point, Rect},
    runtime::{PaintPrimitive, push_visible_fill_rect},
};

const FEEDBACK_GROUP: u64 = 0xfeed_ba5e;
const SPINNER_PROPERTY: u64 = 1;
const SKELETON_PROPERTY: u64 = 2;
const FEEDBACK_PERIOD: Duration = Duration::from_millis(900);
const STATIC_PHASE: f64 = 0.5;

/// Paint-only feedback treatment selected by the public presentation builder.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum FeedbackChromeKind {
    Spinner,
    Skeleton,
}

/// Immutable paint capability for runtime-sampled feedback motion.
pub(super) struct FeedbackChrome {
    kind: FeedbackChromeKind,
    feedback: Option<FeedbackAnimation>,
}

impl FeedbackChrome {
    pub(super) fn new(kind: FeedbackChromeKind) -> Self {
        let property = match kind {
            FeedbackChromeKind::Spinner => SPINNER_PROPERTY,
            FeedbackChromeKind::Skeleton => SKELETON_PROPERTY,
        };
        Self {
            kind,
            feedback: FeedbackAnimation::new(
                property,
                FEEDBACK_GROUP,
                FEEDBACK_PERIOD,
                STATIC_PHASE,
            )
            .ok(),
        }
    }
}

impl Animatable for FeedbackChrome {
    fn targets(&self) -> &[AnimationTarget] {
        &[]
    }

    fn feedback(&self) -> &[FeedbackAnimation] {
        self.feedback.as_slice()
    }

    fn append_paint(
        &self,
        values: AnimationValues<'_>,
        context: AnimationPaintContext<'_>,
        output: &mut Vec<PaintPrimitive>,
    ) {
        let phase = if context.environment.reduced_motion() {
            STATIC_PHASE
        } else {
            self.feedback
                .and_then(|declaration| values.get(declaration.property()))
                .filter(|value| value.is_finite())
                .unwrap_or(STATIC_PHASE)
        };
        match self.kind {
            FeedbackChromeKind::Spinner => {
                paint_spinner(output, context.bounds, phase, context.theme.accent_mint)
            }
            FeedbackChromeKind::Skeleton => {
                paint_skeleton(output, context.bounds, phase, context.theme.surface_raised)
            }
        }
    }
}

fn paint_spinner(
    output: &mut Vec<PaintPrimitive>,
    bounds: Rect,
    phase: f64,
    color: crate::gui::types::Rgba8,
) {
    if !bounds.has_finite_positive_area() {
        return;
    }
    let phase = phase.rem_euclid(1.0) as f32;
    let thickness = (bounds.height().min(bounds.width()) * 0.25).clamp(2.0, 4.0);
    let travel = (bounds.height() - thickness).max(0.0);
    let rect = Rect::from_min_size(
        Point::new(bounds.min.x, bounds.min.y + travel * phase),
        crate::layout::Vector2::new(bounds.width(), thickness),
    );
    push_visible_fill_rect(output, 0, rect, color);
}

fn paint_skeleton(
    output: &mut Vec<PaintPrimitive>,
    bounds: Rect,
    phase: f64,
    color: crate::gui::types::Rgba8,
) {
    if !bounds.has_finite_positive_area() {
        return;
    }
    push_visible_fill_rect(output, 0, bounds, color);
    let phase = phase.rem_euclid(1.0) as f32;
    let highlight_width = (bounds.width() * 0.28).max(1.0).min(bounds.width());
    let travel = (bounds.width() - highlight_width).max(0.0);
    let highlight = Rect::from_min_size(
        Point::new(bounds.min.x + travel * phase, bounds.min.y),
        crate::layout::Vector2::new(highlight_width, bounds.height()),
    );
    push_visible_fill_rect(
        output,
        0,
        highlight,
        crate::gui::types::Rgba8::new(255, 255, 255, 28),
    );
}
