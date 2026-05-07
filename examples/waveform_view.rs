//! Load one WAV file and display it as a Radiant waveform view.

use radiant::gui::types::ImageRgba;
use radiant::prelude as ui;
use std::{path::PathBuf, sync::Arc};

const DEFAULT_SAMPLE_PATH: &str = r"C:\dev\sempal\assets\portal_SS_kick_003.wav";
const FALLBACK_SAMPLE_PATH: &str = r"..\..\assets\portal_SS_kick_003.wav";
const WAVEFORM_WIDTH: usize = 1100;
const WAVEFORM_HEIGHT: usize = 320;

#[derive(Clone, Debug)]
struct WaveformFile {
    path: PathBuf,
    sample_rate: u32,
    channels: usize,
    samples: Vec<f32>,
}

fn main() -> radiant::Result {
    let file = load_waveform_file(resolve_sample_path()?)?;
    let image = Arc::new(
        render_waveform_image(&file, WAVEFORM_WIDTH, WAVEFORM_HEIGHT)
            .ok_or_else(|| String::from("failed to allocate waveform image"))?,
    );
    let title = format!(
        "{} | {} Hz | {} channel{} | {} frames",
        file.path.display(),
        file.sample_rate,
        file.channels,
        if file.channels == 1 { "" } else { "s" },
        file.samples.len() / file.channels.max(1)
    );

    radiant::window("Radiant Waveform View")
        .size(1180, 480)
        .min_size(760, 360)
        .run(view(title, image))
}

fn view(title: String, image: Arc<ImageRgba>) -> ui::View<()> {
    ui::column([
        ui::text("Waveform").height(28.0).fill_width(),
        ui::text(title).height(24.0).fill_width().truncate(),
        ui::image(image)
            .id(10)
            .size(WAVEFORM_WIDTH as f32, WAVEFORM_HEIGHT as f32)
            .fill_width()
            .height(WAVEFORM_HEIGHT as f32),
        ui::spacer().fill(),
    ])
    .padding(16.0)
    .spacing(10.0)
    .fill()
}

fn resolve_sample_path() -> Result<PathBuf, String> {
    if let Some(arg) = std::env::args_os().nth(1) {
        return Ok(PathBuf::from(arg));
    }
    let default = PathBuf::from(DEFAULT_SAMPLE_PATH);
    if default.is_file() {
        return Ok(default);
    }
    let fallback = PathBuf::from(FALLBACK_SAMPLE_PATH);
    if fallback.is_file() {
        return Ok(fallback);
    }
    Err(format!(
        "waveform file not found; pass a path or place a WAV at {DEFAULT_SAMPLE_PATH}"
    ))
}

fn load_waveform_file(path: PathBuf) -> Result<WaveformFile, String> {
    let mut reader =
        hound::WavReader::open(&path).map_err(|err| format!("failed to open WAV: {err}"))?;
    let spec = reader.spec();
    let channels = usize::from(spec.channels).max(1);
    let samples = match spec.sample_format {
        hound::SampleFormat::Float => reader
            .samples::<f32>()
            .map(|sample| {
                sample
                    .map(|value| value.clamp(-1.0, 1.0))
                    .map_err(|err| format!("failed to read float sample: {err}"))
            })
            .collect::<Result<Vec<_>, _>>()?,
        hound::SampleFormat::Int if spec.bits_per_sample <= 16 => {
            let max =
                ((1_i32 << (u32::from(spec.bits_per_sample).saturating_sub(1))) - 1).max(1) as f32;
            reader
                .samples::<i16>()
                .map(|sample| {
                    sample
                        .map(|value| (f32::from(value) / max).clamp(-1.0, 1.0))
                        .map_err(|err| format!("failed to read integer sample: {err}"))
                })
                .collect::<Result<Vec<_>, _>>()?
        }
        hound::SampleFormat::Int => {
            let max =
                ((1_i64 << (u32::from(spec.bits_per_sample).saturating_sub(1))) - 1).max(1) as f32;
            reader
                .samples::<i32>()
                .map(|sample| {
                    sample
                        .map(|value| ((value as f32) / max).clamp(-1.0, 1.0))
                        .map_err(|err| format!("failed to read integer sample: {err}"))
                })
                .collect::<Result<Vec<_>, _>>()?
        }
    };
    if samples.is_empty() {
        return Err(String::from("WAV contains no samples"));
    }
    Ok(WaveformFile {
        path,
        sample_rate: spec.sample_rate,
        channels,
        samples,
    })
}

