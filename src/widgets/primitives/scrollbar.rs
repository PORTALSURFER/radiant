//! Reusable scrollbar primitive.

use crate::gui::types::Rect;
use crate::layout::LayoutOutput;
use crate::runtime::{PaintPrimitive, SurfaceNode, WidgetMessageMapper};
use crate::theme::ThemeTokens;

use super::support::{WidgetCommon, clamp_fraction, push_scrollbar_widget_paint};
use crate::widgets::contract::{FocusBehavior, PaintBounds, Widget, WidgetId, WidgetSizing};
use crate::widgets::interaction::{ScrollbarMessage, WidgetInput, WidgetOutput};

mod geometry;
mod input;
use geometry::{axis_length, axis_rect, axis_start};

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
        input::handle_scrollbar_input(self, bounds, input)
    }

    pub(super) fn set_offset_fraction(&mut self, value: f32) -> Option<ScrollbarMessage> {
        let clamped = clamp_fraction(value);
        if (self.state.offset_fraction - clamped).abs() <= f32::EPSILON {
            return None;
        }
        self.state.offset_fraction = clamped;
        Some(ScrollbarMessage::OffsetChanged {
            offset_fraction: clamped,
        })
    }

    pub(super) fn thumb_fraction(&self, track_length: f32) -> f32 {
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
