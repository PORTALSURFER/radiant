use crate::{gui::types::Vector2, theme::DpiScale};
use winit::dpi::PhysicalSize;

pub(super) fn logical_viewport_for_size(size: PhysicalSize<u32>, dpi_scale: DpiScale) -> Vector2 {
    Vector2::new(
        dpi_scale.physical_to_logical(size.width.max(1) as f32),
        dpi_scale.physical_to_logical(size.height.max(1) as f32),
    )
}

pub(super) fn surface_size_changed(
    current_width: u32,
    current_height: u32,
    next: PhysicalSize<u32>,
) -> bool {
    current_width != next.width || current_height != next.height
}

#[cfg(test)]
mod tests {
    use super::{logical_viewport_for_size, surface_size_changed};
    use crate::{layout::Vector2, theme::DpiScale};
    use winit::dpi::PhysicalSize;

    #[test]
    fn native_surface_resize_detects_only_real_physical_size_changes() {
        assert!(!surface_size_changed(640, 480, PhysicalSize::new(640, 480)));
        assert!(surface_size_changed(640, 480, PhysicalSize::new(800, 480)));
        assert!(surface_size_changed(640, 480, PhysicalSize::new(640, 600)));
    }

    #[test]
    fn native_surface_viewport_uses_logical_size_for_current_dpi_scale() {
        assert_eq!(
            logical_viewport_for_size(PhysicalSize::new(1800, 1200), DpiScale::new(1.5)),
            Vector2::new(1200.0, 800.0)
        );
    }
}
