use super::*;
use crate::runtime::NativePopupOptions;
use crate::runtime::{NativeWindowBehavior, NativeWindowOptions};

#[test]
fn generic_native_window_starts_hidden_during_surface_setup() {
    let attrs = generic_window_attributes(&NativeRunOptions::default());

    assert!(!attrs.visible);
}

#[test]
fn generic_native_window_uses_configured_drag_and_drop_policy() {
    assert!(window::platform_drag_and_drop_enabled(
        &NativeRunOptions::default()
    ));
    assert!(!window::platform_drag_and_drop_enabled(&NativeRunOptions {
        window: NativeWindowOptions {
            behavior: NativeWindowBehavior {
                drag_and_drop: false,
                ..NativeWindowBehavior::default()
            },
            ..NativeWindowOptions::default()
        },
        ..NativeRunOptions::default()
    }));
}

#[test]
fn generic_native_window_reveals_after_surface_setup() {
    let options = NativeRunOptions::default();

    assert!(window::reveal_window_after_surface_setup(&options));
    assert!(!window::reveal_window_after_first_present(&options));
    assert!(!window::hide_window_after_first_present(&options));
}

#[test]
fn generic_native_window_can_remain_hidden_after_surface_setup() {
    let options = NativeRunOptions {
        window: NativeWindowOptions {
            behavior: NativeWindowBehavior {
                reveal_after_surface_setup: false,
                ..NativeWindowBehavior::default()
            },
            ..NativeWindowOptions::default()
        },
        ..NativeRunOptions::default()
    };

    assert!(!window::reveal_window_after_surface_setup(&options));
    assert!(!window::reveal_window_after_first_present(&options));
    assert!(!window::hide_window_after_first_present(&options));
}

#[test]
fn generic_native_window_reveals_popups_after_surface_setup() {
    let options = NativeRunOptions::popup("Drag Preview");

    assert!(window::reveal_window_after_surface_setup(&options));
    assert!(!window::reveal_window_after_first_present(&options));
    assert!(!window::hide_window_after_first_present(&options));
}

#[test]
fn generic_native_window_can_prewarm_hidden_popup_surfaces() {
    let options = NativeRunOptions::popup("Drag Preview")
        .popup_policy(NativePopupOptions::default().initially_visible(false));

    assert!(!window::reveal_window_after_surface_setup(&options));
    assert!(!window::reveal_window_after_first_present(&options));
    assert!(!window::hide_window_after_first_present(&options));
}

#[test]
fn generic_native_window_can_hide_prewarmed_popup_after_first_present() {
    let options = NativeRunOptions::popup("Drag Preview").popup_policy(
        NativePopupOptions::default()
            .position(-20_000.0, -20_000.0)
            .initially_visible(true)
            .hide_after_first_present(true),
    );

    assert!(window::reveal_window_after_surface_setup(&options));
    assert!(!window::reveal_window_after_first_present(&options));
    assert!(window::hide_window_after_first_present(&options));
}

#[test]
fn generic_native_window_applies_floating_popup_policy() {
    let attrs = generic_window_attributes(
        &NativeRunOptions::popup("Drag Preview").popup_policy(
            NativePopupOptions::default()
                .position(64.0, 96.0)
                .initially_focused(true),
        ),
    );

    assert_eq!(attrs.title, "Drag Preview");
    assert!(!attrs.visible);
    assert!(!attrs.decorations);
    assert!(!attrs.resizable);
    assert!(attrs.transparent);
    assert!(attrs.active);
    assert_eq!(attrs.window_level, WindowLevel::AlwaysOnTop);
    assert!(
        matches!(attrs.position, Some(Position::Logical(position)) if position.x == 64.0 && position.y == 96.0)
    );
}
