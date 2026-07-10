mod actions;
mod overlays;
mod projection;

use crate::{
    gui::types::{Point, Rect},
    layout::Vector2,
    runtime::{Event, PaintPrimitive, RuntimeBridge, SurfaceRuntime},
    widgets::{PointerButton, PointerModifiers},
};

#[derive(Clone, Debug, PartialEq)]
enum MenuMessage {
    Open,
    Delete,
    Close,
}

fn click<Bridge>(runtime: &mut SurfaceRuntime<Bridge, MenuMessage>, position: Point)
where
    Bridge: RuntimeBridge<MenuMessage>,
{
    runtime.dispatch_event(Event::PointerPress {
        position,
        button: PointerButton::Primary,
        modifiers: PointerModifiers::default(),
    });
    runtime.dispatch_event(Event::PointerRelease {
        position,
        button: PointerButton::Primary,
        modifiers: PointerModifiers::default(),
    });
}

fn painted_menu_rect(primitives: &[PaintPrimitive], size: Vector2) -> Rect {
    primitives
        .iter()
        .filter_map(|primitive| match primitive {
            PaintPrimitive::FillRect(fill) => Some(fill.rect),
            _ => None,
        })
        .find(|rect| (rect.width() - size.x).abs() < 0.01 && (rect.height() - size.y).abs() < 0.01)
        .expect("painted compact menu rect")
}
