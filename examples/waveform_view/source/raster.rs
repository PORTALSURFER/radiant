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

    fn draw_waveform(&mut self, file: &WaveformFile, viewport: WaveformViewport) {
        let viewport = viewport.clamp(file.frames, super::MIN_VISIBLE_FRAMES);
        let geometry = WaveformGeometry::new(viewport, self.size());
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
            self.draw_band(band, geometry, style);
        }
        self.draw_mono_ridge(file, geometry);
    }

    fn draw_band(&mut self, band: &WaveformBand, geometry: WaveformGeometry, style: BandStyle) {
        for x in 0..self.width() {
            let column = geometry.column_range(x);
            let stats = band.stats(column.start, column.end);
            let peak_extent = stats.peak * geometry.half * style.scale;
            let rms_extent = stats.rms.sqrt().clamp(0.0, 1.0) * geometry.half * style.scale;
            self.draw_symmetric_column(ColumnPaint {
                x,
                extent: rms_extent,
                color: style.fill,
                alpha: 0.28,
            });
            self.draw_symmetric_column(ColumnPaint {
                x,
                extent: peak_extent,
                color: style.fill,
                alpha: band_alpha(stats.peak, style.scale),
            });
            self.stroke_symmetric_extents(ColumnPaint {
                x,
                extent: peak_extent,
                color: style.ridge,
                alpha: 0.7,
            });
        }
    }

    fn draw_mono_ridge(&mut self, file: &WaveformFile, geometry: WaveformGeometry) {
        for x in 0..self.width() {
            let column = geometry.column_range(x);
            let stats = file
                .mono_summary
                .stats(&file.mono_samples, column.start, column.end);
            self.draw_symmetric_column(ColumnPaint {
                x,
                extent: stats.peak * geometry.half * 0.36,
                color: [255, 255, 255, 245],
                alpha: 0.72,
            });
        }
    }

    fn draw_symmetric_column(&mut self, paint: ColumnPaint) {
        let top = (self.mid() - paint.extent).round().max(0.0) as usize;
        let bottom = (self.mid() + paint.extent)
            .round()
            .min(self.height().saturating_sub(1) as f32) as usize;
        for y in top..=bottom {
            self.blend_pixel(PixelPaint {
                x: paint.x,
                y,
                color: paint.color,
                alpha: paint.alpha * column_alpha(y, self.mid(), self.height() as f32 * 0.44),
            });
        }
    }

    fn stroke_symmetric_extents(&mut self, paint: ColumnPaint) {
        let top = (self.mid() - paint.extent).round().max(0.0) as usize;
        let bottom = (self.mid() + paint.extent)
            .round()
            .min(self.height().saturating_sub(1) as f32) as usize;
        self.blend_pixel(PixelPaint {
            x: paint.x,
            y: top,
            color: paint.color,
            alpha: paint.alpha,
        });
        self.blend_pixel(PixelPaint {
            x: paint.x,
            y: bottom,
            color: paint.color,
            alpha: paint.alpha,
        });
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

    fn size(&self) -> RasterSize {
        RasterSize {
            width: self.width(),
            height: self.height(),
        }
    }

    fn mid(&self) -> f32 {
        self.height() as f32 * 0.5
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct BandStyle {
    fill: [u8; 4],
    ridge: [u8; 4],
    scale: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct WaveformGeometry {
    viewport: WaveformViewport,
    visible: usize,
    width: usize,
    half: f32,
}

impl WaveformGeometry {
    fn new(viewport: WaveformViewport, size: RasterSize) -> Self {
        Self {
            viewport,
            visible: viewport.visible_items().max(1),
            width: size.width.max(1),
            half: (size.height as f32 * 0.42).max(1.0),
        }
    }

    fn column_range(self, x: usize) -> std::ops::Range<usize> {
        let start_offset = x * self.visible / self.width;
        let end_offset = ((x + 1) * self.visible / self.width).max(start_offset + 1);
        let start = self.viewport.start + start_offset;
        let end = (self.viewport.start + end_offset).min(self.viewport.end);
        start..end
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct ColumnPaint {
    x: usize,
    extent: f32,
    color: [u8; 4],
    alpha: f32,
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

fn band_alpha(peak: f32, scale: f32) -> f32 {
    (0.34 + peak * 0.72 * scale).clamp(0.28, 0.9)
}

fn lerp(from: f32, to: f32, t: f32) -> f32 {
    from + (to - from) * t.clamp(0.0, 1.0)
}
