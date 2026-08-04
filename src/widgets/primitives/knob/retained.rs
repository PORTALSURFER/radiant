//! Crate-private retained adapter for Knob's typed edit lifecycle.

use crate::gui::types::Rect;
use crate::layout::LayoutOutput;
use crate::runtime::PaintPrimitive;
use crate::theme::ThemeTokens;
use crate::widgets::contract::{Widget, WidgetCapabilities, WidgetSemantics};
use crate::widgets::interaction::{EditEvent, KnobEditBatch, WidgetInput, WidgetOutput};

use super::{KnobWidget, input};
use crate::widgets::primitives::support::WidgetCommon;

/// Runtime-owned adapter for official Knob projections.
///
/// The public [`KnobWidget`] remains a three-field source-compatible widget.
/// This adapter owns the active typed transaction used by official runtime
/// and application constructors.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct RetainedKnobWidget {
    pub(crate) knob: KnobWidget,
    active_edit: Option<EditEvent<f32>>,
}

impl RetainedKnobWidget {
    pub(crate) fn new(knob: KnobWidget) -> Self {
        Self {
            knob,
            active_edit: None,
        }
    }

    pub(super) fn handle_edit_input(
        &mut self,
        bounds: Rect,
        input: WidgetInput,
    ) -> Option<KnobEditBatch> {
        input::handle_knob_edit_input(&mut self.knob, &mut self.active_edit, bounds, input)
    }
}

impl WidgetSemantics for RetainedKnobWidget {
    fn automation_role(&self) -> crate::gui::automation::AutomationRole {
        crate::gui::automation::AutomationRole::Slider
    }

    fn automation_value_text(&self) -> Option<String> {
        Some(format!("{:.3}", self.knob.state.value))
    }
}

impl Widget for RetainedKnobWidget {
    fn common(&self) -> &WidgetCommon {
        &self.knob.common
    }

    fn common_mut(&mut self) -> &mut WidgetCommon {
        &mut self.knob.common
    }

    fn handle_input(&mut self, bounds: Rect, input: WidgetInput) -> Option<WidgetOutput> {
        self.handle_edit_input(bounds, input)
            .map(WidgetOutput::typed)
    }

    fn handle_pointer_capture_cancelled(&mut self, bounds: Rect) -> Option<WidgetOutput> {
        self.handle_edit_input(bounds, WidgetInput::FocusChanged(false))
            .map(WidgetOutput::typed)
    }

    fn synchronize_from_previous(&mut self, previous: &dyn Widget) {
        let Some(previous) = previous.as_any().downcast_ref::<Self>() else {
            return;
        };

        // The fresh projection remains authoritative for value, props, sizing,
        // style, and semantics. Only runtime-owned interaction state crosses a
        // compatible same-wrapper Knob refresh.
        self.knob.common.state.hovered = previous.knob.common.state.hovered;
        self.knob.common.state.focused = previous.knob.common.state.focused;
        if self.knob.common.state.disabled || self.knob.common.state.read_only {
            self.knob.common.state.pressed = false;
            self.knob.state.fine_adjustment = false;
            self.knob.state.gesture_origin = None;
            self.active_edit = None;
        } else {
            self.knob.common.state.pressed = previous.knob.common.state.pressed;
            self.knob.state.fine_adjustment = previous.knob.state.fine_adjustment;
            self.knob.state.gesture_origin = previous.knob.state.gesture_origin;
            self.active_edit = previous.active_edit;
        }
    }

    fn accepts_pointer_move(&self) -> bool {
        true
    }

    fn accepts_wheel_input(&self) -> bool {
        true
    }

    fn capabilities(&self) -> WidgetCapabilities<'_> {
        WidgetCapabilities::new().semantics(self)
    }

    fn append_paint(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        bounds: Rect,
        layout: &LayoutOutput,
        theme: &ThemeTokens,
    ) {
        self.knob.append_paint(primitives, bounds, layout, theme);
    }
}
