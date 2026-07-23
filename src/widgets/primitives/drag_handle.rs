//! Reusable drag-handle primitive.

use crate::gui::types::Rect;
use crate::layout::LayoutOutput;
use crate::runtime::PaintPrimitive;
use crate::theme::ThemeTokens;
use std::time::{Duration, Instant};

use super::support::WidgetCommon;
use crate::widgets::contract::{
    FocusBehavior, PointerCapturePolicy, Widget, WidgetId, WidgetSizing,
};
use crate::widgets::interaction::{DragHandleMessage, WidgetInput, WidgetOutput};

mod builders;
mod input;
mod paint;

const HOVER_HIGHLIGHT_DELAY: Duration = Duration::from_millis(100);

/// Public drag handle primitive for pointer-driven reordering.
#[derive(Clone, Debug, PartialEq)]
pub struct DragHandleWidget {
    /// Shared widget contract.
    pub common: WidgetCommon,
    /// Whether idle handle chrome should be hidden until hover, press, or focus.
    pub hover_chrome_only: bool,
    /// Whether the handle paints a continuous passive rail through its bounds.
    pub full_height_rail: bool,
    /// Optional trailing rail width painted independently from the hit target.
    pub trailing_rail_width: Option<f32>,
    /// Whether a released trailing rail stays visually idle until pointer exit.
    pub hover_suppressed_until_exit: bool,
    /// Delay before hover-only handle chrome becomes visible.
    pub hover_highlight_delay: Duration,
    /// Monotonic start time for the current hover-intent window.
    pub hover_started_at: Option<Instant>,
    /// Whether the delayed hover highlight has reached its reveal deadline.
    pub hover_highlight_revealed: bool,
}

/// Named construction fields for [`DragHandleWidget`].
#[derive(Clone, Debug, PartialEq)]
pub struct DragHandleWidgetParts {
    /// Stable widget identity used by layout, events, and state synchronization.
    pub id: WidgetId,
    /// Intrinsic drag-handle sizing contract.
    pub sizing: WidgetSizing,
}

impl DragHandleWidget {
    /// Build a compact handle from named identity and sizing fields.
    pub fn from_parts(parts: DragHandleWidgetParts) -> Self {
        let mut common = WidgetCommon::new(parts.id, parts.sizing);
        common.focus = FocusBehavior::Pointer;
        Self {
            common,
            hover_chrome_only: false,
            full_height_rail: false,
            trailing_rail_width: None,
            hover_suppressed_until_exit: false,
            hover_highlight_delay: Duration::ZERO,
            hover_started_at: None,
            hover_highlight_revealed: false,
        }
    }

    /// Build a compact handle that emits drag lifecycle messages.
    pub fn new(id: WidgetId, sizing: WidgetSizing) -> Self {
        Self::from_parts(DragHandleWidgetParts { id, sizing })
    }

    /// Paint handle chrome only while hovered, pressed, or focused.
    pub fn with_hover_chrome_only(mut self) -> Self {
        self.hover_chrome_only = true;
        self.hover_highlight_delay = HOVER_HIGHLIGHT_DELAY;
        self
    }

    /// Paint a continuous passive rail while preserving drag interaction.
    pub fn with_full_height_rail(mut self) -> Self {
        self.full_height_rail = true;
        self
    }

    /// Paint a slim full-height rail at the trailing edge of a wider hit target.
    pub fn with_trailing_rail(mut self, width: f32) -> Self {
        self.trailing_rail_width = Some(width.max(0.0));
        self
    }

    pub(super) fn hover_highlight_visible(&self) -> bool {
        self.common.state.hovered
            && (self.hover_highlight_delay.is_zero() || self.hover_highlight_revealed)
    }

    /// Route one backend-neutral interaction into the handle.
    pub fn handle_input(&mut self, bounds: Rect, input: WidgetInput) -> Option<DragHandleMessage> {
        input::handle_drag_handle_input(self, bounds, input)
    }
}

impl Widget for DragHandleWidget {
    fn common(&self) -> &WidgetCommon {
        &self.common
    }

    fn common_mut(&mut self) -> &mut WidgetCommon {
        &mut self.common
    }

    fn handle_input(&mut self, bounds: Rect, input: WidgetInput) -> Option<WidgetOutput> {
        DragHandleWidget::handle_input(self, bounds, input).map(WidgetOutput::typed)
    }

    fn synchronize_from_previous(&mut self, previous: &dyn Widget) {
        let Some(previous) = previous.as_any().downcast_ref::<Self>() else {
            return;
        };
        self.common.state = previous.common.state;
        self.hover_suppressed_until_exit = previous.hover_suppressed_until_exit;
        self.hover_started_at = previous.hover_started_at;
        self.hover_highlight_revealed = previous.hover_highlight_revealed;
    }

    fn timed_repaint_deadline(&self) -> Option<Instant> {
        (self.common.state.hovered
            && !self.hover_highlight_revealed
            && !self.hover_highlight_delay.is_zero())
        .then(|| {
            self.hover_started_at?
                .checked_add(self.hover_highlight_delay)
        })
        .flatten()
    }

