//! Widget contract implementation for interactive row primitives.

use super::{InteractiveRowPointerMotion, InteractiveRowWidget};
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
        match self.props.pointer_motion {
            InteractiveRowPointerMotion::Always => true,
            InteractiveRowPointerMotion::DuringInteraction => {
                self.common.state.pressed
                    || self.props.drag_active
                    || self.props.drag_source
                    || self.props.pointer_motion_active
            }
        }
    }

    fn synchronize_from_previous(&mut self, previous: &dyn Widget) {
        if let Some(previous) = previous.as_any().downcast_ref::<Self>() {
            self.common.state = previous.common.state;
            if self.props.clear_hover_on_sync {
                self.common.state.hovered = false;
            }
            self.dragged = previous.dragged;
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
