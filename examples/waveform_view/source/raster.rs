#[path = "raster/buffer.rs"]
mod buffer;
#[path = "raster/chrome.rs"]
mod chrome;
#[path = "raster/labels.rs"]
mod labels;
#[path = "raster/waveform.rs"]
mod waveform;

use super::{WaveformFile, WaveformViewport};
use buffer::RasterBuffer;
use radiant::gui::types::ImageRgba;

pub(crate) fn render_waveform_image(
    file: &WaveformFile,
    viewport: WaveformViewport,
    width: usize,
    height: usize,
) -> ImageRgba {
    let mut image = WaveformRaster::new(RasterSize { width, height });
    chrome::draw_raster_chrome(&mut image);
    image.draw_waveform(file, viewport);
    image.into_image()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct RasterSize {
    width: usize,
    height: usize,
}

struct WaveformRaster {
    buffer: RasterBuffer,
}

impl WaveformRaster {
    fn new(size: RasterSize) -> Self {
        Self {
            buffer: RasterBuffer::new(size.width, size.height),
        }
    }

    fn into_image(self) -> ImageRgba {
        self.buffer.into_image()
    }

    fn put_pixel(&mut self, x: usize, y: usize, color: [u8; 4]) {
        self.buffer.put_pixel(x, y, color);
    }

    fn blend_pixel(&mut self, paint: PixelPaint) {
        self.buffer.blend_pixel(paint);
    }

    fn width(&self) -> usize {
        self.buffer.width()
    }

    fn height(&self) -> usize {
        self.buffer.height()
    }

    pub(super) fn size(&self) -> RasterSize {
        RasterSize {
            width: self.width(),
            height: self.height(),
        }
    }

    pub(super) fn mid(&self) -> f32 {
        self.height() as f32 * 0.5
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct PixelPaint {
    pub(super) x: usize,
    pub(super) y: usize,
    pub(super) color: [u8; 4],
    pub(super) alpha: f32,
}

fn column_alpha(y: usize, mid: f32, half: f32) -> f32 {
    let distance = ((y as f32 - mid).abs() / half.max(1.0)).clamp(0.0, 1.0);
    lerp(0.42, 0.92, distance)
}

fn lerp(from: f32, to: f32, t: f32) -> f32 {
    from + (to - from) * t.clamp(0.0, 1.0)
}
