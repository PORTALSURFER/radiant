//! Reusable horizontal progress-bar primitive.

mod model;

use crate::gui::feedback::{horizontal_progress_activity_rect, horizontal_progress_fill_rect};
use crate::gui::types::{Point, Rect, Rgba8};
use crate::layout::{LayoutOutput, Vector2};
use crate::runtime::{PaintFillRect, PaintPrimitive};
use crate::theme::ThemeTokens;
use crate::widgets::contract::{FocusBehavior, PaintBounds, Widget, WidgetId, WidgetSizing};
use crate::widgets::interaction::{
    ActivationInputPolicy, WidgetInput, WidgetOutput, handle_activation_input,
};
use crate::widgets::primitives::support::WidgetCommon;

pub use model::{ProgressBarMessage, ProgressBarMode, ProgressBarProps, ProgressBarWidgetParts};

/// Generic horizontal progress-bar widget.
#[derive(Clone, Debug, PartialEq)]
pub struct ProgressBarWidget {
    /// Shared widget contract.
    pub common: WidgetCommon,
    /// Immutable progress-bar configuration.
    pub props: ProgressBarProps,
}

impl ProgressBarWidget {
    /// Build a progress bar from named construction fields.
    pub fn from_parts(parts: ProgressBarWidgetParts) -> Self {
        let props = parts.props.normalized();
        let mut common = WidgetCommon::new(parts.id, parts.sizing);
        common.focus = if props.interactive {
            FocusBehavior::Pointer
        } else {
            FocusBehavior::None
        };
        common.paint.bounds = PaintBounds::ClipToRect;
        common.paint.paints_focus = false;
        common.paint.paints_state_layers = false;
        Self { common, props }
    }

    /// Build a progress bar with a generated runtime id.
    pub fn new(mode: ProgressBarMode) -> Self {
        Self::from_parts(ProgressBarWidgetParts {
            id: 0,
            sizing: WidgetSizing::fixed(Vector2::new(1.0, 1.0)),
            props: ProgressBarProps::new(mode),
        })
    }

    /// Build a determinate progress bar.
    pub fn determinate(fraction: f32) -> Self {
        Self::new(ProgressBarMode::Determinate(fraction))
    }

    /// Build an indeterminate activity progress bar.
    pub fn indeterminate(position_fraction: f32) -> Self {
        Self::new(ProgressBarMode::Indeterminate(position_fraction))
    }

    /// Set explicit track and fill colors.
    pub fn with_colors(mut self, track: Rgba8, fill: Rgba8) -> Self {
        self.props.track_color = Some(track);
        self.props.fill_color = Some(fill);
        self
    }

    /// Set the maximum painted track height.
    pub fn with_max_track_height(mut self, height: f32) -> Self {
        self.props.max_track_height = height.max(0.0);
        self
    }

    /// Emit activation messages for primary pointer clicks.
    pub fn with_activation(mut self) -> Self {
        self.props.interactive = true;
        self.common.focus = FocusBehavior::Pointer;
        self
    }

    /// Route one backend-neutral interaction into the progress bar.
    pub fn handle_input(&mut self, bounds: Rect, input: WidgetInput) -> Option<ProgressBarMessage> {
        if !self.props.interactive {
            return None;
        }
        handle_activation_input(
            &mut self.common.state,
            bounds,
            &input,
            ActivationInputPolicy::pointer_only(),
        )
        .activated()
        .then_some(ProgressBarMessage::Activate)
    }
}

impl Widget for ProgressBarWidget {
    fn common(&self) -> &WidgetCommon {
        &self.common
    }

    fn common_mut(&mut self) -> &mut WidgetCommon {
        &mut self.common
    }

    fn handle_input(&mut self, bounds: Rect, input: WidgetInput) -> Option<WidgetOutput> {
        ProgressBarWidget::handle_input(self, bounds, input).map(WidgetOutput::typed)
    }

    fn synchronize_from_previous(&mut self, previous: &dyn Widget) {
        let Some(previous) = previous.as_any().downcast_ref::<Self>() else {
            return;
        };
        self.common.state = previous.common.state;
    }

    fn accepts_pointer_move(&self) -> bool {
        self.props.interactive
    }

    fn needs_state_synchronization(&self) -> bool {
        self.props.interactive
    }

    fn append_paint(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        bounds: Rect,
        _layout: &LayoutOutput,
        theme: &ThemeTokens,
    ) {
        let Some(track) = centered_horizontal_track(bounds, self.props.max_track_height) else {
            return;
        };
        push_fill(
            primitives,
            self.common.id,
            track,
            progress_track_color(self.props.track_color, theme),
        );
        let Some(fill) = progress_fill_rect(track, self.props) else {
            return;
        };
        push_fill(
            primitives,
            self.common.id,
            fill,
            progress_fill_color(self.props.fill_color, theme),
        );
    }
}

fn centered_horizontal_track(bounds: Rect, max_height: f32) -> Option<Rect> {
    if !bounds.has_finite_positive_area() || !max_height.is_finite() {
        return None;
    }
    let height = bounds.height().min(max_height).max(0.0);
    if height <= 0.0 {
        return None;
    }
    let y = bounds.min.y + (bounds.height() - height) * 0.5;
    Some(Rect::from_min_max(
        Point::new(bounds.min.x, y),
        Point::new(bounds.max.x, y + height),
    ))
}

fn progress_fill_rect(track: Rect, props: ProgressBarProps) -> Option<Rect> {
    match props.mode {
        ProgressBarMode::Determinate(fraction) => horizontal_progress_fill_rect(track, fraction),
        ProgressBarMode::Indeterminate(position) => horizontal_progress_activity_rect(
            track,
            position,
            props.activity_segment_fraction,
            props.min_activity_segment_width,
        ),
    }
}

fn progress_track_color(color: Option<Rgba8>, theme: &ThemeTokens) -> Rgba8 {
    color.unwrap_or(theme.bg_tertiary.with_alpha(210))
}

fn progress_fill_color(color: Option<Rgba8>, theme: &ThemeTokens) -> Rgba8 {
    color.unwrap_or(theme.accent_copper.with_alpha(210))
}

fn push_fill(primitives: &mut Vec<PaintPrimitive>, widget_id: WidgetId, rect: Rect, color: Rgba8) {
    primitives.push(PaintPrimitive::FillRect(PaintFillRect {
        widget_id,
        rect,
        color,
    }));
}