    fn advance_timed_repaint(&mut self, now: Instant) -> bool {
        let Some(deadline) = self.timed_repaint_deadline() else {
            return false;
        };
        if now < deadline {
            return false;
        }
        self.hover_highlight_revealed = true;
        true
    }

    fn accepts_pointer_move(&self) -> bool {
        false
    }

    fn allows_captured_pointer_pass_through(&self) -> bool {
        false
    }

    fn pointer_capture_policy(&self) -> PointerCapturePolicy {
        PointerCapturePolicy::Exclusive
    }

    fn append_paint(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        bounds: Rect,
        _layout: &LayoutOutput,
        theme: &ThemeTokens,
    ) {
        paint::push_drag_handle_widget_paint(primitives, self, bounds, theme);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        gui::types::{Point, Vector2},
        layout::LayoutOutput,
        runtime::PaintPrimitive,
        theme::ThemeTokens,
        widgets::{PointerButton, WidgetInput, WidgetSizing},
    };

    #[test]
    fn hover_chrome_only_drag_handle_hides_idle_paint() {
        let mut handle = DragHandleWidget::new(7, WidgetSizing::fixed(Vector2::new(8.0, 80.0)))
            .with_hover_chrome_only();
        let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(8.0, 80.0));
        let mut primitives = Vec::new();

        handle.append_paint(
            &mut primitives,
            bounds,
            &LayoutOutput::default(),
            &ThemeTokens::default(),
        );
        assert!(primitives.is_empty());

