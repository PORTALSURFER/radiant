//! Reusable scrollbar primitive.

use crate::gui::types::{Point, Rect, Vector2};
use crate::layout::LayoutOutput;
use crate::runtime::{PaintPrimitive, SurfaceNode, WidgetMessageMapper};
use crate::theme::ThemeTokens;

use super::support::{
    WidgetCommon, clamp_fraction, leading_arrow_for_axis, push_scrollbar_widget_paint,
    trailing_arrow_for_axis,
};
use crate::widgets::contract::{FocusBehavior, PaintBounds, Widget, WidgetId, WidgetSizing};
use crate::widgets::interaction::{
    PointerButton, ScrollbarMessage, WidgetInput, WidgetKey, WidgetOutput,
};

const MIN_THUMB_PIXELS: f32 = 12.0;

/// Scrollbar orientation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ScrollbarAxis {
    /// Horizontal scroll direction.
    Horizontal,
    /// Vertical scroll direction.
    Vertical,
}

/// Immutable public properties for a reusable scrollbar widget.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ScrollbarProps {
    /// Scroll direction represented by the scrollbar.
    pub axis: ScrollbarAxis,
    /// Fraction of the full content currently visible inside the viewport.
    pub viewport_fraction: f32,
    /// Fraction moved by one keyboard arrow press.
    pub step_fraction: f32,
}

/// Mutable interaction state for a reusable scrollbar widget.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ScrollbarState {
    /// Normalized viewport start position.
    pub offset_fraction: f32,
    /// Drag grip inside the thumb measured as `0.0..=1.0` of the thumb length.
    pub drag_grip_fraction: Option<f32>,
}

/// Public scrollbar primitive.
#[derive(Clone, Debug, PartialEq)]
pub struct ScrollbarWidget {
    /// Shared widget contract.
    pub common: WidgetCommon,
    /// Immutable user-facing scrollbar configuration.
    pub props: ScrollbarProps,
    /// Mutable scrollbar state owned by the widget.
    pub state: ScrollbarState,
}

impl ScrollbarWidget {
    /// Build a scrollbar descriptor with drag/page request semantics.
    pub fn new(id: WidgetId, axis: ScrollbarAxis, sizing: WidgetSizing) -> Self {
        let mut common = WidgetCommon::new(id, sizing);
        common.focus = FocusBehavior::Pointer;
        common.paint.bounds = PaintBounds::ClipToRect;
        Self {
            common,
            props: ScrollbarProps {
                axis,
                viewport_fraction: 1.0,
                step_fraction: 0.1,
            },
            state: ScrollbarState::default(),
        }
    }

    /// Return the current thumb rectangle inside the provided track bounds.
    pub fn thumb_rect(&self, bounds: Rect) -> Rect {
        let track_length = axis_length(self.props.axis, bounds).max(0.0);
        let thumb_fraction = self.thumb_fraction(track_length);
        let thumb_length = track_length * thumb_fraction;
        let start_fraction = self.state.offset_fraction * (1.0 - thumb_fraction);
        let start = axis_start(self.props.axis, bounds) + start_fraction * track_length;
        axis_rect(self.props.axis, bounds, start, thumb_length)
    }

    /// Route one backend-neutral interaction into the scrollbar.
    pub fn handle_input(&mut self, bounds: Rect, input: WidgetInput) -> Option<ScrollbarMessage> {
        if self.common.state.disabled {
            self.common.state.pressed = false;
            self.state.drag_grip_fraction = None;
            return None;
        }
        match input {
            WidgetInput::PointerMove { position } => {
                self.common.state.hovered = bounds.contains(position);
                self.drag_to(bounds, position)
            }
            WidgetInput::PointerPress {
                position,
                button: PointerButton::Primary,
            } if bounds.contains(position) => {
                self.common.state.focused = true;
                self.common.state.hovered = true;
                let thumb = self.thumb_rect(bounds);
                if thumb.contains(position) {
                    self.common.state.pressed = true;
                    self.state.drag_grip_fraction =
                        Some(self.pointer_grip_fraction(thumb, position));
                    None
                } else {
                    self.set_offset_fraction(self.centered_offset_fraction(bounds, position))
                }
            }
            WidgetInput::PointerRelease {
                position,
                button: PointerButton::Primary,
            } => {
                self.common.state.hovered = bounds.contains(position);
                self.common.state.pressed = false;
                self.state.drag_grip_fraction = None;
                None
            }
            WidgetInput::FocusChanged(focused) => {
                self.common.state.focused = focused;
                if !focused {
                    self.common.state.pressed = false;
                    self.state.drag_grip_fraction = None;
                }
                None
            }
            WidgetInput::KeyPress(key) if self.common.state.focused => self.handle_key_input(key),
            _ => None,
        }
    }

