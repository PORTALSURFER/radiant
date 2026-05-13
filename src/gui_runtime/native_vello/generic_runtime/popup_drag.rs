//! Native movement support for borderless popup windows.

use crate::{
    gui::types::Point,
    gui_runtime::{NativePopupOptions, NativeRunOptions},
    widgets::PointerButton,
};

pub(super) fn should_start_popup_window_drag(
    options: &NativeRunOptions,
    position: Point,
    button: PointerButton,
    routed: bool,
) -> bool {
    if routed || button != PointerButton::Primary {
        return false;
    }
    let Some(popup) = options.popup_options() else {
        return false;
    };
    popup_drag_region_contains(popup, position)
}

fn popup_drag_region_contains(popup: &NativePopupOptions, position: Point) -> bool {
    popup
        .drag_region_height
        .is_some_and(|height| height > 0.0 && position.y >= 0.0 && position.y <= height)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn popup_drag_region_requires_unrouted_primary_press_in_top_region() {
        let options = NativeRunOptions::popup("Popup")
            .popup_policy(NativePopupOptions::default().drag_region_height(42.0));

        assert!(should_start_popup_window_drag(
            &options,
            Point::new(120.0, 24.0),
            PointerButton::Primary,
            false,
        ));
        assert!(!should_start_popup_window_drag(
            &options,
            Point::new(120.0, 64.0),
            PointerButton::Primary,
            false,
        ));
        assert!(!should_start_popup_window_drag(
            &options,
            Point::new(120.0, 24.0),
            PointerButton::Primary,
            true,
        ));
        assert!(!should_start_popup_window_drag(
            &options,
            Point::new(120.0, 24.0),
            PointerButton::Secondary,
            false,
        ));
    }

    #[test]
    fn regular_windows_do_not_start_popup_drag() {
        assert!(!should_start_popup_window_drag(
            &NativeRunOptions::default(),
            Point::new(120.0, 24.0),
            PointerButton::Primary,
            false,
        ));
    }
}
