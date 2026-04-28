//! Hit-testing, hover resolution, and pointer-geometry helpers for native shell state.

use super::*;

#[path = "../../../../../../../src/app_core/native_shell/composition/state/hit_testing/browser.rs"]
mod browser;
#[path = "../../../../../../../src/app_core/native_shell/composition/state/hit_testing/chrome.rs"]
mod chrome;
mod hover;
#[path = "../../../../../../../src/app_core/native_shell/composition/state/hit_testing/map.rs"]
mod map;
#[path = "../../../../../../../src/app_core/native_shell/composition/state/hit_testing/waveform.rs"]
mod waveform;

pub(in crate::gui::native_shell::state) use self::browser::browser_action_hit_test_cache_key;
pub(in crate::gui::native_shell::state) use self::map::{
    map_point_color, map_point_is_focused, map_point_is_selected, map_sample_id_at_point,
};
pub(in crate::gui::native_shell::state) use self::waveform::{
    hovered_waveform_resize_edge_for_point, waveform_hover_marker_rect, waveform_hover_x_for_point,
    waveform_toolbar_hit_test_cache_key, waveform_toolbar_hover_hint,
};

#[cfg(test)]
pub(in crate::gui::native_shell::state) use self::browser::browser_action_model_signature;

#[cfg(test)]
pub(in crate::gui::native_shell::state) use self::waveform::waveform_toolbar_model_flags;
