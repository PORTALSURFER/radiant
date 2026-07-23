//! Native movement support for app-owned window chrome.

use crate::{
    gui::types::Point,
    gui_runtime::{NativePopupOptions, NativeRunOptions},
    widgets::PointerButton,
};

pub(super) fn should_start_native_window_drag(
    options: &NativeRunOptions,
    position: Point,
    button: PointerButton,
    routed: bool,
) -> bool {
    if routed || button != PointerButton::Primary {
        return false;
    }
    if let Some(popup) = options.popup_options() {
        return popup_drag_region_contains(popup, position);
    }
    options.window.behavior.integrated_titlebar
        && options
            .window
            .behavior
            .integrated_titlebar_drag_region_height
            .is_some_and(|height| top_drag_region_contains(height, position))
}

pub(super) fn should_toggle_native_window_maximized(
    options: &NativeRunOptions,
    position: Point,
    button: PointerButton,
    routed: bool,
    double_click: bool,
) -> bool {
    double_click
        && !options.is_popup()
        && should_start_native_window_drag(options, position, button, routed)
}

fn popup_drag_region_contains(popup: &NativePopupOptions, position: Point) -> bool {
    popup
        .drag_region_height
        .is_some_and(|height| top_drag_region_contains(height, position))
}

fn top_drag_region_contains(height: f32, position: Point) -> bool {
    height > 0.0 && position.y >= 0.0 && position.y <= height
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn popup_drag_region_requires_unrouted_primary_press_in_top_region() {
        let options = NativeRunOptions::popup("Popup")
            .popup_policy(NativePopupOptions::default().drag_region_height(42.0));

        assert!(should_start_native_window_drag(
            &options,
            Point::new(120.0, 24.0),
            PointerButton::Primary,
            false,
        ));
        assert!(!should_start_native_window_drag(
            &options,
            Point::new(120.0, 64.0),
            PointerButton::Primary,
            false,
        ));
        assert!(!should_start_native_window_drag(
            &options,
            Point::new(120.0, 24.0),
            PointerButton::Primary,
            true,
        ));
        assert!(!should_start_native_window_drag(
            &options,
            Point::new(120.0, 24.0),
            PointerButton::Secondary,
            false,
        ));
    }

    #[test]
    fn regular_windows_do_not_start_popup_drag() {
        assert!(!should_start_native_window_drag(
            &NativeRunOptions::default(),
            Point::new(120.0, 24.0),
            PointerButton::Primary,
            false,
        ));
    }

    #[test]
    fn integrated_titlebar_drag_requires_unrouted_primary_press() {
        let mut options = NativeRunOptions::default();
        options.window.behavior.integrated_titlebar = true;
        options
            .window
            .behavior
            .integrated_titlebar_drag_region_height = Some(38.0);

        assert!(should_start_native_window_drag(
            &options,
            Point::new(120.0, 20.0),
            PointerButton::Primary,
            false,
        ));
        assert!(!should_start_native_window_drag(
            &options,
            Point::new(120.0, 20.0),
            PointerButton::Primary,
            true,
        ));
    }

    #[test]
    fn integrated_titlebar_double_click_toggles_regular_window_maximized_state() {
        let mut options = NativeRunOptions::default();
        options.window.behavior.integrated_titlebar = true;
        options
            .window
            .behavior
            .integrated_titlebar_drag_region_height = Some(38.0);
        let position = Point::new(120.0, 20.0);

        assert!(should_toggle_native_window_maximized(
            &options,
            position,
            PointerButton::Primary,
            false,
            true,
        ));
        assert!(!should_toggle_native_window_maximized(
            &options,
            position,
            PointerButton::Primary,
            false,
            false,
        ));
        assert!(!should_toggle_native_window_maximized(
            &options,
            position,
            PointerButton::Primary,
            true,
            true,
        ));
    }
}
