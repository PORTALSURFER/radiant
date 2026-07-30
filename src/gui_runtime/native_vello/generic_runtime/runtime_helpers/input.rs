use crate::{layout::Vector2, theme::DpiScale};
use winit::event::MouseScrollDelta;

pub(in crate::gui_runtime::native_vello) fn scroll_delta_to_logical(
    delta: MouseScrollDelta,
    dpi_scale: DpiScale,
) -> Vector2 {
    match delta {
        MouseScrollDelta::LineDelta(x, y) => Vector2::new(
            -(finite_scroll_component(x) * 40.0),
            -(finite_scroll_component(y) * 40.0),
        ),
        MouseScrollDelta::PixelDelta(position) => Vector2::new(
            -dpi_scale.physical_to_logical(finite_scroll_component(position.x as f32)),
            -dpi_scale.physical_to_logical(finite_scroll_component(position.y as f32)),
        ),
    }
}

fn finite_scroll_component(value: f32) -> f32 {
    if value.is_finite() { value } else { 0.0 }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        gui::types::{Point, Rect},
        widgets::{KnobMessage, KnobWidget, WidgetInput},
    };
    use winit::dpi::PhysicalPosition;

    #[test]
    fn scroll_delta_to_logical_sanitizes_nonfinite_native_values() {
        assert_eq!(
            scroll_delta_to_logical(
                MouseScrollDelta::LineDelta(f32::NAN, 1.0),
                DpiScale::new(2.0)
            ),
            Vector2::new(0.0, -40.0)
        );
        assert_eq!(
            scroll_delta_to_logical(
                MouseScrollDelta::PixelDelta(PhysicalPosition::new(f64::MAX, 12.5)),
                DpiScale::new(2.5)
            ),
            Vector2::new(0.0, -5.0)
        );
    }

    #[test]
    fn negative_native_wheel_raises_knob_after_logical_conversion() {
        let logical_delta =
            scroll_delta_to_logical(MouseScrollDelta::LineDelta(0.0, -1.0), DpiScale::new(1.0));
        assert_eq!(logical_delta, Vector2::new(0.0, 40.0));

        let bounds = Rect::from_min_size(Point::default(), Vector2::new(40.0, 40.0));
        let mut knob = KnobWidget::new(1, 0.5);
        assert!(matches!(
            knob.handle_input(
                bounds,
                WidgetInput::plain_wheel(Point::new(20.0, 20.0), logical_delta),
            ),
            Some(KnobMessage::WheelGesture(batch))
                if batch.events
                    == [
                        crate::widgets::KnobAutomationEvent::GestureStarted { value: 0.5 },
                        crate::widgets::KnobAutomationEvent::ValueChanged { value: 0.55 },
                        crate::widgets::KnobAutomationEvent::GestureEnded { value: 0.55 },
                    ]
        ));
    }
}
