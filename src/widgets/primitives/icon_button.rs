//! Reusable SVG icon button primitive.

use crate::gui::{svg::SvgIcon, types::Rect};
use crate::layout::LayoutOutput;
use crate::runtime::{PaintPrimitive, WidgetMessageMapper, inset_rect};
use crate::theme::ThemeTokens;
use crate::widgets::contract::{FocusBehavior, PaintBounds, Widget, WidgetId, WidgetSizing};
use crate::widgets::interaction::{
    ButtonMessage, PointerButton, WidgetInput, WidgetKey, WidgetOutput,
};
use crate::widgets::primitives::support::{WidgetCommon, push_button_chrome};

/// Public SVG icon button primitive.
#[derive(Clone, Debug)]
pub struct IconButtonWidget {
    /// Shared widget contract.
    pub common: WidgetCommon,
    /// Retained icon painted in the button bounds.
    pub icon: SvgIcon,
}

impl IconButtonWidget {
    /// Build an SVG icon button descriptor.
    pub fn new(id: WidgetId, icon: SvgIcon, sizing: WidgetSizing) -> Self {
        let mut common = WidgetCommon::new(id, sizing);
        common.focus = FocusBehavior::Keyboard;
        common.paint.bounds = PaintBounds::ClipToRect;
        common.paint.paints_focus = false;
        Self { common, icon }
    }
}

impl Widget for IconButtonWidget {
    fn common(&self) -> &WidgetCommon {
        &self.common
    }

    fn common_mut(&mut self) -> &mut WidgetCommon {
        &mut self.common
    }

    fn handle_input(&mut self, bounds: Rect, input: WidgetInput) -> Option<WidgetOutput> {
        if self.common.state.disabled {
            self.common.state.pressed = false;
            return None;
        }
        match input {
            WidgetInput::PointerMove { position } => {
                self.common.state.hovered = bounds.contains(position);
                None
            }
            WidgetInput::PointerPress {
                position,
                button: PointerButton::Primary,
                ..
            } if bounds.contains(position) => {
                self.common.state.hovered = true;
                self.common.state.pressed = true;
                self.common.state.focused = true;
                None
            }
            WidgetInput::PointerRelease {
                position,
                button: PointerButton::Primary,
                ..
            } => {
                let activated = self.common.state.pressed && bounds.contains(position);
                self.common.state.pressed = false;
                self.common.state.hovered = bounds.contains(position);
                activated.then(|| WidgetOutput::typed(ButtonMessage::Activate))
            }
            WidgetInput::FocusChanged(focused) => {
                self.common.state.focused = focused;
                if !focused {
                    self.common.state.pressed = false;
                }
                None
            }
            WidgetInput::KeyPress(WidgetKey::Enter | WidgetKey::Space)
                if self.common.state.focused =>
            {
                Some(WidgetOutput::typed(ButtonMessage::Activate))
            }
            _ => {
                if matches!(input, WidgetInput::PointerRelease { .. }) {
                    self.common.state.pressed = false;
                }
                None
            }
        }
    }

    fn accepts_pointer_move(&self) -> bool {
        true
    }

    fn synchronize_from_previous(&mut self, previous: &dyn Widget) {
        if let Some(previous) = previous.as_any().downcast_ref::<Self>() {
            self.common.state = previous.common.state;
        }
    }

    fn append_paint(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        bounds: Rect,
        _layout: &LayoutOutput,
        theme: &ThemeTokens,
    ) {
        push_button_chrome(primitives, &self.common, bounds, theme);
        let side = bounds.width().min(bounds.height()).min(16.0).max(8.0);
        let rect = inset_rect(
            bounds,
            (bounds.width() - side) * 0.5,
            (bounds.height() - side) * 0.5,
        );
        self.icon.append_paint(primitives, self.common.id, rect);
    }
}

impl<Message> WidgetMessageMapper<Message> {
    /// Build an icon-button mapper.
    pub fn icon_button(map: impl Fn(ButtonMessage) -> Message + Send + Sync + 'static) -> Self {
        Self::typed(map)
    }
}
