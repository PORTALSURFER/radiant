//! Reusable horizontal slider primitive.

mod geometry;
mod input;
mod model;
mod paint;

use crate::gui::types::Rect;
use crate::layout::LayoutOutput;
use crate::runtime::{PaintPrimitive, SurfaceNode, WidgetMessageMapper};
use crate::theme::ThemeTokens;

use super::support::{WidgetCommon, clamp_fraction};
use crate::widgets::contract::{FocusBehavior, PaintBounds, Widget, WidgetId, WidgetSizing};
use crate::widgets::interaction::{SliderMessage, WidgetInput, WidgetOutput};

pub use model::{SliderProps, SliderState};

const DEFAULT_KEYBOARD_STEP: f32 = 0.05;

/// Public horizontal slider primitive.
#[derive(Clone, Debug, PartialEq)]
pub struct SliderWidget {
    /// Shared widget contract.
    pub common: WidgetCommon,
    /// Immutable slider configuration.
    pub props: SliderProps,
    /// Mutable slider state owned by the widget.
    pub state: SliderState,
}

/// Named construction fields for [`SliderWidget`].
#[derive(Clone, Debug, PartialEq)]
pub struct SliderWidgetParts {
    /// Stable widget identity used by layout, events, and state synchronization.
    pub id: WidgetId,
    /// Initial normalized slider value.
    pub value: f32,
    /// Intrinsic slider sizing contract.
    pub sizing: WidgetSizing,
}

impl SliderWidget {
    /// Build a horizontal slider from named identity, value, and sizing fields.
    pub fn from_parts(parts: SliderWidgetParts) -> Self {
        let mut common = WidgetCommon::new(parts.id, parts.sizing);
        common.focus = FocusBehavior::Keyboard;
        common.paint.bounds = PaintBounds::ClipToRect;
        Self {
            common,
            props: SliderProps {
                keyboard_step: DEFAULT_KEYBOARD_STEP,
            },
            state: SliderState {
                value: clamp_fraction(parts.value),
            },
        }
    }

    /// Build a horizontal slider with normalized value-change semantics.
    pub fn new(id: WidgetId, value: f32, sizing: WidgetSizing) -> Self {
        Self::from_parts(SliderWidgetParts { id, value, sizing })
    }

    /// Return this slider with an explicit normalized value.
    pub fn with_value(mut self, value: f32) -> Self {
        self.state.value = clamp_fraction(value);
        self
    }

    /// Return the current thumb rectangle inside the provided bounds.
    pub fn thumb_rect(&self, bounds: Rect) -> Rect {
        geometry::thumb_rect(bounds, self.state.value)
    }

    /// Route one backend-neutral interaction into the slider.
    pub fn handle_input(&mut self, bounds: Rect, input: WidgetInput) -> Option<SliderMessage> {
        input::handle_slider_input(self, bounds, input)
    }

    pub(super) fn set_value(&mut self, value: f32) -> Option<SliderMessage> {
        let value = clamp_fraction(value);
        if (self.state.value - value).abs() <= f32::EPSILON {
            return None;
        }
        self.state.value = value;
        Some(SliderMessage::ValueChanged { value })
    }
}

impl Widget for SliderWidget {
    fn common(&self) -> &WidgetCommon {
        &self.common
    }

    fn common_mut(&mut self) -> &mut WidgetCommon {
        &mut self.common
    }

    fn handle_input(&mut self, bounds: Rect, input: WidgetInput) -> Option<WidgetOutput> {
        SliderWidget::handle_input(self, bounds, input).map(WidgetOutput::typed)
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
        paint::push_slider_widget_paint(primitives, self, bounds, theme);
    }
}

impl<Message> WidgetMessageMapper<Message> {
    /// Build a slider-message mapper.
    pub fn slider(map: impl Fn(SliderMessage) -> Message + Send + Sync + 'static) -> Self {
        Self::typed(map)
    }
}

impl<Message> SurfaceNode<Message> {
    /// Build a slider leaf that maps value changes by normalized value.
    pub fn slider(
        id: WidgetId,
        value: f32,
        sizing: WidgetSizing,
        map: impl Fn(f32) -> Message + Send + Sync + 'static,
    ) -> Self {
        Self::slider_mapped(id, value, sizing, move |message| match message {
            SliderMessage::ValueChanged { value } => map(value),
        })
    }

    /// Build a slider leaf with a custom widget-to-host message mapper.
    pub fn slider_mapped(
        id: WidgetId,
        value: f32,
        sizing: WidgetSizing,
        map: impl Fn(SliderMessage) -> Message + Send + Sync + 'static,
    ) -> Self {
        Self::widget(
            SliderWidget::new(id, value, sizing),
            WidgetMessageMapper::slider(map),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gui::types::{Point, Vector2};
    use crate::widgets::interaction::{PointerButton, WidgetKey};

    #[test]
    fn slider_pointer_drag_emits_clamped_values() {
        let mut slider = SliderWidget::new(9, 0.25, WidgetSizing::fixed(Vector2::new(120.0, 28.0)));
        let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(120.0, 28.0));

        assert_eq!(
            slider.handle_input(
                bounds,
                WidgetInput::PointerPress {
                    position: Point::new(60.0, 14.0),
                    button: PointerButton::Primary,
                    modifiers: Default::default(),
                },
            ),
            Some(SliderMessage::ValueChanged { value: 0.5 })
        );
        assert_eq!(
            slider.handle_input(
                bounds,
                WidgetInput::PointerMove {
                    position: Point::new(180.0, 14.0),
                },
            ),
            Some(SliderMessage::ValueChanged { value: 1.0 })
        );
    }

    #[test]
    fn focused_slider_responds_to_keyboard_steps() {
        let mut slider = SliderWidget::new(10, 0.5, WidgetSizing::fixed(Vector2::new(120.0, 28.0)));

        let _ = slider.handle_input(Rect::default(), WidgetInput::FocusChanged(true));
        let Some(SliderMessage::ValueChanged { value }) = slider.handle_input(
            Rect::default(),
            WidgetInput::KeyPress(WidgetKey::ArrowRight),
        ) else {
            panic!("focused slider should emit an arrow-key change");
        };
        assert!((value - 0.55).abs() < f32::EPSILON);
        assert_eq!(
            slider.handle_input(Rect::default(), WidgetInput::KeyPress(WidgetKey::Home)),
            Some(SliderMessage::ValueChanged { value: 0.0 })
        );
    }
}
