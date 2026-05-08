use radiant::{gui::types::ImageRgba, runtime::GpuSignalSummary};
use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
    path::PathBuf,
    sync::Arc,
};
const WAVEFORM_PATH_ENV_VAR: &str = "RADIANT_WAVEFORM_PATH";
pub(super) const MIN_VISIBLE_FRAMES: usize = 256;
pub(super) const BAND_COUNT: usize = 4;
const SUMMARY_BLOCK_FRAMES: usize = 128;
const SYNTHETIC_SAMPLE_RATE: u32 = 48_000;
const SYNTHETIC_SECONDS: usize = 4;

#[derive(Clone, Debug)]
#[allow(dead_code)]
pub(super) struct WaveformFile {
    pub(super) path: PathBuf,
    pub(super) sample_rate: u32,
    pub(super) channels: usize,
    pub(super) frames: usize,
    pub(super) mono_samples: Vec<f32>,
    pub(super) mono_summary: WaveformSummary,
    pub(super) bands: [WaveformBand; BAND_COUNT],
    pub(super) gpu_signal_samples: Arc<[f32]>,
    pub(super) gpu_signal_summary: Arc<GpuSignalSummary>,
}

impl WaveformFile {
    pub(super) fn path_hash(&self) -> u64 {
        let mut hasher = DefaultHasher::new();
        self.path.hash(&mut hasher);
        self.frames.hash(&mut hasher);
        self.sample_rate.hash(&mut hasher);
        hasher.finish()
    }
}

#[derive(Clone, Debug)]
#[allow(dead_code)]
pub(super) struct WaveformBand {
    samples: Vec<f32>,
    summary: WaveformSummary,
}

