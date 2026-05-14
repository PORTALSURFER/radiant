//! Native popup window policies used by the popup example surfaces.

use super::model::{POPUP_POSITION, POPUP_PREWARM_POSITION, PopupLaunch};
use radiant::prelude::*;

pub(super) fn popup_policy(initially_visible: bool) -> NativePopupOptions {
    popup_policy_at(POPUP_POSITION, initially_visible, initially_visible)
}

pub(super) fn popup_policy_for_launch(launch: PopupLaunch) -> NativePopupOptions {
    if launch.prewarmed {
        popup_policy_at(POPUP_PREWARM_POSITION, true, false).hide_after_first_present(true)
    } else {
        popup_policy(true)
    }
}

fn popup_policy_at(
    position: [f32; 2],
    initially_visible: bool,
    initially_focused: bool,
) -> NativePopupOptions {
    NativePopupOptions::default()
        .position(position[0], position[1])
        .transparent(true)
        .always_on_top(true)
        .initially_focused(initially_focused)
        .skip_taskbar(true)
        .initially_visible(initially_visible)
        .drag_region_height(38.0)
}

#[cfg(test)]
pub(super) fn popup_spec() -> WindowSpec {
    WindowSpec::popup("workflow-popup", "Radiant Floating Popup")
        .logical_size(340.0, 156.0)
        .popup_policy(popup_policy(true))
}
