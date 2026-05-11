//! Reusable button primitive.

mod input;
mod model;
mod paint;

use crate::gui::types::Rect;
use crate::layout::LayoutOutput;
use crate::runtime::{PaintPrimitive, PaintText, SurfaceNode, WidgetMessageMapper};
use crate::theme::ThemeTokens;

use super::support::WidgetCommon;
use crate::widgets::contract::{FocusBehavior, Widget, WidgetId, WidgetSizing};
use crate::widgets::interaction::{ButtonMessage, WidgetInput, WidgetOutput};

pub use model::{ButtonProps, ButtonState};

/// Public button primitive.
#[derive(Clone, Debug, PartialEq)]
pub struct ButtonWidget {
    /// Shared widget contract.
    pub common: WidgetCommon,
    /// Immutable user-facing button configuration.
    pub props: ButtonProps,
    /// Mutable interaction state owned by the button.
    pub state: ButtonState,
}

impl ButtonWidget {
    /// Build a button descriptor with keyboard focus and activation semantics.
    pub fn new(id: WidgetId, label: impl Into<PaintText>, sizing: WidgetSizing) -> Self {
        let mut common = WidgetCommon::new(id, sizing);
        common.focus = FocusBehavior::Keyboard;
        Self {
            common,
            props: ButtonProps {
                label: label.into(),
                secondary_click: false,
                drag: false,
            },
            state: ButtonState::default(),
        }
    }

    /// Enable secondary/right-click activation messages for this button.
    pub fn with_secondary_click(mut self) -> Self {
        self.props.secondary_click = true;
        self
    }

    /// Enable primary-pointer drag lifecycle messages from the button surface.
    pub fn with_drag(mut self) -> Self {
        self.props.drag = true;
        self
    }

    /// Route one backend-neutral interaction into the button.
    ///
    /// The button emits [`ButtonMessage::Activate`] when a primary press is
    /// released inside bounds or when the focused widget receives Enter/Space.
    pub fn handle_input(&mut self, bounds: Rect, input: WidgetInput) -> Option<ButtonMessage> {
        input::handle_button_input(self, bounds, input)
    }
}

impl Widget for ButtonWidget {
    fn common(&self) -> &WidgetCommon {
        &self.common
    }

    fn common_mut(&mut self) -> &mut WidgetCommon {
        &mut self.common
    }

    fn handle_input(&mut self, bounds: Rect, input: WidgetInput) -> Option<WidgetOutput> {
        ButtonWidget::handle_input(self, bounds, input).map(WidgetOutput::typed)
    }

    fn synchronize_from_previous(&mut self, previous: &dyn Widget) {
        let Some(previous) = previous.as_any().downcast_ref::<Self>() else {
            return;
        };
        self.common.state = previous.common.state;
        self.state = previous.state;
    }

    fn accepts_pointer_move(&self) -> bool {
        false
    }

    fn append_paint(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        bounds: Rect,
        _layout: &LayoutOutput,
        theme: &ThemeTokens,
    ) {
        paint::push_button_widget_paint(primitives, self, bounds, theme);
    }
}

impl<Message> WidgetMessageMapper<Message> {
    /// Build a button-message mapper.
    pub fn button(map: impl Fn(ButtonMessage) -> Message + Send + Sync + 'static) -> Self {
        Self::typed(map)
    }
}

impl<Message> SurfaceNode<Message> {
    /// Build a button leaf node that emits one cloned host message when activated.
    pub fn button(
        id: WidgetId,
        label: impl Into<String>,
        sizing: WidgetSizing,
        message: Message,
    ) -> Self
    where
        Message: Clone + Send + Sync + 'static,
    {
        Self::button_mapped(id, label, sizing, move |_| message.clone())
    }

