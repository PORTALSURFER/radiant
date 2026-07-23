//! Widget contract implementation for interactive row primitives.

use super::InteractiveRowWidget;
use crate::{
    gui::types::Rect,
    layout::LayoutOutput,
    runtime::PaintPrimitive,
    theme::ThemeTokens,
    widgets::{
        contract::Widget,
        interaction::{WidgetInput, WidgetOutput},
        primitives::support::{WidgetCommon, push_control_chrome},
    },
};

impl Widget for InteractiveRowWidget {
    fn common(&self) -> &WidgetCommon {
        &self.common
    }

    fn common_mut(&mut self) -> &mut WidgetCommon {
        &mut self.common
    }

    fn handle_input(&mut self, bounds: Rect, input: WidgetInput) -> Option<WidgetOutput> {
        InteractiveRowWidget::handle_input(self, bounds, input).map(WidgetOutput::typed)
    }

    fn accepts_pointer_move(&self) -> bool {
        self.accepts_stable_pointer_move()
    }

    fn synchronize_from_previous(&mut self, previous: &dyn Widget) {
        if let Some(previous) = previous.as_any().downcast_ref::<Self>() {
            self.common.state = previous.common.state;
            if self.props.clear_hover_on_sync {
                self.common.state.hovered = false;
            }
            self.pressed_position = previous.pressed_position;
            self.dragged = previous.dragged;
            self.double_activated = previous.double_activated;
            if retained_drag_ended(previous, self) {
                self.common.state.pressed = false;
                self.pressed_position = None;
                self.dragged = false;
                self.double_activated = false;
            }
        }
    }

    fn append_paint(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        bounds: Rect,
        _layout: &LayoutOutput,
        theme: &ThemeTokens,
    ) {
        if self.common.paint.paints_state_layers {
            push_control_chrome(primitives, &self.common, bounds, theme);
        }
    }
}

fn retained_drag_ended(previous: &InteractiveRowWidget, current: &InteractiveRowWidget) -> bool {
    (previous.props.drag_active && !current.props.drag_active)
        || (previous.props.drag_source && !current.props.drag_source)
}
