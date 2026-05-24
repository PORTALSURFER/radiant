use crate::gui_runtime::NativeRunOptions;
use winit::window::Window;

pub(super) fn centered_position(
    parent_window: Option<&Window>,
    options: &NativeRunOptions,
) -> Option<[f32; 2]> {
    let parent = parent_window?;
    let parent_position = parent.outer_position().ok()?;
    let parent_size = parent.outer_size();
    let scale = parent.scale_factor().max(f64::EPSILON);
    let [child_width, child_height] = options.window.geometry.inner_size.unwrap_or([480.0, 360.0]);
    let child_width = (child_width as f64 * scale).round();
    let child_height = (child_height as f64 * scale).round();
    let x = parent_position.x as f64 + ((parent_size.width as f64 - child_width) / 2.0);
    let y = parent_position.y as f64 + ((parent_size.height as f64 - child_height) / 2.0);
    Some([(x / scale) as f32, (y / scale) as f32])
}
