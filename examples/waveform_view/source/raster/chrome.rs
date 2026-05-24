use super::{PixelPaint, WaveformRaster, lerp};

pub(super) fn draw_raster_chrome(raster: &mut WaveformRaster) {
    fill_background(raster);
    draw_grid(raster);
}

fn fill_background(raster: &mut WaveformRaster) {
    for y in 0..raster.height() {
        let t = y as f32 / raster.height().max(1) as f32;
        let shade = lerp(1.0, 8.0, t) as u8;
        for x in 0..raster.width() {
            raster.put_pixel(
                x,
                y,
                [shade, shade.saturating_add(1), shade.saturating_add(1), 255],
            );
        }
    }
}

fn draw_grid(raster: &mut WaveformRaster) {
    let major = [46, 48, 50, 255];
    let minor = [22, 24, 26, 255];
    for x in (0..raster.width()).step_by((raster.width() / 16).max(1)) {
        let color = if x % ((raster.width() / 4).max(1)) == 0 {
            major
        } else {
            minor
        };
        for y in 0..raster.height() {
            raster.blend_pixel(PixelPaint {
                x,
                y,
                color,
                alpha: 0.55,
            });
        }
    }
    for y in (0..raster.height()).step_by((raster.height() / 4).max(1)) {
        for x in 0..raster.width() {
            raster.blend_pixel(PixelPaint {
                x,
                y,
                color: minor,
                alpha: 0.5,
            });
        }
    }
    let mid = raster.height() / 2;
    for x in 0..raster.width() {
        raster.blend_pixel(PixelPaint {
            x,
            y: mid,
            color: [82, 82, 78, 255],
            alpha: 0.55,
        });
    }
}