    fn handle_key_input(&mut self, key: WidgetKey) -> Option<ScrollbarMessage> {
        let delta = if key == leading_arrow_for_axis(self.props.axis) {
            Some(-self.props.step_fraction)
        } else if key == trailing_arrow_for_axis(self.props.axis) {
            Some(self.props.step_fraction)
        } else {
            None
        };
        match key {
            WidgetKey::Home => self.set_offset_fraction(0.0),
            WidgetKey::End => self.set_offset_fraction(1.0),
            _ => delta.and_then(|step| self.set_offset_fraction(self.state.offset_fraction + step)),
        }
    }

    fn drag_to(&mut self, bounds: Rect, position: Point) -> Option<ScrollbarMessage> {
        let Some(grip_fraction) = self.state.drag_grip_fraction else {
            return None;
        };
        let track_length = axis_length(self.props.axis, bounds);
        let thumb_fraction = self.thumb_fraction(track_length);
        let thumb_length = track_length * thumb_fraction;
        let pointer_axis = axis_position(self.props.axis, position);
        let start = pointer_axis - thumb_length * grip_fraction;
        let free_track = (track_length - thumb_length).max(0.0);
        let offset = if free_track <= f32::EPSILON {
            0.0
        } else {
            (start - axis_start(self.props.axis, bounds)) / free_track
        };
        self.set_offset_fraction(offset)
    }

    fn centered_offset_fraction(&self, bounds: Rect, position: Point) -> f32 {
        let track_length = axis_length(self.props.axis, bounds);
        let thumb_fraction = self.thumb_fraction(track_length);
        let thumb_length = track_length * thumb_fraction;
        let centered_start = axis_position(self.props.axis, position)
            - axis_start(self.props.axis, bounds)
            - thumb_length * 0.5;
        let free_track = (track_length - thumb_length).max(0.0);
        if free_track <= f32::EPSILON {
            0.0
        } else {
            centered_start / free_track
        }
    }

    fn pointer_grip_fraction(&self, thumb: Rect, position: Point) -> f32 {
        let grip = axis_position(self.props.axis, position) - axis_start(self.props.axis, thumb);
        let thumb_length = axis_length(self.props.axis, thumb).max(1.0);
        clamp_fraction(grip / thumb_length)
    }

    fn set_offset_fraction(&mut self, value: f32) -> Option<ScrollbarMessage> {
        let clamped = clamp_fraction(value);
        if (self.state.offset_fraction - clamped).abs() <= f32::EPSILON {
            return None;
        }
        self.state.offset_fraction = clamped;
        Some(ScrollbarMessage::OffsetChanged {
            offset_fraction: clamped,
        })
    }

    fn thumb_fraction(&self, track_length: f32) -> f32 {
        let viewport = clamp_fraction(self.props.viewport_fraction);
        if track_length <= f32::EPSILON {
            return 1.0;
        }
        viewport.max((MIN_THUMB_PIXELS / track_length).min(1.0))
    }
}

impl Widget for ScrollbarWidget {
    fn common(&self) -> &WidgetCommon {
        &self.common
    }

    fn common_mut(&mut self) -> &mut WidgetCommon {
        &mut self.common
    }

    fn handle_input(&mut self, bounds: Rect, input: WidgetInput) -> Option<WidgetOutput> {
        ScrollbarWidget::handle_input(self, bounds, input).map(WidgetOutput::typed)
    }

    fn append_paint(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        bounds: Rect,
        _layout: &LayoutOutput,
        theme: &ThemeTokens,
    ) {
        push_scrollbar_widget_paint(primitives, self, bounds, theme);
    }
}

