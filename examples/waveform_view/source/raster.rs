use super::{WaveformBand, WaveformFile, WaveformViewport};
use radiant::gui::types::ImageRgba;

#[allow(dead_code)]
pub(crate) fn render_waveform_image(
    file: &WaveformFile,
    viewport: WaveformViewport,
    width: usize,
    height: usize,
) -> ImageRgba {
    let mut image = WaveformRaster::new(width, height);
    image.fill_background();
    image.draw_grid();
    image.draw_waveform(file, viewport);
    image.into_image()
}

#[allow(dead_code)]
struct WaveformRaster {
    width: usize,
    height: usize,
    pixels: Vec<u8>,
}

#[allow(dead_code)]
impl WaveformRaster {
    fn new(width: usize, height: usize) -> Self {
        Self {
            width,
            height,
            pixels: vec![0; width.saturating_mul(height).saturating_mul(4)],
        }
    }

    fn fill_background(&mut self) {
        for y in 0..self.height {
            let t = y as f32 / self.height.max(1) as f32;
            let shade = lerp(1.0, 8.0, t) as u8;
            for x in 0..self.width {
                self.put_pixel(
                    x,
                    y,
                    [shade, shade.saturating_add(1), shade.saturating_add(1), 255],
                );
            }
        }
    }

    fn draw_grid(&mut self) {
        let major = [46, 48, 50, 255];
        let minor = [22, 24, 26, 255];
        for x in (0..self.width).step_by((self.width / 16).max(1)) {
            let color = if x % ((self.width / 4).max(1)) == 0 {
                major
            } else {
                minor
            };
            for y in 0..self.height {
                self.blend_pixel(x, y, color, 0.55);
            }
        }
        for y in (0..self.height).step_by((self.height / 4).max(1)) {
            for x in 0..self.width {
                self.blend_pixel(x, y, minor, 0.5);
            }
        }
        let mid = self.height / 2;
        for x in 0..self.width {
            self.blend_pixel(x, mid, [82, 82, 78, 255], 0.55);
        }
    }

    fn draw_waveform(&mut self, file: &WaveformFile, viewport: WaveformViewport) {
        let viewport = viewport.clamp(file.frames);
        let visible = viewport.visible_frames().max(1);
        let mid = self.height as f32 * 0.5;
        let half = (self.height as f32 * 0.42).max(1.0);
        self.draw_band_labels();

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
        for x in 0..self.width {
            let start = viewport.start + x * visible / self.width.max(1);
            let end = viewport.start
                + ((x + 1) * visible / self.width.max(1)).max(x * visible / self.width.max(1) + 1);
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
        for x in 0..self.width {
            let start = viewport.start + x * visible / self.width.max(1);
            let end = viewport.start
                + ((x + 1) * visible / self.width.max(1)).max(x * visible / self.width.max(1) + 1);
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
        let bottom = (mid + extent).round().min((self.height - 1) as f32) as usize;
        for y in top..=bottom {
            self.blend_pixel(
                x,
                y,
                color,
                alpha * column_alpha(y, mid, self.height as f32 * 0.44),
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
        let bottom = (mid + extent).round().min((self.height - 1) as f32) as usize;
        self.blend_pixel(x, top, color, alpha);
        self.blend_pixel(x, bottom, color, alpha);
    }

    fn draw_band_labels(&mut self) {
        let labels = [
            ("low", [32, 139, 255, 255]),
            ("low_mid", [205, 132, 60, 255]),
            ("mid", [255, 190, 84, 255]),
            ("high", [255, 255, 255, 255]),
        ];
        let mut x = 8;
        for (label, color) in labels {
            self.draw_block_label(x, 8, label, color);
            x += label.len() * 6 + 18;
        }
    }

    fn draw_block_label(&mut self, x: usize, y: usize, label: &str, color: [u8; 4]) {
        for swatch_x in x..x + 8 {
            for swatch_y in y + 1..y + 9 {
                self.blend_pixel(swatch_x, swatch_y, color, 0.85);
            }
        }
        let mut cursor = x + 12;
        for ch in label.chars() {
            self.draw_glyph(cursor, y, ch, color);
            cursor += 5;
        }
    }

    fn draw_glyph(&mut self, x: usize, y: usize, ch: char, color: [u8; 4]) {
        let rows = glyph_rows(ch);
        for (row, bits) in rows.iter().enumerate() {
            for col in 0..3 {
                if bits & (1 << (2 - col)) != 0 {
                    self.blend_pixel(x + col, y + row, color, 0.9);
                }
            }
        }
    }

    fn into_image(self) -> ImageRgba {
        ImageRgba::new(self.width, self.height, self.pixels).expect("valid waveform image")
    }

    fn put_pixel(&mut self, x: usize, y: usize, color: [u8; 4]) {
        let Some(index) = self.pixel_index(x, y) else {
            return;
        };
        self.pixels[index..index + 4].copy_from_slice(&color);
    }

    fn blend_pixel(&mut self, x: usize, y: usize, color: [u8; 4], alpha: f32) {
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

#[derive(Clone, Copy, Debug, PartialEq)]
#[allow(dead_code)]
struct BandStyle {
    fill: [u8; 4],
    ridge: [u8; 4],
    scale: f32,
}

#[allow(dead_code)]
fn column_alpha(y: usize, mid: f32, half: f32) -> f32 {
    let distance = ((y as f32 - mid).abs() / half.max(1.0)).clamp(0.0, 1.0);
    lerp(0.42, 0.92, distance)
}

#[allow(dead_code)]
fn band_alpha(peak: f32, scale: f32) -> f32 {
    (0.34 + peak * 0.72 * scale).clamp(0.28, 0.9)
}

#[allow(dead_code)]
fn glyph_rows(ch: char) -> [u8; 7] {
    match ch {
        'd' => [0b110, 0b101, 0b101, 0b101, 0b101, 0b101, 0b110],
        'g' => [0b111, 0b100, 0b100, 0b101, 0b101, 0b101, 0b111],
        'h' => [0b101, 0b101, 0b101, 0b111, 0b101, 0b101, 0b101],
        'i' => [0b111, 0b010, 0b010, 0b010, 0b010, 0b010, 0b111],
        'l' => [0b100, 0b100, 0b100, 0b100, 0b100, 0b100, 0b111],
        'm' => [0b101, 0b111, 0b111, 0b101, 0b101, 0b101, 0b101],
        'o' => [0b111, 0b101, 0b101, 0b101, 0b101, 0b101, 0b111],
        'w' => [0b101, 0b101, 0b101, 0b101, 0b111, 0b111, 0b101],
        '_' => [0b000, 0b000, 0b000, 0b000, 0b000, 0b000, 0b111],
        _ => [0; 7],
    }
}

#[allow(dead_code)]
fn lerp(from: f32, to: f32, t: f32) -> f32 {
    from + (to - from) * t.clamp(0.0, 1.0)
}
