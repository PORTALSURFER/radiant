//! Reusable badge and pill primitive.

use crate::gui::types::{Point, Rect, Vector2};
use crate::layout::LayoutOutput;
use crate::runtime::{PaintPrimitive, PaintText, ResolvedEnvironment, button_font_size};
use crate::theme::ThemeTokens;

use super::support::WidgetCommon;
use crate::widgets::contract::{
    FocusBehavior, Widget, WidgetId, WidgetPaintContext, WidgetPointerMotion,
    WidgetPointerMotionRevision, WidgetProminence, WidgetSizing, WidgetStyle, WidgetTone,
};
use crate::widgets::interaction::{BadgeMessage, WidgetInput, WidgetOutput};

mod builders;
mod input;
mod model;
mod paint;

pub use model::{BadgeChrome, BadgeProps, BadgeState};

/// Public badge/pill primitive.
#[derive(Clone, Debug, PartialEq)]
pub struct BadgeWidget {
    /// Shared widget contract.
    pub common: WidgetCommon,
    /// Immutable user-facing badge configuration.
    pub props: BadgeProps,
    /// Mutable interaction state owned by the badge.
    pub state: BadgeState,
}

/// Named construction fields for [`BadgeWidget`].
#[derive(Clone, Debug, PartialEq)]
pub struct BadgeWidgetParts {
    /// Stable widget identity used by layout, events, and state synchronization.
    pub id: WidgetId,
    /// User-facing badge text.
    pub label: PaintText,
    /// Intrinsic badge sizing contract.
    pub sizing: WidgetSizing,
}

impl BadgeWidget {
    pub(super) fn declared_text_metrics(&self) -> crate::widgets::DeclaredTextMetrics {
        crate::widgets::DeclaredTextMetrics::new(
            self.common.sizing,
            button_font_size(Rect::from_min_size(
                Point::default(),
                Vector2::new(0.0, self.common.sizing.preferred.y),
            )),
            Vector2::new(8.0, 3.0),
        )
    }

    /// Build a badge descriptor from named identity, content, and sizing fields.
    pub fn from_parts(parts: BadgeWidgetParts) -> Self {
        let mut common = WidgetCommon::new(parts.id, parts.sizing);
        common.focus = FocusBehavior::Keyboard;
        common.style = WidgetStyle {
            tone: WidgetTone::Neutral,
            prominence: WidgetProminence::Subtle,
        };
        Self {
            common,
            props: BadgeProps {
                label: parts.label,
                chrome: BadgeChrome::Filled,
            },
            state: BadgeState::default(),
        }
    }

    /// Build a badge descriptor with optional activation semantics.
    pub fn new(id: WidgetId, label: impl Into<PaintText>, sizing: WidgetSizing) -> Self {
        Self::from_parts(BadgeWidgetParts {
            id,
            label: label.into(),
            sizing,
        })
    }

    /// Set the active visual state for this badge.
    pub fn with_active(mut self, active: bool) -> Self {
        self.common.state.active = active;
        self
    }

    /// Use an outlined rather than filled badge surface.
    pub fn with_outline(mut self, outline: bool) -> Self {
        self.props.chrome = if outline {
            BadgeChrome::Outline
        } else {
            BadgeChrome::Filled
        };
        self
    }

    /// Route one backend-neutral interaction into the badge.
    pub fn handle_input(&mut self, bounds: Rect, input: WidgetInput) -> Option<BadgeMessage> {
        input::handle_badge_input(self, bounds, input)
    }
}

impl WidgetPointerMotion for BadgeWidget {
    fn revision(&self) -> WidgetPointerMotionRevision {
        WidgetPointerMotionRevision::exact(false)
    }

    fn accepts_pointer_move(&self) -> bool {
        false
    }
}

impl Widget for BadgeWidget {
    fn focused_key_disposition(
        &self,
        key: crate::widgets::WidgetKey,
    ) -> crate::widgets::FocusedKeyDisposition {
        if matches!(
            key,
            crate::widgets::WidgetKey::Enter | crate::widgets::WidgetKey::Space
        ) {
            crate::widgets::FocusedKeyDisposition::Consumed
        } else {
            crate::widgets::FocusedKeyDisposition::Unhandled
        }
    }

    fn text_scale_participation(&self) -> crate::widgets::TextScaleParticipation {
        crate::widgets::TextScaleParticipation::Scaled
    }

    fn layout_node_with_environment(
        &self,
        environment: &ResolvedEnvironment,
    ) -> crate::layout::LayoutNode {
        crate::layout::LayoutNode::Widget(
            self.declared_text_metrics()
                .resolve(environment, self.text_scale_participation())
                .layout_node(self.common.id),
        )
    }

    fn common(&self) -> &WidgetCommon {
        &self.common
    }

    fn common_mut(&mut self) -> &mut WidgetCommon {
        &mut self.common
    }

    fn handle_input(&mut self, bounds: Rect, input: WidgetInput) -> Option<WidgetOutput> {
        BadgeWidget::handle_input(self, bounds, input).map(WidgetOutput::typed)
    }

    fn handle_pointer_capture_cancelled(&mut self, _bounds: Rect) -> Option<WidgetOutput> {
        input::handle_pointer_capture_cancelled(self);
        None
    }

    fn capabilities(&self) -> crate::widgets::WidgetCapabilities<'_> {
        crate::widgets::WidgetCapabilities::none()
    }

    fn capabilities_v2(&self) -> crate::widgets::WidgetCapabilitiesV2<'_> {
        crate::widgets::WidgetCapabilitiesV2::new().with_pointer_motion(self)
    }

    fn synchronize_from_previous(&mut self, previous: &dyn Widget) {
        let Some(previous) = previous.as_any().downcast_ref::<Self>() else {
            return;
        };
        self.common.state = previous.common.state;
        self.state = previous.state;
    }

    fn append_paint(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        bounds: Rect,
        _layout: &LayoutOutput,
        theme: &ThemeTokens,
    ) {
        paint::push_badge_widget_paint(primitives, self, bounds, theme);
    }

    fn append_paint_with_context(&self, context: &mut WidgetPaintContext<'_>) {
        paint::push_badge_widget_paint_with_context(context, self);
    }
}

#[cfg(test)]
#[path = "badge/tests.rs"]
mod tests;