impl<Message> WidgetMessageMapper<Message> {
    /// Build a scrollbar-message mapper.
    pub fn scrollbar(map: impl Fn(ScrollbarMessage) -> Message + Send + Sync + 'static) -> Self {
        Self::typed(map)
    }
}

impl<Message> SurfaceNode<Message> {
    /// Build a scrollbar leaf that maps offset changes by normalized offset.
    pub fn scrollbar(
        id: WidgetId,
        axis: ScrollbarAxis,
        sizing: WidgetSizing,
        map: impl Fn(f32) -> Message + Send + Sync + 'static,
    ) -> Self {
        Self::scrollbar_mapped(id, axis, sizing, move |message| match message {
            ScrollbarMessage::OffsetChanged { offset_fraction } => map(offset_fraction),
        })
    }

    /// Build a scrollbar leaf with a custom widget-to-host message mapper.
    pub fn scrollbar_mapped(
        id: WidgetId,
        axis: ScrollbarAxis,
        sizing: WidgetSizing,
        map: impl Fn(ScrollbarMessage) -> Message + Send + Sync + 'static,
    ) -> Self {
        Self::widget(
            ScrollbarWidget::new(id, axis, sizing),
            WidgetMessageMapper::scrollbar(map),
        )
    }
}

fn axis_length(axis: ScrollbarAxis, rect: Rect) -> f32 {
    match axis {
        ScrollbarAxis::Horizontal => rect.width(),
        ScrollbarAxis::Vertical => rect.height(),
    }
}

fn axis_start(axis: ScrollbarAxis, rect: Rect) -> f32 {
    match axis {
        ScrollbarAxis::Horizontal => rect.min.x,
        ScrollbarAxis::Vertical => rect.min.y,
    }
}

fn axis_position(axis: ScrollbarAxis, point: Point) -> f32 {
    match axis {
        ScrollbarAxis::Horizontal => point.x,
        ScrollbarAxis::Vertical => point.y,
    }
}

fn axis_rect(axis: ScrollbarAxis, bounds: Rect, start: f32, length: f32) -> Rect {
    match axis {
        ScrollbarAxis::Horizontal => Rect::from_min_size(
            Point::new(start, bounds.min.y),
            Vector2::new(length, bounds.height()),
        ),
        ScrollbarAxis::Vertical => Rect::from_min_size(
            Point::new(bounds.min.x, start),
            Vector2::new(bounds.width(), length),
        ),
    }
}

#[cfg(test)]
mod tests {
    use crate::gui::types::{Point, Vector2};

    use super::*;
    use crate::widgets::interaction::{PointerButton, WidgetInput};

    #[test]
    fn scrollbar_drag_emits_clamped_offset_changes() {
        let mut scrollbar = ScrollbarWidget::new(
            9,
            ScrollbarAxis::Vertical,
            WidgetSizing::fixed(Vector2::new(12.0, 120.0)),
        );
        scrollbar.props.viewport_fraction = 0.25;
        let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(12.0, 120.0));
        let thumb = scrollbar.thumb_rect(bounds);
        let grip_y = thumb.min.y + thumb.height() * 0.5;

        assert_eq!(
            scrollbar.handle_input(
                bounds,
                WidgetInput::PointerPress {
                    position: Point::new(6.0, grip_y),
                    button: PointerButton::Primary,
                },
            ),
            None
        );

        let message = scrollbar.handle_input(
            bounds,
            WidgetInput::PointerMove {
                position: Point::new(6.0, 96.0),
            },
        );
        assert_eq!(
            message,
            Some(ScrollbarMessage::OffsetChanged {
                offset_fraction: 0.9,
            })
        );
    }

    #[test]
    fn scrollbar_track_click_centers_thumb() {
        let mut scrollbar = ScrollbarWidget::new(
            10,
            ScrollbarAxis::Horizontal,
            WidgetSizing::fixed(Vector2::new(120.0, 12.0)),
        );
        scrollbar.props.viewport_fraction = 0.5;
        let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(120.0, 12.0));

        assert_eq!(
            scrollbar.handle_input(
                bounds,
                WidgetInput::PointerPress {
                    position: Point::new(90.0, 6.0),
                    button: PointerButton::Primary,
                },
            ),
            Some(ScrollbarMessage::OffsetChanged {
                offset_fraction: 1.0,
            })
        );
    }
}
