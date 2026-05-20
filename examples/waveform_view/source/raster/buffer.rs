//! Pixel buffer helpers for the waveform raster example.

use radiant::gui::types::ImageRgba;

pub(super) struct RasterBuffer {
    width: usize,
    height: usize,
    pixels: Vec<u8>,
}

impl RasterBuffer {
    pub(super) fn new(width: usize, height: usize) -> Self {
        Self {
            width,
            height,
            pixels: vec![0; width.saturating_mul(height).saturating_mul(4)],
        }
    }

    pub(super) fn width(&self) -> usize {
        self.width
    }

    pub(super) fn height(&self) -> usize {
        self.height
    }

    pub(super) fn into_image(self) -> ImageRgba {
        ImageRgba::new(self.width, self.height, self.pixels).expect("valid waveform image")
    }

    pub(super) fn put_pixel(&mut self, x: usize, y: usize, color: [u8; 4]) {
        let Some(index) = self.pixel_index(x, y) else {
            return;
        };
        self.pixels[index..index + 4].copy_from_slice(&color);
    }

    pub(super) fn blend_pixel(&mut self, x: usize, y: usize, color: [u8; 4], alpha: f32) {
        let Some(index) = self.pixel_index(x, y) else {
            return;
        };
        let alpha = (color[3] as f32 / 255.0) * alpha.clamp(0.0, 1.0);
        for (channel, source) in color.iter().take(3).enumerate() {
            let current = self.pixels[index + channel] as f32;
            self.pixels[index + channel] = lerp(current, *source as f32, alpha)
                .round()
                .clamp(0.0, 255.0) as u8;
        }
        self.pixels[index + 3] = 255;
    }

    fn pixel_index(&self, x: usize, y: usize) -> Option<usize> {
        if x >= self.width || y >= self.height {
            return None;
        }
        y.checked_mul(self.width)
            .and_then(|row| row.checked_add(x))
            .and_then(|pixel| pixel.checked_mul(4))
            .filter(|index| index + 3 < self.pixels.len())
    }
}

fn lerp(from: f32, to: f32, t: f32) -> f32 {
    from + (to - from) * t.clamp(0.0, 1.0)
}
