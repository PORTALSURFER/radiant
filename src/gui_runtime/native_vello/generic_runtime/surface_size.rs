use super::*;

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

    pub(super) fn logical_size(self) -> Vector2 {
        Vector2::new(self.width as f32, self.height as f32)
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

        assert_eq!(size.logical_size(), Vector2::new(1920.0, 1080.0));
    }
}
