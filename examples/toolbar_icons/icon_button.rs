use super::*;
use crate::icons::ToolbarIcon;
use crate::model::{ToolId, ToolMessage};

#[derive(Clone, Debug)]
pub(super) struct IconToggleButton {
    common: WidgetCommon,
    tool: ToolId,
    icon: ToolbarIcon,
    pub(super) active: bool,
}

impl IconToggleButton {
    pub(super) fn new(tool: ToolId, icon: ToolbarIcon, active: bool) -> Self {
        let mut common = WidgetCommon::fixed(0, 42.0, 36.0);
        common.focus = FocusBehavior::Keyboard;
        common.state.active = active;
        common.style = WidgetStyle {
            tone: WidgetTone::Neutral,
            ..WidgetStyle::default()
        };
        Self {
            common,
            tool,
            icon,
            active,
        }
    }
}

impl Widget for IconToggleButton {
    fn common(&self) -> &WidgetCommon {
        &self.common
    }

    fn common_mut(&mut self) -> &mut WidgetCommon {
        &mut self.common
    }

    fn handle_input(&mut self, bounds: Rect, input: WidgetInput) -> Option<WidgetOutput> {
        match input {
            WidgetInput::PointerMove { position } => {
                self.common.state.hovered = bounds.contains(position);
                None
            }
            WidgetInput::PointerPress {
                position,
                button: PointerButton::Primary,
                ..
            } => {
                self.common.state.pressed = bounds.contains(position);
                None
            }
            WidgetInput::PointerRelease {
                position,
                button: PointerButton::Primary,
                ..
            } => {
                let should_toggle = self.common.state.pressed && bounds.contains(position);
                self.common.state.pressed = false;
                should_toggle.then(|| WidgetOutput::custom(ToolMessage::Toggle(self.tool)))
            }
            WidgetInput::KeyPress(WidgetKey::Enter) | WidgetInput::KeyPress(WidgetKey::Space)
                if self.common.state.focused =>
            {
                Some(WidgetOutput::custom(ToolMessage::Toggle(self.tool)))
            }
            WidgetInput::FocusChanged(focused) => {
                self.common.state.focused = focused;
                None
            }
            _ => None,
        }
    }

    fn append_paint(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        bounds: Rect,
        _layout: &LayoutOutput,
        theme: &ThemeTokens,
    ) {
        let tokens =
            resolve_widget_visual_tokens(theme, self.common.style, toolbar_state(&self.common));
        primitives.push(PaintPrimitive::FillRect(PaintFillRect {
            widget_id: self.common.id,
            rect: bounds,
            color: tokens.fill,
        }));
        primitives.push(PaintPrimitive::StrokeRect(PaintStrokeRect {
            widget_id: self.common.id,
            rect: bounds,
            color: tokens.border,
            width: 1.0,
        }));
        if self.common.state.focused {
            primitives.push(PaintPrimitive::StrokeRect(PaintStrokeRect {
                widget_id: self.common.id,
                rect: Rect::from_min_max(
                    Point::new(bounds.min.x - 1.0, bounds.min.y - 1.0),
                    Point::new(bounds.max.x + 1.0, bounds.max.y + 1.0),
                ),
                color: tokens.emphasis,
                width: 1.0,
            }));
        }
        if self.active {
            primitives.push(PaintPrimitive::FillRect(PaintFillRect {
                widget_id: self.common.id,
                rect: Rect::from_min_max(
                    Point::new(bounds.min.x + 7.0, bounds.max.y - 4.0),
                    Point::new(bounds.max.x - 7.0, bounds.max.y - 2.0),
                ),
                color: tokens.emphasis,
            }));
        }
        let icon_side = 20.0;
        let icon_rect = Rect::from_min_size(
            Point::new(
                bounds.min.x + (bounds.width() - icon_side) * 0.5,
                bounds.min.y + (bounds.height() - icon_side) * 0.5 - 1.0,
            ),
            Vector2::new(icon_side, icon_side),
        );
        self.icon
            .glyph(self.active)
            .append_paint(primitives, self.common.id, icon_rect);
    }
}

fn toolbar_state(common: &WidgetCommon) -> WidgetState {
    common.state
}
