//! Hit-testing, hover resolution, and pointer-geometry helpers for native shell state.

use super::*;

mod browser;
mod chrome;
mod hover;
mod map;
mod waveform;

pub(in crate::gui::native_shell::state) use self::browser::content_action_hit_test_cache_key;
pub(in crate::gui::native_shell::state) use self::map::{
    map_point_color, map_point_is_focused, map_point_is_selected, spatial_content_id_at_point,
};
pub(in crate::gui::native_shell::state) use self::waveform::{
    hovered_waveform_resize_edge_for_point, waveform_hover_marker_rect, waveform_hover_x_for_point,
    waveform_toolbar_hit_test_cache_key, waveform_toolbar_hover_hint,
};
