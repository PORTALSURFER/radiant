use crate::gui::types::Vector2;
use crate::theme::DpiScale;
use vello::util::RenderSurface;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct RenderSurfacePixelSize {
    width: u32,
    height: u32,
}

impl RenderSurfacePixelSize {
    pub(super) fn from_surface(surface: &RenderSurface<'_>) -> Self {
        Self {
            width: surface.config.width,
            height: surface.config.height,
        }
    }

    pub(super) fn physical_size(self) -> Vector2 {
        Vector2::new(self.width as f32, self.height as f32)
    }

    pub(super) fn logical_size(self, dpi_scale: DpiScale) -> Vector2 {
        Vector2::new(
            dpi_scale.physical_to_logical(self.width as f32),
            dpi_scale.physical_to_logical(self.height as f32),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_surface_pixel_size_projects_logical_target_size() {
        let size = RenderSurfacePixelSize {
            width: 1920,
            height: 1080,
        };

        assert_eq!(size.physical_size(), Vector2::new(1920.0, 1080.0));
        assert_eq!(
            size.logical_size(DpiScale::new(2.0)),
            Vector2::new(960.0, 540.0)
        );
    }
}
