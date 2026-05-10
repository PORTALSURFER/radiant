//! Shared helpers for box-like layout and measurement strategies.

use crate::gui::layout_core::tree::ContainerNode;

pub(in crate::gui::layout_core::engine) fn select_switch_child(
    container: &ContainerNode,
    width: f32,
) -> Option<usize> {
    if container.children.is_empty() {
        return None;
    }
    if container.policy.switch_breakpoints.is_empty() {
        return Some(0);
    }

    for (index, breakpoint) in container.policy.switch_breakpoints.iter().enumerate() {
        if breakpoint.contains(width) && index < container.children.len() {
            return Some(index);
        }
    }
    Some(0)
}

pub(in crate::gui::layout_core::engine) fn fit_aspect_box(
    max_w: f32,
    max_h: f32,
    ratio: f32,
) -> (f32, f32) {
    if max_w <= 0.0 || max_h <= 0.0 {
        return (0.0, 0.0);
    }
    let by_width_h = max_w / ratio;
    if by_width_h <= max_h {
        return (max_w, by_width_h.max(0.0));
    }
    let by_height_w = max_h * ratio;
    (by_height_w.max(0.0), max_h)
}
