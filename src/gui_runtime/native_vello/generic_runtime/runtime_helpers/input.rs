use crate::{
    gui::input::{InputSequenceRange, InputTimestamp},
    layout::Vector2,
    theme::DpiScale,
    widgets::{PointerModifiers, WheelDelta, WheelPhase, WheelSample, WheelSampleError},
};
use winit::event::{MouseScrollDelta, TouchPhase};

pub(in crate::gui_runtime::native_vello) fn native_wheel_sample(
    delta: MouseScrollDelta,
    phase: TouchPhase,
    dpi_scale: DpiScale,
    modifiers: PointerModifiers,
    timestamp: Option<InputTimestamp>,
    sequence_range: Option<InputSequenceRange>,
) -> Result<WheelSample, WheelSampleError> {
    let delta = match delta {
        MouseScrollDelta::LineDelta(x, y) => {
            WheelDelta::lines(Vector2::new(-x, -y)).map_err(WheelSampleError::Delta)?
        }
        MouseScrollDelta::PixelDelta(position) => WheelDelta::pixels(Vector2::new(
            -dpi_scale.physical_to_logical(position.x as f32),
            -dpi_scale.physical_to_logical(position.y as f32),
        ))
        .map_err(WheelSampleError::Delta)?,
    };
    let phase = Some(match phase {
        TouchPhase::Started => WheelPhase::Started,
        TouchPhase::Moved => WheelPhase::Changed,
        TouchPhase::Ended => WheelPhase::Ended,
        TouchPhase::Cancelled => WheelPhase::Cancelled,
    });
    WheelSample::new_with_metadata(delta, phase, modifiers, timestamp, sequence_range)
}

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
        gui::input::{InputSequence, InputSequenceRange, InputTimestamp},
        gui::types::{Point, Rect},
        widgets::{KnobMessage, KnobWidget, WidgetInput},
    };
    use winit::dpi::PhysicalPosition;

    #[test]
    fn native_wheel_sample_preserves_line_units_sign_phase_and_metadata() {
        let modifiers = PointerModifiers {
            command: true,
            shift: true,
            alt: false,
        };
        let timestamp = Some(InputTimestamp::capture());
        let sequence_range = Some(InputSequenceRange::singleton(
            InputSequence::from_runtime_value(7),
        ));

        let sample = native_wheel_sample(
            MouseScrollDelta::LineDelta(1.25, -2.0),
            TouchPhase::Started,
            DpiScale::ONE,
            modifiers,
            timestamp,
            sequence_range,
        )
        .expect("finite line sample");

        assert_eq!(sample.delta(), WheelDelta::Lines(Vector2::new(-1.25, 2.0)));
        assert_eq!(sample.phase(), Some(WheelPhase::Started));
        assert_eq!(sample.modifiers(), modifiers);
        assert_eq!(sample.timestamp(), timestamp);
        assert_eq!(sample.sequence_range(), sequence_range);
    }

    #[test]
    fn native_wheel_sample_preserves_pixel_units_dpi_sign_and_all_phases() {
        let sample = native_wheel_sample(
            MouseScrollDelta::PixelDelta(PhysicalPosition::new(30.0, -60.0)),
            TouchPhase::Moved,
            DpiScale::new(2.0),
            PointerModifiers::default(),
            None,
            None,
        )
        .expect("finite pixel sample");
        assert_eq!(
            sample.delta(),
            WheelDelta::Pixels(Vector2::new(-15.0, 30.0))
        );
        assert_eq!(sample.phase(), Some(WheelPhase::Changed));

        for (phase, expected) in [
            (TouchPhase::Started, WheelPhase::Started),
            (TouchPhase::Moved, WheelPhase::Changed),
            (TouchPhase::Ended, WheelPhase::Ended),
            (TouchPhase::Cancelled, WheelPhase::Cancelled),
        ] {
            assert_eq!(
                native_wheel_sample(
                    MouseScrollDelta::LineDelta(0.0, 1.0),
                    phase,
                    DpiScale::ONE,
                    PointerModifiers::default(),
                    None,
                    None,
                )
                .expect("finite phase sample")
                .phase(),
                Some(expected)
            );
        }
    }

    #[test]
    fn native_wheel_sample_rejects_nonfinite_exact_evidence() {
        assert!(
            native_wheel_sample(
                MouseScrollDelta::LineDelta(f32::NAN, 0.0),
                TouchPhase::Moved,
                DpiScale::ONE,
                PointerModifiers::default(),
                None,
                None,
            )
            .is_err()
        );
        assert!(
            native_wheel_sample(
                MouseScrollDelta::PixelDelta(PhysicalPosition::new(f64::MAX, 0.0)),
                TouchPhase::Moved,
                DpiScale::ONE,
                PointerModifiers::default(),
                None,
                None,
            )
            .is_err()
        );
    }

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
