use super::*;
use crate::launcher::{LauncherMessage, LauncherState, launcher_view, update_launcher};
use crate::model::{POPUP_POSITION, POPUP_PREWARM_POSITION, PopupLaunch, PopupMode};
use crate::policy::{popup_policy, popup_policy_for_launch, popup_spec};
use crate::popup::{PopupState, popup_view};
use radiant::{runtime::UiSurface, widgets::TextWidget};

#[test]
fn popup_policy_describes_focused_transient_window() {
    let policy = popup_policy(true);

    assert_eq!(policy.position, Some(POPUP_POSITION));
    assert!(policy.transparent);
    assert!(policy.always_on_top);
    assert!(policy.initially_focused);
    assert!(policy.skip_taskbar);
    assert!(policy.initially_visible);
    assert_eq!(policy.drag_region_height, Some(38.0));
    assert!(!policy.resizable);
}

#[test]
fn popup_policy_can_prepare_rendered_transient_window_offscreen() {
    let policy = popup_policy_for_launch(PopupLaunch {
        mode: PopupMode::Tooltip,
        prewarmed: true,
    });

    assert!(policy.initially_visible);
    assert!(policy.hide_after_first_present);
    assert!(!policy.initially_focused);
    assert_eq!(policy.position, Some(POPUP_PREWARM_POSITION));
    assert!(policy.always_on_top);
    assert!(policy.skip_taskbar);
}

#[test]
fn popup_spec_uses_borderless_popup_window_options() {
    let spec = popup_spec();

    assert!(spec.is_popup());
    assert_eq!(spec.key, "workflow-popup");
    assert_eq!(spec.inner_size(), Some([340.0, 156.0]));
    assert_eq!(
        spec.popup_options().and_then(|popup| popup.position),
        Some(POPUP_POSITION)
    );
    assert!(!spec.native_options().window.behavior.decorations);
    assert!(!spec.drag_and_drop_enabled());
}

#[test]
fn launcher_view_tracks_selected_popup_mode_and_status() {
    let mut state = LauncherState::default();
    state.mark_popups_ready_for_test();
    update_launcher(
        &mut state,
        LauncherMessage::SelectMode(PopupMode::CommandPalette),
        &mut UpdateContext::default(),
    );

    let view = launcher_view(&mut state).into_surface();

    assert_eq!(text(&view, 11).text, "Popup workflow");
    assert_eq!(state.selected_mode(), PopupMode::CommandPalette);
    assert!(state.status().contains("Command palette"));
}

#[test]
fn popup_view_switches_between_modes_and_exposes_close_button() {
    let mut state = PopupState::new_for_test(PopupMode::Tooltip, false);
    let tooltip_view = popup_view(&mut state).into_surface();
    assert_eq!(text(&tooltip_view, 12).text, "Tooltip");
    assert!(tooltip_view.find_widget(18).is_some());

    state.set_mode_for_test(PopupMode::CommandPalette);
    let command_view = popup_view(&mut state).into_surface();
    assert_eq!(text(&command_view, 12).text, "Command palette");
}

fn text<Message>(surface: &UiSurface<Message>, id: u64) -> &TextWidget {
    surface
        .find_widget(id)
        .expect("text widget should exist")
        .widget()
        .as_any()
        .downcast_ref::<TextWidget>()
        .expect("widget should be text")
}
