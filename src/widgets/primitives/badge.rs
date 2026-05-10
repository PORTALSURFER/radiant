//! Reusable badge and pill primitive.

use crate::gui::types::Rect;
use crate::layout::LayoutOutput;
use crate::runtime::{PaintPrimitive, SurfaceNode, WidgetMessageMapper};
use crate::theme::ThemeTokens;

use super::support::WidgetCommon;
use crate::widgets::contract::{
    FocusBehavior, Widget, WidgetId, WidgetProminence, WidgetSizing, WidgetStyle, WidgetTone,
};
use crate::widgets::interaction::{BadgeMessage, WidgetInput, WidgetOutput};

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

impl BadgeWidget {
    /// Build a badge descriptor with optional activation semantics.
    pub fn new(id: WidgetId, label: impl Into<String>, sizing: WidgetSizing) -> Self {
        let mut common = WidgetCommon::new(id, sizing);
        common.focus = FocusBehavior::Keyboard;
        common.style = WidgetStyle {
            tone: WidgetTone::Neutral,
            prominence: WidgetProminence::Subtle,
        };
        Self {
            common,
            props: BadgeProps {
                label: label.into(),
            },
            state: BadgeState::default(),
        }
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

impl<Message> WidgetMessageMapper<Message> {
    /// Build a badge-message mapper.
    pub fn badge(map: impl Fn(BadgeMessage) -> Message + Send + Sync + 'static) -> Self {
        Self::typed(map)
    }
}

impl<Message> SurfaceNode<Message> {
    /// Build a badge or pill leaf node that emits one cloned host message when activated.
    pub fn badge(
        id: WidgetId,
        label: impl Into<String>,
        sizing: WidgetSizing,
        message: Message,
    ) -> Self
    where
        Message: Clone + Send + Sync + 'static,
    {
        Self::badge_mapped(id, label, sizing, move |_| message.clone())
    }

    /// Build a badge or pill leaf node with a custom widget-to-host message mapper.
    pub fn badge_mapped(
        id: WidgetId,
        label: impl Into<String>,
        sizing: WidgetSizing,
        map: impl Fn(BadgeMessage) -> Message + Send + Sync + 'static,
    ) -> Self {
        Self::widget(
            BadgeWidget::new(id, label, sizing),
            WidgetMessageMapper::badge(map),
        )
    }
}

#[cfg(test)]
mod tests {
    use crate::gui::types::{Point, Vector2};

    use super::*;
    use crate::widgets::interaction::{PointerButton, WidgetInput, WidgetKey};

    #[test]
    fn badge_releases_inside_bounds_emit_activation() {
        let mut badge =
            BadgeWidget::new(5, "Filter", WidgetSizing::fixed(Vector2::new(72.0, 24.0)));
        let bounds = Rect::from_min_size(Point::new(10.0, 20.0), Vector2::new(72.0, 24.0));

        assert_eq!(
            badge.handle_input(
                bounds,
                WidgetInput::PointerPress {
                    position: Point::new(20.0, 30.0),
                    button: PointerButton::Primary,
                },
            ),
            None
        );
        assert!(badge.common.state.pressed);

        assert_eq!(
            badge.handle_input(
                bounds,
                WidgetInput::PointerRelease {
                    position: Point::new(24.0, 32.0),
                    button: PointerButton::Primary,
                },
            ),
            Some(BadgeMessage::Activate)
        );
        assert!(!badge.common.state.pressed);
    }

    #[test]
    fn focused_badge_enter_emits_activation() {
        let mut badge =
            BadgeWidget::new(6, "Active", WidgetSizing::fixed(Vector2::new(72.0, 24.0)));

        let _ = badge.handle_input(Rect::default(), WidgetInput::FocusChanged(true));

        assert_eq!(
            badge.handle_input(Rect::default(), WidgetInput::KeyPress(WidgetKey::Enter)),
            Some(BadgeMessage::Activate)
        );
    }
}
