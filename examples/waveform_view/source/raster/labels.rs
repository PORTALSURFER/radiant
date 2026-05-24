use super::{PixelPaint, WaveformRaster};

pub(super) fn draw_band_labels(raster: &mut WaveformRaster) {
    let labels = [
        ("low", [32, 139, 255, 255]),
        ("low_mid", [205, 132, 60, 255]),
        ("mid", [255, 190, 84, 255]),
        ("high", [255, 255, 255, 255]),
    ];
    let mut x = 8;
    for (label, color) in labels {
        draw_block_label(
            raster,
            LabelPaint {
                x,
                y: 8,
                label,
                color,
            },
        );
        x += label.len() * 6 + 18;
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct LabelPaint<'a> {
    x: usize,
    y: usize,
    label: &'a str,
    color: [u8; 4],
}

fn draw_block_label(raster: &mut WaveformRaster, paint: LabelPaint<'_>) {
    for swatch_x in paint.x..paint.x + 8 {
        for swatch_y in paint.y + 1..paint.y + 9 {
            raster.blend_pixel(PixelPaint {
                x: swatch_x,
                y: swatch_y,
                color: paint.color,
                alpha: 0.85,
            });
        }
    }
    let mut cursor = paint.x + 12;
    for ch in paint.label.chars() {
        draw_glyph(
            raster,
            GlyphPaint {
                x: cursor,
                y: paint.y,
                ch,
                color: paint.color,
            },
        );
        cursor += 5;
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct GlyphPaint {
    x: usize,
    y: usize,
    ch: char,
    color: [u8; 4],
}

fn draw_glyph(raster: &mut WaveformRaster, paint: GlyphPaint) {
    let rows = glyph_rows(paint.ch);
    for (row, bits) in rows.iter().enumerate() {
        for col in 0..3 {
            if bits & (1 << (2 - col)) != 0 {
                raster.blend_pixel(PixelPaint {
                    x: paint.x + col,
                    y: paint.y + row,
                    color: paint.color,
                    alpha: 0.9,
                });
            }
        }
    }
}

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
