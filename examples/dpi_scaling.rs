//! DPI scaling sandbox for Radiant's logical-point runtime boundary.

use radiant::prelude::*;
use radiant::theme::DpiScale;

const BASE_WINDOW_WIDTH: f32 = 760.0;
const BASE_WINDOW_HEIGHT: f32 = 520.0;

fn main() -> radiant::Result {
    println!(
        "radiant_dpi_scaling logical_100_at_150_percent={:.1} physical_pixels",
        DpiScale::new(1.5).logical_to_physical(100.0)
    );

    radiant::app(DpiScalingState::default())
        .title("Radiant DPI Scaling")
        .size(BASE_WINDOW_WIDTH as u32, BASE_WINDOW_HEIGHT as u32)
        .min_size(620, 420)
        .view(project_surface)
        .on_startup(|state, context| {
            apply_scale(context, state.selected);
        })
        .handle_message(|state, message, context| match message {
            DpiScalingMessage::Select(scale) => {
                state.selected = scale;
                apply_scale(context, scale);
            }
        })
        .run()
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct DpiScalingState {
    selected: DpiScaleChoice,
}

impl Default for DpiScalingState {
    fn default() -> Self {
        Self {
            selected: DpiScaleChoice::One,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DpiScalingMessage {
    Select(DpiScaleChoice),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DpiScaleChoice {
    Half,
    One,
    OneAndHalf,
    Two,
}

impl DpiScaleChoice {
    fn label(self) -> &'static str {
        match self {
            Self::Half => "50%",
            Self::One => "100%",
            Self::OneAndHalf => "150%",
            Self::Two => "200%",
        }
    }

    fn scale(self) -> DpiScale {
        match self {
            Self::Half => DpiScale::new(0.5),
            Self::One => DpiScale::new(1.0),
            Self::OneAndHalf => DpiScale::new(1.5),
            Self::Two => DpiScale::new(2.0),
        }
    }
}

fn apply_scale<Message>(context: &mut UiUpdateContext<Message>, choice: DpiScaleChoice) {
    context.set_dpi_scale(choice.scale());
    context.set_window_logical_size(Vector2::new(BASE_WINDOW_WIDTH, BASE_WINDOW_HEIGHT));
}

fn project_surface(state: &mut DpiScalingState) -> View<DpiScalingMessage> {
    let scale = state.selected.scale();
    column([
        text(format!(
            "Selected scale: {} ({:.1}x)",
            state.selected.label(),
            scale.factor()
        ))
        .height(34.0)
        .fill_width(),
        row([
            scale_button(DpiScaleChoice::Half, state.selected),
            scale_button(DpiScaleChoice::One, state.selected),
            scale_button(DpiScaleChoice::OneAndHalf, state.selected),
            scale_button(DpiScaleChoice::Two, state.selected),
        ])
        .spacing(8.0)
        .fill_width(),
        row([
            metric_tile("Logical app width", "680 pt"),
            metric_tile(
                "Physical framebuffer",
                format!("{:.0} px", scale.logical_to_physical(680.0)),
            ),
            metric_tile(
                "Pointer remap",
                format!(
                    "{:.0} px -> {:.0} pt",
                    scale.logical_to_physical(120.0),
                    scale.physical_to_logical(scale.logical_to_physical(120.0))
                ),
            ),
        ])
        .spacing(10.0)
        .fill_width(),
        column([
            text("This whole window is rendered with the selected DPI override.").fill_width(),
            text("The example also resizes the native window so each scale keeps the same logical workspace visible.").fill_width(),
            row([
                button("Switch to 200%")
                    .style(WidgetStyle {
                        tone: WidgetTone::Success,
                        prominence: WidgetProminence::Strong,
                    })
                    .message(DpiScalingMessage::Select(DpiScaleChoice::Two))
                    .height(34.0)
                    .fill_width(),
                text("Normal label")
                    .style(WidgetStyle {
                        tone: WidgetTone::Accent,
                        prominence: WidgetProminence::Strong,
                    })
                    .height(30.0)
                    .fill_width(),
            ])
            .spacing(10.0)
            .fill_width(),
        ])
        .style(WidgetStyle {
            tone: WidgetTone::Neutral,
            prominence: WidgetProminence::Subtle,
        })
        .padding(12.0)
        .spacing(10.0)
        .fill_width(),
        column([
            text("OS DPI changes still work automatically unless this example overrides the scale.")
                .fill_width(),
        ])
        .style(WidgetStyle {
            tone: WidgetTone::Accent,
            prominence: WidgetProminence::Subtle,
        })
        .padding(12.0)
        .spacing(6.0)
        .fill_width(),
    ])
    .padding(18.0)
    .spacing(12.0)
}

fn scale_button(choice: DpiScaleChoice, selected: DpiScaleChoice) -> View<DpiScalingMessage> {
    let label = if choice == selected {
        format!("{} active", choice.label())
    } else {
        choice.label().to_string()
    };
    button(label)
        .style(WidgetStyle {
            tone: WidgetTone::Accent,
            prominence: if choice == selected {
                WidgetProminence::Strong
            } else {
                WidgetProminence::Normal
            },
        })
        .message(DpiScalingMessage::Select(choice))
        .height(36.0)
        .fill_width()
}

fn metric_tile(label: impl Into<String>, value: impl Into<String>) -> View<DpiScalingMessage> {
    column([
        text(label.into()).height(22.0).fill_width(),
        text(value.into())
            .style(WidgetStyle {
                tone: WidgetTone::Success,
                prominence: WidgetProminence::Strong,
            })
            .height(28.0)
            .fill_width(),
    ])
    .style(WidgetStyle {
        tone: WidgetTone::Neutral,
        prominence: WidgetProminence::Normal,
    })
    .padding(10.0)
    .spacing(8.0)
    .height(96.0)
    .fill_width()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dpi_scaling_example_projects_metrics_for_selected_scale() {
        let mut state = DpiScalingState {
            selected: DpiScaleChoice::Two,
        };

        let surface = project_surface(&mut state).into_surface();

        let _layout = surface.layout_node();
        assert_eq!(
            DpiScaleChoice::Two.scale().logical_to_physical(680.0),
            1360.0
        );
        assert_eq!(
            DpiScaleChoice::Half.scale().logical_to_physical(680.0),
            340.0
        );
    }
}