    /// Build a button leaf node with a custom widget-to-host message mapper.
    pub fn button_mapped(
        id: WidgetId,
        label: impl Into<String>,
        sizing: WidgetSizing,
        map: impl Fn(ButtonMessage) -> Message + Send + Sync + 'static,
    ) -> Self {
        Self::widget(
            ButtonWidget::new(id, PaintText::from(label.into()), sizing),
            WidgetMessageMapper::button(map),
        )
    }
}

#[cfg(test)]
mod tests {
    use crate::gui::types::{Point, Vector2};

    use super::*;
    use crate::widgets::interaction::{DragHandleMessage, PointerButton, WidgetInput, WidgetKey};

    #[test]
    fn button_releases_inside_bounds_emit_activation() {
        let mut button =
            ButtonWidget::new(5, "Play", WidgetSizing::fixed(Vector2::new(80.0, 28.0)));
        let bounds = Rect::from_min_size(Point::new(10.0, 20.0), Vector2::new(80.0, 28.0));

        assert_eq!(
            button.handle_input(
                bounds,
                WidgetInput::PointerPress {
                    position: Point::new(20.0, 30.0),
                    button: PointerButton::Primary,
                },
            ),
            None
        );
        assert!(button.common.state.pressed);

        assert_eq!(
            button.handle_input(
                bounds,
                WidgetInput::PointerRelease {
                    position: Point::new(24.0, 32.0),
                    button: PointerButton::Primary,
                },
            ),
            Some(ButtonMessage::Activate)
        );
        assert!(!button.common.state.pressed);
    }

    #[test]
    fn focused_button_space_emits_activation() {
        let mut button =
            ButtonWidget::new(6, "Stop", WidgetSizing::fixed(Vector2::new(80.0, 28.0)));

        let _ = button.handle_input(Rect::default(), WidgetInput::FocusChanged(true));

        assert_eq!(
            button.handle_input(Rect::default(), WidgetInput::KeyPress(WidgetKey::Space)),
            Some(ButtonMessage::Activate)
        );
    }

    #[test]
    fn secondary_click_only_emits_when_enabled() {
        let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(80.0, 28.0));
        let mut default_button =
            ButtonWidget::new(7, "More", WidgetSizing::fixed(Vector2::new(80.0, 28.0)));
        let mut context_button =
            ButtonWidget::new(8, "More", WidgetSizing::fixed(Vector2::new(80.0, 28.0)))
                .with_secondary_click();

        let secondary_press = WidgetInput::PointerPress {
            position: Point::new(10.0, 10.0),
            button: PointerButton::Secondary,
        };

        assert_eq!(
            default_button.handle_input(bounds, secondary_press.clone()),
            None
        );
        assert_eq!(
            context_button.handle_input(bounds, secondary_press),
            Some(ButtonMessage::SecondaryActivate {
                position: Point::new(10.0, 10.0),
            })
        );
    }

    #[test]
    fn draggable_button_emits_drag_lifecycle_instead_of_click_when_moved() {
        let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(80.0, 28.0));
        let mut button =
            ButtonWidget::new(9, "Folder", WidgetSizing::fixed(Vector2::new(80.0, 28.0)))
                .with_drag();

        assert_eq!(
            button.handle_input(
                bounds,
                WidgetInput::PointerPress {
                    position: Point::new(10.0, 10.0),
                    button: PointerButton::Primary,
                },
            ),
            None
        );
        assert_eq!(
            button.handle_input(
                bounds,
                WidgetInput::PointerMove {
                    position: Point::new(12.0, 14.0),
                },
            ),
            Some(ButtonMessage::Drag(DragHandleMessage::Started {
                position: Point::new(12.0, 14.0)
            }))
        );
        assert_eq!(
            button.handle_input(
                bounds,
                WidgetInput::PointerRelease {
                    position: Point::new(20.0, 22.0),
                    button: PointerButton::Primary,
                },
            ),
            Some(ButtonMessage::Drag(DragHandleMessage::Ended {
                position: Point::new(20.0, 22.0)
            }))
        );
    }
}