        let _ = handle.handle_input(bounds, WidgetInput::pointer_move(Point::new(4.0, 20.0)));
        let deadline = handle.timed_repaint_deadline().expect("hover deadline");
        assert!(handle.advance_timed_repaint(deadline));
        handle.append_paint(
            &mut primitives,
            bounds,
            &LayoutOutput::default(),
            &ThemeTokens::default(),
        );
        assert!(
            primitives
                .iter()
                .any(|primitive| matches!(primitive, PaintPrimitive::StrokePolyline(_)))
        );
    }

    #[test]
    fn hover_chrome_only_drag_handle_paints_while_pressed() {
        let mut handle = DragHandleWidget::new(8, WidgetSizing::fixed(Vector2::new(8.0, 80.0)))
            .with_hover_chrome_only();
        let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(8.0, 80.0));

        let _ = handle.handle_input(
            bounds,
            WidgetInput::PointerPress {
                position: Point::new(4.0, 20.0),
                button: PointerButton::Primary,
                modifiers: Default::default(),
            },
        );

        let mut primitives = Vec::new();
        handle.append_paint(
            &mut primitives,
            bounds,
            &LayoutOutput::default(),
            &ThemeTokens::default(),
        );
        assert!(!primitives.is_empty());
    }

    #[test]
    fn full_height_rail_remains_visible_with_hover_only_chrome() {
        let handle = DragHandleWidget::new(9, WidgetSizing::fixed(Vector2::new(1.0, 80.0)))
            .with_hover_chrome_only()
            .with_full_height_rail();
        let bounds = Rect::from_min_size(Point::default(), Vector2::new(1.0, 80.0));
        let mut primitives = Vec::new();

        handle.append_paint(
            &mut primitives,
            bounds,
            &LayoutOutput::default(),
            &ThemeTokens::default(),
        );

        let rails = primitives
            .iter()
            .filter_map(|primitive| match primitive {
                PaintPrimitive::FillRect(fill) => Some(fill),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(rails.len(), 1);
        assert_eq!(rails[0].rect, bounds);
        assert_eq!(rails[0].color, ThemeTokens::default().border_emphasis);
    }

    #[test]
    fn trailing_rail_lights_full_height_without_painting_handle_glyphs() {
        let mut handle = DragHandleWidget::new(10, WidgetSizing::fixed(Vector2::new(5.0, 80.0)))
            .with_hover_chrome_only()
            .with_trailing_rail(1.0);
        let bounds = Rect::from_min_size(Point::default(), Vector2::new(5.0, 80.0));
        let theme = ThemeTokens::default();
        let mut primitives = Vec::new();

        handle.append_paint(&mut primitives, bounds, &LayoutOutput::default(), &theme);
        let idle_rect = primitives
            .iter()
            .find_map(|primitive| match primitive {
                PaintPrimitive::FillRect(fill) => Some(fill.rect),
                _ => None,
            })
            .expect("idle trailing rail");
        assert_eq!(idle_rect.min.x, bounds.max.x - 1.0);
        assert_eq!(idle_rect.height(), bounds.height());

        let _ = handle.handle_input(bounds, WidgetInput::pointer_move(Point::new(2.0, 20.0)));
        primitives.clear();
        handle.append_paint(&mut primitives, bounds, &LayoutOutput::default(), &theme);
        let crossing = primitives
            .iter()
            .find_map(|primitive| match primitive {
                PaintPrimitive::FillRect(fill) => Some(fill),
                _ => None,
            })
            .expect("crossing trailing rail");
        assert_eq!(crossing.color, theme.border_emphasis);

        let deadline = handle.timed_repaint_deadline().expect("hover deadline");
        assert!(handle.advance_timed_repaint(deadline));
        primitives.clear();
        handle.append_paint(&mut primitives, bounds, &LayoutOutput::default(), &theme);
        let hovered = primitives
            .iter()
            .find_map(|primitive| match primitive {
                PaintPrimitive::FillRect(fill) => Some(fill),
                _ => None,
            })
            .expect("hovered trailing rail");
        assert_eq!(hovered.rect, idle_rect);
        assert_ne!(hovered.color, theme.border_emphasis);
        assert_eq!(
            primitives
                .iter()
                .filter(|primitive| matches!(primitive, PaintPrimitive::FillRect(_)))
                .count(),
            1
        );
        assert!(!primitives.iter().any(|primitive| matches!(
            primitive,
            PaintPrimitive::StrokePolyline(_) | PaintPrimitive::StrokeRect(_)
        )));

        let _ = handle.handle_input(bounds, WidgetInput::primary_press(Point::new(2.0, 20.0)));
        let _ = handle.handle_input(bounds, WidgetInput::primary_release(Point::new(2.0, 20.0)));
        let _ = handle.handle_input(bounds, WidgetInput::pointer_move(Point::new(2.0, 20.0)));
        primitives.clear();
        handle.append_paint(&mut primitives, bounds, &LayoutOutput::default(), &theme);
        let released = primitives
            .iter()
            .find_map(|primitive| match primitive {
                PaintPrimitive::FillRect(fill) => Some(fill),
                _ => None,
            })
            .expect("released trailing rail");
        assert_eq!(released.color, theme.border_emphasis);

        let _ = handle.handle_input(bounds, WidgetInput::pointer_move(Point::new(8.0, 20.0)));
        let _ = handle.handle_input(bounds, WidgetInput::pointer_move(Point::new(2.0, 20.0)));
        let deadline = handle.timed_repaint_deadline().expect("re-entry deadline");
        assert!(handle.advance_timed_repaint(deadline));
        primitives.clear();
        handle.append_paint(&mut primitives, bounds, &LayoutOutput::default(), &theme);
        let reentered = primitives
            .iter()
            .find_map(|primitive| match primitive {
                PaintPrimitive::FillRect(fill) => Some(fill),
                _ => None,
            })
            .expect("re-entered trailing rail");
        assert_ne!(reentered.color, theme.border_emphasis);
    }

    #[test]
    fn trailing_rail_reactivates_on_first_reentry_after_release_outside() {
        let mut handle = DragHandleWidget::new(11, WidgetSizing::fixed(Vector2::new(5.0, 80.0)))
            .with_trailing_rail(1.0);
        let bounds = Rect::from_min_size(Point::default(), Vector2::new(5.0, 80.0));
        let inside = Point::new(2.0, 20.0);
        let outside = Point::new(20.0, 20.0);

        let _ = handle.handle_input(bounds, WidgetInput::primary_press(inside));
        let _ = handle.handle_input(bounds, WidgetInput::pointer_move(outside));
        let _ = handle.handle_input(bounds, WidgetInput::primary_release(outside));
        let _ = handle.handle_input(bounds, WidgetInput::pointer_move(inside));

        assert!(handle.common.state.hovered);
        assert!(!handle.hover_suppressed_until_exit);
    }

    #[test]
    fn hover_delay_restarts_when_focus_loss_cancels_a_pressed_handle() {
        let mut handle = DragHandleWidget::new(12, WidgetSizing::fixed(Vector2::new(5.0, 80.0)))
            .with_hover_chrome_only()
            .with_trailing_rail(1.0);
        let bounds = Rect::from_min_size(Point::default(), Vector2::new(5.0, 80.0));
        let inside = Point::new(2.0, 20.0);
        let theme = ThemeTokens::default();

        let _ = handle.handle_input(bounds, WidgetInput::pointer_move(inside));
        let _ = handle.handle_input(bounds, WidgetInput::primary_press(inside));
        let _ = handle.handle_input(bounds, WidgetInput::FocusChanged(false));

        let immediate = handle.paint_plan(bounds, &LayoutOutput::default(), &theme);
        assert_eq!(
            immediate.fill_rects().next().map(|fill| fill.color),
            Some(theme.border_emphasis)
        );

        let deadline = handle
            .timed_repaint_deadline()
            .expect("focus-loss deadline");
        assert!(handle.advance_timed_repaint(deadline));
        let delayed = handle.paint_plan(bounds, &LayoutOutput::default(), &theme);
        assert_ne!(
            delayed.fill_rects().next().map(|fill| fill.color),
            Some(theme.border_emphasis)
        );
    }
}