#[derive(Clone, Debug)]
#[allow(dead_code)]
pub(super) struct WaveformSummary {
    blocks: Vec<SummaryBlock>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(super) struct SummaryBlock {
    peak: f32,
    energy: f32,
    count: usize,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct WaveformViewport {
    pub(super) start: usize,
    pub(super) end: usize,
}

pub(super) fn resolve_sample_path() -> Option<PathBuf> {
    std::env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .or_else(|| std::env::var_os(WAVEFORM_PATH_ENV_VAR).map(PathBuf::from))
}

pub(super) fn load_waveform_source(path: Option<PathBuf>) -> Result<WaveformFile, String> {
    match path {
        Some(path) => load_waveform_file(path),
        None => Ok(synthetic_waveform_file()),
    }
}

fn synthetic_waveform_file() -> WaveformFile {
    let frames = SYNTHETIC_SAMPLE_RATE as usize * SYNTHETIC_SECONDS;
    let samples = (0..frames)
        .map(|frame| {
            let t = frame as f32 / SYNTHETIC_SAMPLE_RATE as f32;
            let envelope = (1.0 - t / SYNTHETIC_SECONDS as f32).clamp(0.18, 1.0);
            let low = (std::f32::consts::TAU * 72.0 * t).sin() * 0.48;
            let mid = (std::f32::consts::TAU * 220.0 * t).sin() * 0.24;
            let high = (std::f32::consts::TAU * 1_760.0 * t).sin() * 0.1;
            ((low + mid + high) * envelope).clamp(-1.0, 1.0)
        })
        .collect::<Vec<_>>();
    waveform_file_from_mono_samples(
        PathBuf::from("synthetic-waveform"),
        SYNTHETIC_SAMPLE_RATE,
        1,
        samples,
    )
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

    let frames = samples.len() / channels;
    let mono_samples = downmix_to_mono(&samples, channels, frames);
    if mono_samples.is_empty() {
        return Err(String::from("WAV contains no complete frames"));
    }
    let mono_summary = WaveformSummary::from_samples(&mono_samples);
    let bands = split_frequency_bands(&mono_samples, spec.sample_rate);
    let gpu_signal_samples = interleaved_band_samples(&bands);
    let gpu_signal_summary = Arc::new(GpuSignalSummary::from_interleaved_samples(
        &gpu_signal_samples,
        frames,
        BAND_COUNT,
    ));

    Ok(WaveformFile {
        path,
        sample_rate: spec.sample_rate,
        channels,
        frames,
        mono_samples,
        mono_summary,
        bands,
        gpu_signal_samples,
        gpu_signal_summary,
    })
}

pub(super) fn waveform_file_from_mono_samples(
    path: PathBuf,
    sample_rate: u32,
    channels: usize,
    mono_samples: Vec<f32>,
) -> WaveformFile {
    let bands = split_frequency_bands(&mono_samples, sample_rate);
    let gpu_signal_samples = interleaved_band_samples(&bands);
    let gpu_signal_summary = Arc::new(GpuSignalSummary::from_interleaved_samples(
        &gpu_signal_samples,
        mono_samples.len(),
        BAND_COUNT,
    ));
    WaveformFile {
        path,
        sample_rate,
        channels,
        frames: mono_samples.len(),
        mono_summary: WaveformSummary::from_samples(&mono_samples),
        bands,
        mono_samples,
        gpu_signal_samples,
        gpu_signal_summary,
    }
}

pub(super) fn interleaved_band_samples(bands: &[WaveformBand; BAND_COUNT]) -> Arc<[f32]> {
    let frames = bands
        .first()
        .map(|band| band.samples.len())
        .unwrap_or_default();
    let mut samples = Vec::with_capacity(frames.saturating_mul(BAND_COUNT));
    for frame in 0..frames {
        for band in bands {
            samples.push(band.samples.get(frame).copied().unwrap_or_default());
        }
    }
    samples.into()
}

pub(super) fn downmix_to_mono(samples: &[f32], channels: usize, frames: usize) -> Vec<f32> {
    let channels = channels.max(1);
    (0..frames)
        .map(|frame| {
            let start = frame * channels;
            let sum = samples[start..start + channels]
                .iter()
                .copied()
                .sum::<f32>();
            (sum / channels as f32).clamp(-1.0, 1.0)
        })
        .collect()
}

fn split_frequency_bands(samples: &[f32], sample_rate: u32) -> [WaveformBand; BAND_COUNT] {
    let low_160 = lowpass(samples, sample_rate, 160.0);
    let low_700 = lowpass(samples, sample_rate, 700.0);
    let low_2k8 = lowpass(samples, sample_rate, 2_800.0);
    let low = low_160.clone();
    let low_mid = subtract_samples(&low_700, &low_160);
    let mid = subtract_samples(&low_2k8, &low_700);
    let high = subtract_samples(samples, &low_2k8);
    [
        WaveformBand::new(normalized_band(low, 1.45)),
        WaveformBand::new(normalized_band(low_mid, 1.25)),
        WaveformBand::new(normalized_band(mid, 1.1)),
        WaveformBand::new(normalized_band(high, 0.95)),
    ]
}

fn lowpass(samples: &[f32], sample_rate: u32, cutoff_hz: f32) -> Vec<f32> {
    let alpha = (1.0 - (-std::f32::consts::TAU * cutoff_hz / sample_rate.max(1) as f32).exp())
        .clamp(0.0, 1.0);
    let mut value = 0.0_f32;
    samples
        .iter()
        .map(|sample| {
            value += alpha * (*sample - value);
            value
        })
        .collect()
}

fn subtract_samples(left: &[f32], right: &[f32]) -> Vec<f32> {
    left.iter()
        .zip(right)
        .map(|(left, right)| (left - right).clamp(-1.0, 1.0))
        .collect()
}

fn normalized_band(mut samples: Vec<f32>, gain: f32) -> Vec<f32> {
    let peak = samples
        .iter()
        .map(|sample| sample.abs())
        .fold(0.0_f32, f32::max)
        .max(0.001);
    let scale = gain / peak.max(0.32);
    for sample in &mut samples {
        *sample = (*sample * scale).clamp(-1.0, 1.0);
    }
    samples
}

#[allow(dead_code)]
pub(super) fn render_waveform_image(
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

#[derive(Clone, Copy, Debug, PartialEq)]
#[allow(dead_code)]
pub(super) struct BandStats {
    pub(super) peak: f32,
    pub(super) rms: f32,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[allow(dead_code)]
struct StatsAccumulator {
    peak: f32,
    energy: f32,
    count: usize,
}

#[allow(dead_code)]
impl WaveformBand {
    pub(super) fn new(samples: Vec<f32>) -> Self {
        let summary = WaveformSummary::from_samples(&samples);
        Self { samples, summary }
    }

    fn stats(&self, start: usize, end: usize) -> BandStats {
        self.summary.stats(&self.samples, start, end)
    }
}

#[allow(dead_code)]
impl WaveformSummary {
    pub(super) fn from_samples(samples: &[f32]) -> Self {
        let blocks = samples
            .chunks(SUMMARY_BLOCK_FRAMES)
            .map(SummaryBlock::from_samples)
            .collect();
        Self { blocks }
    }

    pub(super) fn stats(&self, samples: &[f32], start: usize, end: usize) -> BandStats {
        let start = start.min(samples.len());
        let end = end.min(samples.len()).max(start + 1).min(samples.len());
        if end <= start {
            return BandStats {
                peak: 0.0,
                rms: 0.0,
            };
        }
        if end - start <= SUMMARY_BLOCK_FRAMES * 2 {
            return SummaryBlock::from_samples(&samples[start..end]).into_stats();
        }

        let first_full_block = start.div_ceil(SUMMARY_BLOCK_FRAMES);
        let last_full_block = end / SUMMARY_BLOCK_FRAMES;
        let mut stats = StatsAccumulator::default();
        let left_end = (first_full_block * SUMMARY_BLOCK_FRAMES).min(end);
        stats.add_samples(&samples[start..left_end]);
        for block in &self.blocks[first_full_block..last_full_block] {
            stats.add_block(*block);
        }
        let right_start = (last_full_block * SUMMARY_BLOCK_FRAMES).max(left_end);
        stats.add_samples(&samples[right_start..end]);
        stats.into_stats()
    }
}

#[allow(dead_code)]
impl SummaryBlock {
    pub(super) fn from_samples(samples: &[f32]) -> Self {
        let mut block = Self::default();
        for sample in samples {
            block.peak = block.peak.max(sample.abs());
            block.energy += sample * sample;
            block.count += 1;
        }
        block
    }

    fn into_stats(self) -> BandStats {
        StatsAccumulator {
            peak: self.peak,
            energy: self.energy,
            count: self.count,
        }
        .into_stats()
    }
}

#[allow(dead_code)]
impl StatsAccumulator {
    fn add_samples(&mut self, samples: &[f32]) {
        for sample in samples {
            self.peak = self.peak.max(sample.abs());
            self.energy += sample * sample;
            self.count += 1;
        }
    }

    fn add_block(&mut self, block: SummaryBlock) {
        self.peak = self.peak.max(block.peak);
        self.energy += block.energy;
        self.count += block.count;
    }

    fn into_stats(self) -> BandStats {
        BandStats {
            peak: self.peak,
            rms: if self.count == 0 {
                0.0
            } else {
                self.energy / self.count as f32
            },
        }
    }
}

#[cfg(test)]
pub(super) fn band_stats(samples: &[f32], start: usize, end: usize) -> BandStats {
    let start = start.min(samples.len());
    let end = end.min(samples.len()).max(start + 1).min(samples.len());
    SummaryBlock::from_samples(&samples[start..end]).into_stats()
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

impl WaveformViewport {
    pub(super) fn full(frames: usize) -> Self {
        Self {
            start: 0,
            end: frames.max(1),
        }
    }

    pub(super) fn visible_frames(self) -> usize {
        self.end.saturating_sub(self.start).max(1)
    }

    pub(super) fn visible_seconds(self, sample_rate: u32) -> f32 {
        self.visible_frames() as f32 / sample_rate.max(1) as f32
    }

    pub(super) fn visible_fraction(self, total_frames: usize) -> f32 {
        self.visible_frames() as f32 / total_frames.max(1) as f32
    }

    pub(super) fn offset_fraction(self, total_frames: usize) -> f32 {
        let total_frames = total_frames.max(1);
        let free_frames = total_frames.saturating_sub(self.visible_frames());
        if free_frames == 0 {
            0.0
        } else {
            self.start as f32 / free_frames as f32
        }
    }

    pub(super) fn is_zoomed_in(self, total_frames: usize) -> bool {
        self.visible_frames() < total_frames.max(1)
    }

    pub(super) fn clamp(self, total_frames: usize) -> Self {
        let total_frames = total_frames.max(1);
        let visible = self
            .visible_frames()
            .clamp(MIN_VISIBLE_FRAMES.min(total_frames), total_frames);
        let start = self.start.min(total_frames.saturating_sub(visible));
        Self {
            start,
            end: start + visible,
        }
    }
}
