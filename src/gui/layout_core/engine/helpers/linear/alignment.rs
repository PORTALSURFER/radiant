//! Main-axis alignment helpers for linear containers.

use crate::gui::layout_core::model::MainAlign;

pub(in crate::gui::layout_core::engine) fn align_main_offsets(
    align: MainAlign,
    available_main: f32,
    total_main: f32,
    base_spacing: f32,
    count: usize,
) -> (f32, f32) {
    if count <= 1 {
        let leading = match align {
            MainAlign::Center => (available_main - total_main).max(0.0) * 0.5,
            MainAlign::End => (available_main - total_main).max(0.0),
            _ => 0.0,
        };
        return (leading, 0.0);
    }
    let free = (available_main - total_main).max(0.0);
    match align {
        MainAlign::Start => (0.0, base_spacing),
        MainAlign::Center => (free * 0.5, base_spacing),
        MainAlign::End => (free, base_spacing),
        MainAlign::SpaceBetween => (0.0, base_spacing + free / (count as f32 - 1.0)),
        MainAlign::SpaceAround => {
            let extra = free / count as f32;
            (extra * 0.5, base_spacing + extra)
        }
        MainAlign::SpaceEvenly => {
            let gap = free / (count as f32 + 1.0);
            (gap, base_spacing + gap)
        }
    }
}
