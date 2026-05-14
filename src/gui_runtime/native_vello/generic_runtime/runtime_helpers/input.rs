use crate::layout::Vector2;
use winit::event::MouseScrollDelta;

pub(in crate::gui_runtime::native_vello) fn scroll_delta_to_logical(
    delta: MouseScrollDelta,
) -> Vector2 {
    match delta {
        MouseScrollDelta::LineDelta(x, y) => Vector2::new(-(x * 40.0), -(y * 40.0)),
        MouseScrollDelta::PixelDelta(position) => {
            Vector2::new(-(position.x as f32), -(position.y as f32))
        }
    }
}
