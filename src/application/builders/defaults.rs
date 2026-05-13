use crate::{layout::Vector2, widgets::WidgetSizing};

pub(in crate::application) fn default_text_sizing() -> WidgetSizing {
    WidgetSizing::fixed(Vector2::new(160.0, 24.0)).with_baseline(17.0)
}

pub(in crate::application) fn default_button_sizing(label: &str) -> WidgetSizing {
    let width = (label.chars().count() as f32 * 9.0 + 36.0).clamp(88.0, 260.0);
    WidgetSizing::fixed(Vector2::new(width, 36.0)).with_baseline(23.0)
}

pub(in crate::application) fn default_drag_handle_sizing() -> WidgetSizing {
    WidgetSizing::fixed(Vector2::new(24.0, 24.0))
}

pub(in crate::application) fn default_badge_sizing(label: &str) -> WidgetSizing {
    let width = (label.chars().count() as f32 * 8.0 + 24.0).clamp(56.0, 180.0);
    WidgetSizing::fixed(Vector2::new(width, 24.0)).with_baseline(17.0)
}

pub(in crate::application) fn default_selectable_sizing(label: &str) -> WidgetSizing {
    let width = (label.chars().count() as f32 * 8.0 + 28.0).clamp(92.0, 260.0);
    WidgetSizing::fixed(Vector2::new(width, 30.0)).with_baseline(20.0)
}

pub(in crate::application) fn default_toggle_sizing(label: &str, compact: bool) -> WidgetSizing {
    if compact {
        return WidgetSizing::fixed(Vector2::new(22.0, 22.0)).with_baseline(16.0);
    }
    let width = (label.chars().count() as f32 * 8.0 + 52.0).clamp(96.0, 280.0);
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
