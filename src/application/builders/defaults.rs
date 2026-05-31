use crate::{
    gui::text_layout::{TextWidthEstimate, estimated_text_width_in_range},
    layout::Vector2,
    widgets::WidgetSizing,
};

pub(in crate::application) fn default_text_sizing() -> WidgetSizing {
    WidgetSizing::fixed(Vector2::new(160.0, 24.0)).with_baseline(17.0)
}

pub(in crate::application) fn default_button_sizing(label: &str) -> WidgetSizing {
    let width = default_text_width(label, 9.0, 36.0, 88.0, 260.0);
    WidgetSizing::fixed(Vector2::new(width, 36.0)).with_baseline(23.0)
}

pub(in crate::application) fn default_drag_handle_sizing() -> WidgetSizing {
    WidgetSizing::fixed(Vector2::new(24.0, 24.0))
}

pub(in crate::application) fn default_badge_sizing(label: &str) -> WidgetSizing {
    let width = default_text_width(label, 8.0, 24.0, 56.0, 180.0);
    WidgetSizing::fixed(Vector2::new(width, 24.0)).with_baseline(17.0)
}

pub(in crate::application) fn default_selectable_sizing(label: &str) -> WidgetSizing {
    let width = default_text_width(label, 8.0, 28.0, 92.0, 260.0);
    WidgetSizing::fixed(Vector2::new(width, 30.0)).with_baseline(20.0)
}

pub(in crate::application) fn default_toggle_sizing(label: &str, compact: bool) -> WidgetSizing {
    if compact {
        return WidgetSizing::fixed(Vector2::new(22.0, 22.0)).with_baseline(16.0);
    }
    let width = default_text_width(label, 8.0, 52.0, 96.0, 280.0);
    WidgetSizing::fixed(Vector2::new(width, 30.0))
}

pub(in crate::application) fn default_text_input_sizing() -> WidgetSizing {
    WidgetSizing::new(Vector2::new(180.0, 42.0), Vector2::new(280.0, 42.0)).with_baseline(26.0)
}

pub(in crate::application) fn default_slider_sizing() -> WidgetSizing {
    WidgetSizing::new(Vector2::new(160.0, 28.0), Vector2::new(240.0, 28.0))
}

pub(in crate::application) fn default_canvas_sizing() -> WidgetSizing {
    WidgetSizing::fixed(Vector2::new(1.0, 1.0))
}

pub(in crate::application) fn default_card_sizing() -> WidgetSizing {
    WidgetSizing::new(Vector2::new(120.0, 72.0), Vector2::new(220.0, 120.0))
}

pub(in crate::application) fn default_gpu_surface_sizing() -> WidgetSizing {
    WidgetSizing::new(Vector2::new(160.0, 90.0), Vector2::new(320.0, 180.0))
}

fn default_text_width(
    label: &str,
    character_advance: f32,
    horizontal_padding: f32,
    min_width: f32,
    max_width: f32,
) -> f32 {
    estimated_text_width_in_range(
        label,
        TextWidthEstimate::new(character_advance, horizontal_padding),
        min_width,
        max_width,
    )
}
