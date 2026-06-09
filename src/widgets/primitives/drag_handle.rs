//! Reusable drag-handle primitive.

use crate::gui::types::Rect;
use crate::layout::LayoutOutput;
use crate::runtime::PaintPrimitive;
use crate::theme::ThemeTokens;

use super::support::WidgetCommon;
use crate::widgets::contract::{
    FocusBehavior, PointerCapturePolicy, Widget, WidgetId, WidgetSizing,
};
use crate::widgets::interaction::{DragHandleMessage, WidgetInput, WidgetOutput};

mod builders;
mod input;
mod paint;

/// Public drag handle primitive for pointer-driven reordering.
#[derive(Clone, Debug, PartialEq)]
pub struct DragHandleWidget {
    /// Shared widget contract.
    pub common: WidgetCommon,
    /// Whether idle handle chrome should be hidden until hover, press, or focus.
    pub hover_chrome_only: bool,
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
        }
    }

    /// Build a compact handle that emits drag lifecycle messages.
    pub fn new(id: WidgetId, sizing: WidgetSizing) -> Self {
        Self::from_parts(DragHandleWidgetParts { id, sizing })
    }

    /// Paint handle chrome only while hovered, pressed, or focused.
    pub fn with_hover_chrome_only(mut self) -> Self {
        self.hover_chrome_only = true;
        self
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
}
