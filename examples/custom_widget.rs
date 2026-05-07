//! User-authored widget object integrated through Radiant's application builder.

use radiant::prelude::*;
use radiant::{
    layout::{LayoutOutput, Rect, Vector2},
    runtime::{PaintFillRect, PaintPrimitive, PaintTextAlign, PaintTextRun},
    theme::ThemeTokens,
    widgets::{FocusBehavior, PointerButton, TextWrap, WidgetCommon, WidgetInput, WidgetSizing},
};

#[derive(Default)]
struct DemoState {
    active: bool,
}

#[derive(Clone, Copy)]
enum ChipOutput {
    Toggle,
}

#[derive(Clone)]
struct StatusChip {
    common: WidgetCommon,
    active: bool,
}

impl StatusChip {
    fn new(active: bool) -> Self {
        let mut common = WidgetCommon::new(0, WidgetSizing::fixed(Vector2::new(140.0, 32.0)));
        common.focus = FocusBehavior::Keyboard;
        Self { common, active }
    }
}

impl Widget for StatusChip {
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
            } if bounds.contains(position) => Some(WidgetOutput::custom(ChipOutput::Toggle)),
            WidgetInput::KeyPress(radiant::widgets::WidgetKey::Enter)
                if self.common.state.focused =>
            {
                Some(WidgetOutput::custom(ChipOutput::Toggle))
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
        let fill = if self.active {
            theme.accent_danger
        } else if self.common.state.hovered {
            theme.surface_raised
        } else {
            theme.surface_base
        };
        primitives.push(PaintPrimitive::FillRect(PaintFillRect {
            widget_id: self.common.id,
            rect: bounds,
            color: fill,
        }));
        primitives.push(PaintPrimitive::Text(PaintTextRun {
            widget_id: self.common.id,
            text: if self.active { "Active" } else { "Idle" }.to_owned(),
            rect: bounds,
            font_size: 14.0,
            baseline: Some(21.0),
            color: theme.text_primary,
            align: PaintTextAlign::Center,
            wrap: TextWrap::None,
        }));
    }
}

fn main() -> radiant::Result {
    radiant::app(DemoState::default())
        .title("Radiant Custom Widget")
        .size(320, 120)
        .view(|state| {
            column([custom_widget(StatusChip::new(state.active), |output| {
                output
                    .custom_ref::<ChipOutput>()
                    .map(|_| ChipOutput::Toggle)
            })
            .id(10)])
            .padding(24.0)
        })
        .update(|state, message| match message {
            ChipOutput::Toggle => state.active = !state.active,
        })
        .run()
}
