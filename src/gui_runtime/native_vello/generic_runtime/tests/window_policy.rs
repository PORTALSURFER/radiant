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

#[test]
fn normal_window_delays_activation_until_surface_reveal() {
    let options = NativeRunOptions::default();

    let policy = activation::StartupActivationPolicy::for_options(&options);
    assert_eq!(
        policy,
        activation::StartupActivationPolicy::DelayedNormalWindow
    );
    assert!(!policy.activate_ignoring_other_apps_at_launch());
    let mut controller = activation::ActivationRevealController::with_launch_foreground_process(
        activation::StartupActivationPolicy::DelayedNormalWindow,
        Some(41),
    );
    assert_eq!(
        controller.surface_ready(false, Some(41), std::time::Instant::now()),
        activation::SurfaceReadyActivationAction::RequestActivation
    );
}

#[test]
fn active_normal_window_reveals_without_redundant_activation() {
    let mut controller = activation::ActivationRevealController::with_launch_foreground_process(
        activation::StartupActivationPolicy::DelayedNormalWindow,
        Some(41),
    );

    assert_eq!(
        controller.surface_ready(true, Some(41), std::time::Instant::now()),
        activation::SurfaceReadyActivationAction::RevealActiveApplication
    );
}

#[test]
fn requested_activation_waits_for_active_confirmation() {
    let mut controller = activation::ActivationRevealController::with_launch_foreground_process(
        activation::StartupActivationPolicy::DelayedNormalWindow,
        Some(41),
    );

    assert_eq!(
        controller.surface_ready(false, Some(41), std::time::Instant::now()),
        activation::SurfaceReadyActivationAction::RequestActivation
    );
    assert!(!controller.observe_application_active(false));
    assert!(controller.observe_application_active(true));
}

#[test]
fn pending_activation_is_canceled_after_foreground_switch() {
    let now = std::time::Instant::now();
    let mut controller = activation::ActivationRevealController::with_launch_foreground_process(
        activation::StartupActivationPolicy::DelayedNormalWindow,
        Some(41),
    );

    assert_eq!(
        controller.surface_ready(false, Some(41), now),
        activation::SurfaceReadyActivationAction::RequestActivation
    );
    assert_eq!(
        controller.activation_poll(now + std::time::Duration::from_millis(100), Some(73)),
        activation::ActivationPoll::ForegroundChanged
    );
}

#[test]
fn foreground_transition_to_application_keeps_activation_pending() {
    let now = std::time::Instant::now();
    let mut controller = activation::ActivationRevealController::with_launch_foreground_process(
        activation::StartupActivationPolicy::DelayedNormalWindow,
        Some(41),
    );

    assert_eq!(
        controller.surface_ready(false, Some(41), now),
        activation::SurfaceReadyActivationAction::RequestActivation
    );
    assert!(matches!(
        controller.activation_poll(now + std::time::Duration::from_millis(100), Some(7)),
        activation::ActivationPoll::WaitUntil(_)
    ));
}

#[test]
fn hidden_normal_window_never_selects_delayed_activation() {
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

    let policy = activation::StartupActivationPolicy::for_options(&options);
    assert_eq!(policy, activation::StartupActivationPolicy::Passive);
    assert!(!policy.activate_ignoring_other_apps_at_launch());
    assert!(!window::reveal_window_after_surface_setup(&options));
}

#[test]
fn foreground_switch_during_startup_defers_activation_until_user_returns() {
    let mut controller = activation::ActivationRevealController::with_launch_foreground_process(
        activation::StartupActivationPolicy::DelayedNormalWindow,
        Some(41),
    );

    assert_eq!(
        controller.surface_ready(false, Some(73), std::time::Instant::now()),
        activation::SurfaceReadyActivationAction::AwaitExternalActivation
    );
    assert!(!controller.observe_application_active(true));
    assert!(controller.observe_user_reopen(true));
}

#[test]
fn late_activation_after_foreground_switch_stays_fenced() {
    let now = std::time::Instant::now();
    let mut controller = activation::ActivationRevealController::with_launch_foreground_process(
        activation::StartupActivationPolicy::DelayedNormalWindow,
        Some(41),
    );

    assert_eq!(
        controller.surface_ready(false, Some(41), now),
        activation::SurfaceReadyActivationAction::RequestActivation
    );
    assert_eq!(
        controller.activation_poll(now + std::time::Duration::from_millis(100), Some(73)),
        activation::ActivationPoll::ForegroundChanged
    );
    assert!(!controller.observe_application_active(true));
    assert!(controller.observe_user_reopen(true));
}

#[test]
fn timed_out_activation_requires_explicit_reopen_intent() {
    let now = std::time::Instant::now();
    let mut controller = activation::ActivationRevealController::with_launch_foreground_process(
        activation::StartupActivationPolicy::DelayedNormalWindow,
        Some(41),
    );

    assert_eq!(
        controller.surface_ready(false, Some(41), now),
        activation::SurfaceReadyActivationAction::RequestActivation
    );
    assert_eq!(
        controller.activation_poll(now + std::time::Duration::from_secs(2), Some(41)),
        activation::ActivationPoll::TimedOut
    );
    assert!(!controller.observe_application_active(true));
    assert!(!controller.observe_user_reopen(false));
    assert!(controller.observe_application_active(true));
}

#[test]
fn nonfocused_and_prewarmed_popups_remain_passive() {
    for options in [
        NativeRunOptions::popup("Popup"),
        NativeRunOptions::prewarmed_popup("Prewarmed", -20_000.0, -20_000.0),
    ] {
        assert_eq!(
            activation::StartupActivationPolicy::for_options(&options),
            activation::StartupActivationPolicy::Passive
        );
    }
}

#[test]
fn focused_popup_activation_is_explicit() {
    let options = NativeRunOptions::popup("Focused")
        .popup_policy(NativePopupOptions::default().initially_focused(true));

    let policy = activation::StartupActivationPolicy::for_options(&options);
    assert_eq!(
        policy,
        activation::StartupActivationPolicy::EagerFocusedPopup
    );
    assert!(policy.activate_ignoring_other_apps_at_launch());
}
