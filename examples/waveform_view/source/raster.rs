#[path = "raster/buffer.rs"]
mod buffer;
#[path = "raster/chrome.rs"]
mod chrome;
#[path = "raster/labels.rs"]
mod labels;

use super::{WaveformBand, WaveformFile, WaveformViewport};
use buffer::RasterBuffer;
use radiant::gui::types::ImageRgba;

pub(crate) fn render_waveform_image(
    file: &WaveformFile,
    viewport: WaveformViewport,
    width: usize,
    height: usize,
) -> ImageRgba {
    let mut image = WaveformRaster::new(width, height);
    chrome::draw_raster_chrome(&mut image);
    image.draw_waveform(file, viewport);
    image.into_image()
}

struct WaveformRaster {
    buffer: RasterBuffer,
}

impl WaveformRaster {
    fn new(width: usize, height: usize) -> Self {
        Self {
            buffer: RasterBuffer::new(width, height),
        }
    }

    fn draw_waveform(&mut self, file: &WaveformFile, viewport: WaveformViewport) {
        let viewport = viewport.clamp(file.frames, super::MIN_VISIBLE_FRAMES);
        let visible = viewport.visible_items().max(1);
        let mid = self.height() as f32 * 0.5;
        let half = (self.height() as f32 * 0.42).max(1.0);
        labels::draw_band_labels(self);

        let band_styles = [
            BandStyle {
                fill: [0, 102, 255, 215],
                ridge: [32, 139, 255, 255],
                scale: 1.0,
            },
            BandStyle {
                fill: [154, 91, 38, 198],
                ridge: [205, 132, 60, 240],
                scale: 0.82,
            },
            BandStyle {
                fill: [246, 160, 58, 212],
                ridge: [255, 190, 84, 250],
                scale: 0.72,
            },
            BandStyle {
                fill: [250, 250, 244, 238],
                ridge: [255, 255, 255, 255],
                scale: 0.48,
            },
        ];
        for (band, style) in file.bands.iter().zip(band_styles) {
            self.draw_band(band, viewport, visible, mid, half, style);
        }
        self.draw_mono_ridge(file, viewport, visible, mid, half);
    }

    fn draw_band(
        &mut self,
        band: &WaveformBand,
        viewport: WaveformViewport,
        visible: usize,
        mid: f32,
        half: f32,
        style: BandStyle,
    ) {
        for x in 0..self.width() {
            let start = viewport.start + x * visible / self.width().max(1);
            let end = viewport.start
                + ((x + 1) * visible / self.width().max(1))
                    .max(x * visible / self.width().max(1) + 1);
            let stats = band.stats(start, end.min(viewport.end));
            let peak_extent = stats.peak * half * style.scale;
            let rms_extent = stats.rms.sqrt().clamp(0.0, 1.0) * half * style.scale;
            self.draw_symmetric_column(x, mid, rms_extent, style.fill, 0.28);
            self.draw_symmetric_column(
                x,
                mid,
                peak_extent,
                style.fill,
                band_alpha(stats.peak, style.scale),
            );
            self.stroke_symmetric_extents(x, mid, peak_extent, style.ridge, 0.7);
        }
    }

    fn draw_mono_ridge(
        &mut self,
        file: &WaveformFile,
        viewport: WaveformViewport,
        visible: usize,
        mid: f32,
        half: f32,
    ) {
        for x in 0..self.width() {
            let start = viewport.start + x * visible / self.width().max(1);
            let end = viewport.start
                + ((x + 1) * visible / self.width().max(1))
                    .max(x * visible / self.width().max(1) + 1);
            let stats = file
                .mono_summary
                .stats(&file.mono_samples, start, end.min(viewport.end));
            self.draw_symmetric_column(
                x,
                mid,
                stats.peak * half * 0.36,
                [255, 255, 255, 245],
                0.72,
            );
        }
    }

    fn draw_symmetric_column(
        &mut self,
        x: usize,
        mid: f32,
        extent: f32,
        color: [u8; 4],
        alpha: f32,
    ) {
        let top = (mid - extent).round().max(0.0) as usize;
        let bottom = (mid + extent)
            .round()
            .min(self.height().saturating_sub(1) as f32) as usize;
        for y in top..=bottom {
            self.blend_pixel(
                x,
                y,
                color,
                alpha * column_alpha(y, mid, self.height() as f32 * 0.44),
            );
        }
    }

    fn stroke_symmetric_extents(
        &mut self,
        x: usize,
        mid: f32,
        extent: f32,
        color: [u8; 4],
        alpha: f32,
    ) {
        let top = (mid - extent).round().max(0.0) as usize;
        let bottom = (mid + extent)
            .round()
            .min(self.height().saturating_sub(1) as f32) as usize;
        self.blend_pixel(x, top, color, alpha);
        self.blend_pixel(x, bottom, color, alpha);
    }

    fn into_image(self) -> ImageRgba {
        self.buffer.into_image()
    }

    fn put_pixel(&mut self, x: usize, y: usize, color: [u8; 4]) {
        self.buffer.put_pixel(x, y, color);
    }

    fn blend_pixel(&mut self, x: usize, y: usize, color: [u8; 4], alpha: f32) {
        self.buffer.blend_pixel(x, y, color, alpha);
    }

    fn width(&self) -> usize {
        self.buffer.width()
    }

    fn height(&self) -> usize {
        self.buffer.height()
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct BandStyle {
    fill: [u8; 4],
    ridge: [u8; 4],
    scale: f32,
}

fn column_alpha(y: usize, mid: f32, half: f32) -> f32 {
    let distance = ((y as f32 - mid).abs() / half.max(1.0)).clamp(0.0, 1.0);
    lerp(0.42, 0.92, distance)
}

fn band_alpha(peak: f32, scale: f32) -> f32 {
    (0.34 + peak * 0.72 * scale).clamp(0.28, 0.9)
}

fn lerp(from: f32, to: f32, t: f32) -> f32 {
    from + (to - from) * t.clamp(0.0, 1.0)
}
