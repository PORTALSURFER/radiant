//! Reusable horizontal slider primitive.

use crate::gui::types::{Point, Rect, Vector2};
use crate::layout::LayoutOutput;
use crate::runtime::{
    PaintFillRect, PaintPrimitive, PaintStrokeRect, SurfaceNode, WidgetMessageMapper,
};
use crate::theme::ThemeTokens;

use super::support::{WidgetCommon, clamp_fraction};
use crate::widgets::contract::{FocusBehavior, PaintBounds, Widget, WidgetId, WidgetSizing};
use crate::widgets::interaction::{
    PointerButton, SliderMessage, WidgetInput, WidgetKey, WidgetOutput,
};

const TRACK_HEIGHT: f32 = 6.0;
const THUMB_WIDTH: f32 = 12.0;
const DEFAULT_KEYBOARD_STEP: f32 = 0.05;

/// Immutable slider configuration.
#[derive(Clone, Debug, PartialEq)]
pub struct SliderProps {
    /// Normalized amount applied for each arrow-key step.
    pub keyboard_step: f32,
}

/// Mutable slider interaction state.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct SliderState {
    /// Current normalized value in the inclusive range `0.0..=1.0`.
    pub value: f32,
}

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

impl SliderWidget {
    /// Build a horizontal slider with normalized value-change semantics.
    pub fn new(id: WidgetId, value: f32, sizing: WidgetSizing) -> Self {
        let mut common = WidgetCommon::new(id, sizing);
        common.focus = FocusBehavior::Keyboard;
        common.paint.bounds = PaintBounds::ClipToRect;
        Self {
            common,
            props: SliderProps {
                keyboard_step: DEFAULT_KEYBOARD_STEP,
            },
            state: SliderState {
                value: clamp_fraction(value),
            },
        }
    }

    /// Return this slider with an explicit normalized value.
    pub fn with_value(mut self, value: f32) -> Self {
        self.state.value = clamp_fraction(value);
        self
    }

    /// Return the current thumb rectangle inside the provided bounds.
    pub fn thumb_rect(&self, bounds: Rect) -> Rect {
        let track = track_rect(bounds);
        let x = track.min.x + self.state.value * track.width();
        Rect::from_min_size(
            Point::new(x - THUMB_WIDTH * 0.5, bounds.min.y),
            Vector2::new(THUMB_WIDTH, bounds.height()),
        )
    }

    /// Route one backend-neutral interaction into the slider.
    pub fn handle_input(&mut self, bounds: Rect, input: WidgetInput) -> Option<SliderMessage> {
        match input {
            WidgetInput::PointerMove { position } => {
                self.common.state.hovered = bounds.contains(position);
                self.common
                    .state
                    .pressed
                    .then(|| self.set_value(value_for_position(bounds, position)))
                    .flatten()
            }
            WidgetInput::PointerPress {
                position,
                button: PointerButton::Primary,
            } if bounds.contains(position) => {
                self.common.state.hovered = true;
                self.common.state.pressed = true;
                self.common.state.focused = true;
                self.set_value(value_for_position(bounds, position))
            }
            WidgetInput::PointerRelease {
                position,
                button: PointerButton::Primary,
            } => {
                let was_pressed = self.common.state.pressed;
                self.common.state.pressed = false;
                was_pressed
                    .then(|| self.set_value(value_for_position(bounds, position)))
                    .flatten()
            }
            WidgetInput::FocusChanged(focused) => {
                self.common.state.focused = focused;
                None
            }
            WidgetInput::KeyPress(key) if self.common.state.focused => match key {
                WidgetKey::ArrowLeft | WidgetKey::ArrowDown => {
                    self.set_value(self.state.value - self.props.keyboard_step)
                }
                WidgetKey::ArrowRight | WidgetKey::ArrowUp => {
                    self.set_value(self.state.value + self.props.keyboard_step)
                }
                WidgetKey::Home => self.set_value(0.0),
                WidgetKey::End => self.set_value(1.0),
                _ => None,
            },
            _ => None,
        }
    }

    fn set_value(&mut self, value: f32) -> Option<SliderMessage> {
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
        push_slider_widget_paint(primitives, self, bounds, theme);
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

fn value_for_position(bounds: Rect, position: Point) -> f32 {
    let track = track_rect(bounds);
    if track.width() <= f32::EPSILON {
        return 0.0;
    }
    clamp_fraction((position.x - track.min.x) / track.width())
}

fn track_rect(bounds: Rect) -> Rect {
    let y = bounds.min.y + (bounds.height() - TRACK_HEIGHT) * 0.5;
    let inset = (THUMB_WIDTH * 0.5).min(bounds.width() * 0.5);
    Rect::from_min_max(
        Point::new(bounds.min.x + inset, y),
        Point::new(bounds.max.x - inset, y + TRACK_HEIGHT),
    )
}

fn push_slider_widget_paint(
    primitives: &mut Vec<PaintPrimitive>,
    slider: &SliderWidget,
    bounds: Rect,
    theme: &ThemeTokens,
) {
    let track = track_rect(bounds);
    let thumb = slider.thumb_rect(bounds);
    let tokens = crate::widgets::resolve_widget_visual_tokens(
        theme,
        slider.common.style,
        slider.common.state,
    );
    primitives.push(PaintPrimitive::FillRect(PaintFillRect {
        widget_id: slider.common.id,
        rect: track,
        color: theme.bg_tertiary,
    }));
    primitives.push(PaintPrimitive::FillRect(PaintFillRect {
        widget_id: slider.common.id,
        rect: Rect::from_min_max(track.min, Point::new(thumb.center().x, track.max.y)),
        color: tokens.emphasis,
    }));
    primitives.push(PaintPrimitive::FillRect(PaintFillRect {
        widget_id: slider.common.id,
        rect: thumb,
        color: tokens.fill,
    }));
    primitives.push(PaintPrimitive::StrokeRect(PaintStrokeRect {
        widget_id: slider.common.id,
        rect: thumb,
        color: tokens.border,
        width: 1.0,
    }));
    if slider.common.state.focused && slider.common.paint.paints_focus {
        primitives.push(PaintPrimitive::StrokeRect(PaintStrokeRect {
            widget_id: slider.common.id,
            rect: bounds,
            color: tokens.emphasis,
            width: 1.0,
        }));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
