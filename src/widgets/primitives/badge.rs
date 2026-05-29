//! Reusable badge and pill primitive.

use crate::gui::types::Rect;
use crate::layout::LayoutOutput;
use crate::runtime::{PaintPrimitive, PaintText};
use crate::theme::ThemeTokens;

use super::support::WidgetCommon;
use crate::widgets::contract::{
    FocusBehavior, Widget, WidgetId, WidgetProminence, WidgetSizing, WidgetStyle, WidgetTone,
};
use crate::widgets::interaction::{BadgeMessage, WidgetInput, WidgetOutput};

mod builders;
mod input;
mod model;
mod paint;

pub use model::{BadgeProps, BadgeState};

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
            props: BadgeProps { label: parts.label },
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

    /// Route one backend-neutral interaction into the badge.
    pub fn handle_input(&mut self, bounds: Rect, input: WidgetInput) -> Option<BadgeMessage> {
        input::handle_badge_input(self, bounds, input)
    }
}

impl Widget for BadgeWidget {
    fn common(&self) -> &WidgetCommon {
        &self.common
    }

    fn common_mut(&mut self) -> &mut WidgetCommon {
        &mut self.common
    }

    fn handle_input(&mut self, bounds: Rect, input: WidgetInput) -> Option<WidgetOutput> {
        BadgeWidget::handle_input(self, bounds, input).map(WidgetOutput::typed)
    }

    fn accepts_pointer_move(&self) -> bool {
        false
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
}

#[cfg(test)]
#[path = "badge/tests.rs"]
mod tests;
