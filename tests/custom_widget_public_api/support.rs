use radiant::{
    gui::types::Rgba8,
    layout::Rect,
    runtime::{PaintPrimitive, SurfacePaintPlan},
    theme::ThemeTokens,
    widgets::{
        PointerButton, Widget, WidgetCommon, WidgetInput, WidgetKey, WidgetOutput, WidgetSizing,
    },
};

#[derive(Clone, Debug, PartialEq)]
pub(super) enum DemoMessage {
    Rename(String),
    SetActive(bool),
}

#[derive(Default)]
pub(super) struct DemoState {
    pub(super) name: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum CustomWidgetMessage {
    Activated,
}

#[derive(Clone)]
pub(super) struct CustomStatusWidget {
    pub(super) common: WidgetCommon,
    label: &'static str,
    pub(super) activation_count: usize,
}

impl CustomStatusWidget {
    pub(super) fn new(id: u64) -> Self {
        let mut common = WidgetCommon::new(
            id,
            WidgetSizing::fixed(radiant::layout::Vector2::new(120.0, 28.0)),
        );
        common.focus = radiant::widgets::FocusBehavior::Keyboard;
        Self {
            common,
            label: "custom",
            activation_count: 0,
        }
    }
}

impl Widget for CustomStatusWidget {
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
            WidgetInput::PointerRelease {
                position,
                button: PointerButton::Primary,
                ..
            } if bounds.contains(position) => {
                self.activation_count += 1;
                Some(WidgetOutput::custom(CustomWidgetMessage::Activated))
            }
            WidgetInput::KeyPress(WidgetKey::Enter) if self.common.state.focused => {
                self.activation_count += 1;
                Some(WidgetOutput::custom(CustomWidgetMessage::Activated))
            }
            WidgetInput::FocusChanged(focused) => {
                self.common.state.focused = focused;
                None
            }
            _ => None,
        }
    }

    fn synchronize_from_previous(&mut self, previous: &dyn Widget) {
        if let Some(previous) = previous.as_any().downcast_ref::<CustomStatusWidget>() {
            self.activation_count = previous.activation_count;
        }
    }

    fn append_paint(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        bounds: Rect,
        _layout: &radiant::layout::LayoutOutput,
        theme: &ThemeTokens,
    ) {
        primitives.push(PaintPrimitive::FillRect(radiant::runtime::PaintFillRect {
            widget_id: self.common.id,
            rect: bounds,
            color: if self.common.state.hovered {
                theme.accent_danger
            } else {
                theme.surface_base
            },
        }));
        primitives.push(PaintPrimitive::Text(radiant::runtime::PaintTextRun {
            widget_id: self.common.id,
            text: self.label.into(),
            rect: bounds,
            font_size: 13.0,
            baseline: Some(18.0),
            color: theme.text_primary,
            align: radiant::runtime::PaintTextAlign::Center,
            wrap: radiant::widgets::TextWrap::None,
        }));
    }
}

pub(super) fn widget_fill_color(plan: &SurfacePaintPlan, widget_id: u64) -> Option<Rgba8> {
    plan.primitives
        .iter()
        .find_map(|primitive| match primitive {
            PaintPrimitive::FillRect(fill) if fill.widget_id == widget_id => Some(fill.color),
            _ => None,
        })
}
