//! Reusable toggle primitive.

mod input;
mod model;
mod paint;

use crate::gui::types::Rect;
use crate::layout::LayoutOutput;
use crate::runtime::{PaintPrimitive, SurfaceNode, WidgetMessageMapper};
use crate::theme::ThemeTokens;

use super::support::WidgetCommon;
use crate::widgets::contract::{FocusBehavior, Widget, WidgetId, WidgetSizing};
use crate::widgets::interaction::{ToggleMessage, WidgetInput, WidgetOutput};

pub use model::{ToggleProps, ToggleState};

/// Public toggle primitive.
#[derive(Clone, Debug, PartialEq)]
pub struct ToggleWidget {
    /// Shared widget contract.
    pub common: WidgetCommon,
    /// Immutable user-facing toggle configuration.
    pub props: ToggleProps,
    /// Mutable interaction state owned by the toggle.
    pub state: ToggleState,
}

impl ToggleWidget {
    /// Build a toggle descriptor with value-change semantics.
    pub fn new(id: WidgetId, label: impl Into<String>, sizing: WidgetSizing) -> Self {
        let mut common = WidgetCommon::new(id, sizing);
        common.focus = FocusBehavior::Keyboard;
        Self {
            common,
            props: ToggleProps {
                label: label.into(),
            },
            state: ToggleState::default(),
        }
    }

    /// Return this toggle with an explicit checked value.
    pub fn with_checked(mut self, checked: bool) -> Self {
        self.state.checked = checked;
        self.common.state.active = checked;
        self
    }

    /// Route one backend-neutral interaction into the toggle.
    pub fn handle_input(&mut self, bounds: Rect, input: WidgetInput) -> Option<ToggleMessage> {
        input::handle_toggle_input(self, bounds, input)
    }
}

impl ToggleWidget {
    pub(super) fn toggle(&mut self) -> ToggleMessage {
        self.state.checked = !self.state.checked;
        self.common.state.active = self.state.checked;
        ToggleMessage::ValueChanged {
            checked: self.state.checked,
        }
    }
}

impl Widget for ToggleWidget {
    fn common(&self) -> &WidgetCommon {
        &self.common
    }

    fn common_mut(&mut self) -> &mut WidgetCommon {
        &mut self.common
    }

    fn handle_input(&mut self, bounds: Rect, input: WidgetInput) -> Option<WidgetOutput> {
        ToggleWidget::handle_input(self, bounds, input).map(WidgetOutput::typed)
    }

    fn append_paint(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        bounds: Rect,
        _layout: &LayoutOutput,
        theme: &ThemeTokens,
    ) {
        paint::push_toggle_widget_paint(primitives, self, bounds, theme);
    }
}

impl<Message> WidgetMessageMapper<Message> {
    /// Build a toggle-message mapper.
    pub fn toggle(map: impl Fn(ToggleMessage) -> Message + Send + Sync + 'static) -> Self {
        Self::typed(map)
    }
}

impl<Message> SurfaceNode<Message> {
    /// Build a toggle leaf that maps value changes by checked state.
    pub fn toggle(
        id: WidgetId,
        label: impl Into<String>,
        sizing: WidgetSizing,
        map: impl Fn(bool) -> Message + Send + Sync + 'static,
    ) -> Self {
        Self::toggle_with_checked(id, label, false, sizing, map)
    }

    /// Build a toggle leaf with an explicit checked state.
    pub fn toggle_with_checked(
        id: WidgetId,
        label: impl Into<String>,
        checked: bool,
        sizing: WidgetSizing,
        map: impl Fn(bool) -> Message + Send + Sync + 'static,
    ) -> Self {
        Self::toggle_mapped_with_checked(id, label, checked, sizing, move |message| match message {
            ToggleMessage::ValueChanged { checked } => map(checked),
        })
    }

    /// Build a toggle leaf with a custom widget-to-host message mapper.
    pub fn toggle_mapped(
        id: WidgetId,
        label: impl Into<String>,
        sizing: WidgetSizing,
        map: impl Fn(ToggleMessage) -> Message + Send + Sync + 'static,
    ) -> Self {
        Self::toggle_mapped_with_checked(id, label, false, sizing, map)
    }

    /// Build a toggle leaf with explicit checked state and a custom mapper.
    pub fn toggle_mapped_with_checked(
        id: WidgetId,
        label: impl Into<String>,
        checked: bool,
        sizing: WidgetSizing,
        map: impl Fn(ToggleMessage) -> Message + Send + Sync + 'static,
    ) -> Self {
        Self::widget(
            ToggleWidget::new(id, label, sizing).with_checked(checked),
            WidgetMessageMapper::toggle(map),
        )
    }
}

#[cfg(test)]
mod tests {
    use crate::gui::types::{Point, Vector2};

    use super::*;
    use crate::widgets::interaction::{PointerButton, WidgetKey};

    #[test]
    fn toggle_keyboard_activation_flips_active_state() {
        let mut toggle =
            ToggleWidget::new(8, "Snap", WidgetSizing::fixed(Vector2::new(88.0, 28.0)));
        let _ = toggle.handle_input(Rect::default(), WidgetInput::FocusChanged(true));

        assert_eq!(
            toggle.handle_input(Rect::default(), WidgetInput::KeyPress(WidgetKey::Enter)),
            Some(ToggleMessage::ValueChanged { checked: true })
        );
        assert!(toggle.common.state.active);

        let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(88.0, 28.0));
        assert_eq!(
            toggle.handle_input(
                bounds,
                WidgetInput::PointerPress {
                    position: Point::new(10.0, 10.0),
                    button: PointerButton::Primary,
                },
            ),
            None
        );
        assert_eq!(
            toggle.handle_input(
                bounds,
                WidgetInput::PointerRelease {
                    position: Point::new(10.0, 10.0),
                    button: PointerButton::Primary,
                },
            ),
            Some(ToggleMessage::ValueChanged { checked: false })
        );
    }
}
