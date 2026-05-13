//! Context-menu geometry for the folder browser example.

use radiant::{
    gui::{panel::anchored_panel_rect, types::Rect},
    layout::{Point, Vector2},
};

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
    let bounds = Rect::from_min_size(
        Point::new(0.0, 0.0),
        Vector2::new(WINDOW_WIDTH, WINDOW_HEIGHT),
    );
    let rect = anchored_panel_rect(
        bounds,
        position.unwrap_or_else(|| Point::new(0.0, 0.0)),
        Vector2::new(menu_width, menu_height),
        0.0,
    );
    (rect.min.x, rect.min.y)
}