fn render_waveform_image(file: &WaveformFile, width: usize, height: usize) -> Option<ImageRgba> {
    let mut pixels = vec![0; width.checked_mul(height)?.checked_mul(4)?];
    fill_background(&mut pixels, width, height);
    draw_grid(&mut pixels, width, height);

    if file.channels >= 2 {
        let gap = 10;
        let band_height = (height.saturating_sub(gap)) / 2;
        draw_channel(&mut pixels, file, 0, width, 0, band_height);
        draw_channel(
            &mut pixels,
            file,
            1,
            width,
            band_height + gap,
            height.saturating_sub(band_height + gap),
        );
    } else {
        draw_channel(&mut pixels, file, 0, width, 0, height);
    }

    ImageRgba::new(width, height, pixels)
}

fn fill_background(pixels: &mut [u8], width: usize, height: usize) {
    for y in 0..height {
        let shade = 18_u8.saturating_add(((y as f32 / height.max(1) as f32) * 10.0) as u8);
        for x in 0..width {
            put_pixel(
                pixels,
                width,
                x,
                y,
                [shade, shade.saturating_add(2), 25, 255],
            );
        }
    }
}

fn draw_grid(pixels: &mut [u8], width: usize, height: usize) {
    let grid = [48, 50, 56, 255];
    for x in (0..width).step_by((width / 12).max(1)) {
        for y in 0..height {
            put_pixel(pixels, width, x, y, grid);
        }
    }
    for y in (0..height).step_by((height / 4).max(1)) {
        for x in 0..width {
            put_pixel(pixels, width, x, y, grid);
        }
    }
}

fn draw_channel(
    pixels: &mut [u8],
    file: &WaveformFile,
    channel: usize,
    width: usize,
    top: usize,
    height: usize,
) {
    if height == 0 {
        return;
    }
    let frame_count = file.samples.len() / file.channels.max(1);
    let mid = top + height / 2;
    let half = (height.saturating_sub(2) as f32 / 2.0).max(1.0);
    let body = [242, 107, 82, 225];
    let outline = [255, 183, 148, 255];
    let center = [92, 86, 90, 255];

    for x in 0..width {
        put_pixel(pixels, width, x, mid.min(top + height - 1), center);
        let start = x * frame_count / width.max(1);
        let end = ((x + 1) * frame_count / width.max(1)).max(start + 1);
        let (min, max) = sample_range(file, channel, start, end.min(frame_count));
        let y_top = sample_y(top, half, max).min(top + height - 1);
        let y_bottom = sample_y(top, half, min).min(top + height - 1);
        let from = y_top.min(y_bottom);
        let to = y_top.max(y_bottom);
        for y in from..=to {
            put_pixel(pixels, width, x, y, body);
        }
        put_pixel(pixels, width, x, from, outline);
        put_pixel(pixels, width, x, to, outline);
    }
}

fn sample_range(file: &WaveformFile, channel: usize, start: usize, end: usize) -> (f32, f32) {
    let mut min = 0.0_f32;
    let mut max = 0.0_f32;
    for frame in start..end {
        let index = frame * file.channels + channel.min(file.channels - 1);
        if let Some(sample) = file.samples.get(index).copied() {
            min = min.min(sample);
            max = max.max(sample);
        }
    }
    (min, max)
}

fn sample_y(top: usize, half: f32, sample: f32) -> usize {
    (top as f32 + half - sample.clamp(-1.0, 1.0) * half)
        .round()
        .max(0.0) as usize
}

fn put_pixel(pixels: &mut [u8], width: usize, x: usize, y: usize, color: [u8; 4]) {
    let Some(index) = y
        .checked_mul(width)
        .and_then(|row| row.checked_add(x))
        .and_then(|pixel| pixel.checked_mul(4))
    else {
        return;
    };
    if index + 3 >= pixels.len() {
        return;
    }
    pixels[index..index + 4].copy_from_slice(&color);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn synthetic_waveform_renders_nonblank_image() {
        let file = WaveformFile {
            path: PathBuf::from("synthetic.wav"),
            sample_rate: 48_000,
            channels: 1,
            samples: (0..512)
                .map(|index| ((index as f32 / 16.0).sin() * 0.8).clamp(-1.0, 1.0))
                .collect(),
        };

        let image = render_waveform_image(&file, 128, 48).expect("image should render");

        assert_eq!(image.width, 128);
        assert_eq!(image.height, 48);
        assert!(
            image
                .pixels
                .chunks_exact(4)
                .any(|pixel| pixel[0] == 255 && pixel[1] == 183 && pixel[2] == 148),
            "waveform outline should produce visible bright pixels"
        );
    }

    #[test]
    fn provided_sample_decodes_when_available() {
        let path = PathBuf::from(DEFAULT_SAMPLE_PATH);
        if !path.is_file() {
            return;
        }

        let file = load_waveform_file(path).expect("provided sample should decode");

        assert!(file.sample_rate > 0);
        assert!(!file.samples.is_empty());
        assert!(render_waveform_image(&file, 320, 96).is_some());
    }
}
