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
}
