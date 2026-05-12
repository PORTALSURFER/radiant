//! User-authored widget object integrated through Radiant's application builder.

use radiant::prelude::*;

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
            WidgetInput::KeyPress(WidgetKey::Enter) if self.common.state.focused => {
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
            text: if self.active { "Active" } else { "Idle" }.into(),
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
            column([
                custom_widget_mapped(StatusChip::new(state.active), |message: ChipOutput| message)
                    .id(10),
            ])
            .padding(24.0)
        })
        .update(|state, message| match message {
            ChipOutput::Toggle => state.active = !state.active,
        })
        .run()
}

#[cfg(test)]
mod tests {
    use super::*;
    use radiant::runtime::SurfaceRuntime;

    #[test]
    fn custom_widget_routes_typed_output_through_application_runtime() {
        let bridge = radiant::app(DemoState::default())
            .view(|state| {
                column([custom_widget_mapped(
                    StatusChip::new(state.active),
                    |message: ChipOutput| message,
                )
                .id(10)])
            })
            .update(|state, message| match message {
                ChipOutput::Toggle => state.active = !state.active,
            })
            .into_bridge();
        let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(320.0, 120.0));

        let handled = runtime.dispatch_input(
            10,
            WidgetInput::PointerRelease {
                position: Point::new(20.0, 20.0),
                button: PointerButton::Primary,
            },
        );
        let active = runtime
            .surface()
            .find_widget(10)
            .and_then(|widget| widget.widget_object().as_any().downcast_ref::<StatusChip>())
            .map(|chip| chip.active);

        assert!(handled);
        assert_eq!(active, Some(true));
    }
}
