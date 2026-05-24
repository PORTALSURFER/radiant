use super::panel::MixerPanelWidget;
use super::*;
use radiant::prelude::*;
use radiant::runtime::{RuntimeBridge, SurfaceRuntime};
use radiant::widgets::PointerModifiers;

#[path = "tests/model_behavior.rs"]
mod model_behavior;
#[path = "tests/panel_interaction.rs"]
mod panel_interaction;
#[path = "tests/panel_paint.rs"]
mod panel_paint;
#[path = "tests/runtime.rs"]
mod runtime;

fn mixer_widget(state: &MixerState) -> MixerPanelWidget {
    MixerPanelWidget::new(
        state.channels,
        state.selection.clone(),
        state.selected_channel,
        state.frame,
    )
}

fn mixer_test_bridge(state: MixerState) -> impl RuntimeBridge<MixerMessage> {
    radiant::app(state)
        .view(project_surface)
        .animation(|state| state.running)
        .on_frame(|| MixerMessage::Frame)
        .update(update)
        .into_bridge()
}

fn mixer_bounds() -> Rect {
    Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(1400.0, 500.0))
}

fn press_strip_label(
    widget: &mut MixerPanelWidget,
    bounds: Rect,
    channel: usize,
    modifiers: PointerModifiers,
) -> Option<MixerPanelMessage> {
    let strip = widget.strip_rect(bounds, channel);
    let position = Point::new(strip.center().x, strip.min.y + 22.0);
    let output = widget
        .handle_input(
            bounds,
            WidgetInput::PointerPress {
                position,
                button: PointerButton::Primary,
                modifiers,
            },
        )
        .and_then(|output| output.typed_ref::<MixerPanelMessage>().copied());
    let _ = widget.handle_input(
        bounds,
        WidgetInput::PointerRelease {
            position,
            button: PointerButton::Primary,
            modifiers,
        },
    );
    output
}

fn status_text<Bridge>(runtime: &SurfaceRuntime<Bridge, MixerMessage>) -> String
where
    Bridge: RuntimeBridge<MixerMessage>,
{
    runtime
        .paint_plan(&ThemeTokens::default())
        .primitives
        .iter()
        .find_map(|primitive| match primitive {
            PaintPrimitive::Text(text) if text.widget_id == STATUS_WIDGET_ID => {
                Some(text.text.as_str().to_string())
            }
            _ => None,
        })
        .expect("status text should be painted")
}
