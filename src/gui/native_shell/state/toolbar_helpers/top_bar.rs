//! Top-bar control layout helpers.

use super::super::*;

pub(in crate::gui::native_shell::state) fn top_bar_controls_layout(
    layout: &ShellLayout,
    sizing: SizingTokens,
) -> TopBarControlsLayout {
    let resolved = compute_top_bar_controls_sections(layout, sizing);
    TopBarControlsLayout {
        active: resolved.active,
        options_label: resolved.options_label,
        volume_meter: resolved.volume_meter,
        volume_value: resolved.volume_value,
        volume_label: resolved.volume_label,
    }
}
