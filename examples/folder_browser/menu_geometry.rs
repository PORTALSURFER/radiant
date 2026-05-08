//! Context-menu geometry for the folder browser example.

use radiant::layout::Point;

const WINDOW_WIDTH: f32 = 900.0;
const WINDOW_HEIGHT: f32 = 540.0;

pub(super) const FOLDER_MENU_WIDTH: f32 = 190.0;
pub(super) const FOLDER_MENU_HEIGHT: f32 = 126.0;
pub(super) const FILE_MENU_WIDTH: f32 = 190.0;
pub(super) const FILE_MENU_HEIGHT: f32 = 126.0;
pub(super) const COLUMN_MENU_WIDTH: f32 = 210.0;
pub(super) const COLUMN_MENU_HEIGHT: f32 = 250.0;

pub(super) fn anchored_context_menu_position(
    position: Option<Point>,
    menu_width: f32,
    menu_height: f32,
) -> (f32, f32) {
    let position = position.unwrap_or_else(|| Point::new(0.0, 0.0));
    let max_left = (WINDOW_WIDTH - menu_width).max(0.0);
    let left = position.x.clamp(0.0, max_left);
    let top = if position.y + menu_height <= WINDOW_HEIGHT {
        position.y.max(0.0)
    } else {
        (position.y - menu_height).max(0.0)
    };
    (left, top)
}
