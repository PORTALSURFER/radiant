use super::WaveformRaster;

pub(super) fn draw_band_labels(raster: &mut WaveformRaster) {
    let labels = [
        ("low", [32, 139, 255, 255]),
        ("low_mid", [205, 132, 60, 255]),
        ("mid", [255, 190, 84, 255]),
        ("high", [255, 255, 255, 255]),
    ];
    let mut x = 8;
    for (label, color) in labels {
        draw_block_label(raster, x, 8, label, color);
        x += label.len() * 6 + 18;
    }
}

fn draw_block_label(raster: &mut WaveformRaster, x: usize, y: usize, label: &str, color: [u8; 4]) {
    for swatch_x in x..x + 8 {
        for swatch_y in y + 1..y + 9 {
            raster.blend_pixel(swatch_x, swatch_y, color, 0.85);
        }
    }
    let mut cursor = x + 12;
    for ch in label.chars() {
        draw_glyph(raster, cursor, y, ch, color);
        cursor += 5;
    }
}

fn draw_glyph(raster: &mut WaveformRaster, x: usize, y: usize, ch: char, color: [u8; 4]) {
    let rows = glyph_rows(ch);
    for (row, bits) in rows.iter().enumerate() {
        for col in 0..3 {
            if bits & (1 << (2 - col)) != 0 {
                raster.blend_pixel(x + col, y + row, color, 0.9);
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
