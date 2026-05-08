//! Load one WAV file and display it as an interactive mono waveform view.

use radiant::gui::types::ImageRgba;
use radiant::prelude as ui;
use radiant::{
    gui::types::{Point, Rect, Rgba8, Vector2},
    layout::LayoutOutput,
    runtime::{
        GpuHoverCursor, GpuSignalSummary, GpuSurfaceCapabilities, GpuSurfaceContent,
        PaintGpuSurface, PaintPrimitive,
    },
    theme::ThemeTokens,
    widgets::{
        FocusBehavior, PaintBounds, ScrollbarAxis, ScrollbarMessage, ScrollbarWidget, Widget,
        WidgetCommon, WidgetInput, WidgetOutput, WidgetSizing,
    },
};
use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
    path::PathBuf,
    sync::Arc,
};

const WAVEFORM_PATH_ENV_VAR: &str = "RADIANT_WAVEFORM_PATH";
const WAVEFORM_WIDTH: usize = 1200;
const WAVEFORM_HEIGHT: usize = 320;
const MIN_VISIBLE_FRAMES: usize = 256;
const BAND_COUNT: usize = 4;
const SUMMARY_BLOCK_FRAMES: usize = 128;
const SYNTHETIC_SAMPLE_RATE: u32 = 48_000;
const SYNTHETIC_SECONDS: usize = 4;

#[derive(Clone, Debug)]
#[allow(dead_code)]
struct WaveformFile {
    path: PathBuf,
    sample_rate: u32,
    channels: usize,
    frames: usize,
    mono_samples: Vec<f32>,
    mono_summary: WaveformSummary,
    bands: [WaveformBand; BAND_COUNT],
    gpu_signal_samples: Arc<[f32]>,
    gpu_signal_summary: Arc<GpuSignalSummary>,
}

impl WaveformFile {
    fn path_hash(&self) -> u64 {
        let mut hasher = DefaultHasher::new();
        self.path.hash(&mut hasher);
        self.frames.hash(&mut hasher);
        self.sample_rate.hash(&mut hasher);
        hasher.finish()
    }
}

#[derive(Clone, Debug)]
#[allow(dead_code)]
struct WaveformBand {
    samples: Vec<f32>,
    summary: WaveformSummary,
}

#[derive(Clone, Debug)]
#[allow(dead_code)]
struct WaveformSummary {
    blocks: Vec<SummaryBlock>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct SummaryBlock {
    peak: f32,
    energy: f32,
    count: usize,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct WaveformViewport {
    start: usize,
    end: usize,
}

#[derive(Debug)]
struct WaveformApp {
    file: Arc<WaveformFile>,
    viewport: WaveformViewport,
    zoom_anchor_ratio: f32,
    playing: bool,
    playhead_ratio: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum WaveformInteraction {
    Wheel { delta: Vector2, anchor_ratio: f32 },
    ScrollTo { offset_fraction: f32 },
    Zoom { factor: f32 },
    Pan { visible_fraction: f32 },
    TogglePlayback,
    Frame,
    Reset,
}

fn main() -> radiant::Result {
    let file = Arc::new(load_waveform_source(resolve_sample_path())?);
    let viewport = WaveformViewport::full(file.frames);

    radiant::app(WaveformApp {
        file,
        viewport,
        zoom_anchor_ratio: 0.5,
        playing: false,
        playhead_ratio: 0.5,
    })
    .title("Radiant Waveform View")
    .size(1280, 560)
    .min_size(820, 420)
    .view(view)
    .animation(|state| state.playing)
    .on_frame(|| WaveformInteraction::Frame)
    .update_with(|state, message, context| {
        state.apply_interaction(message);
        context.request_repaint();
    })
    .run()
}

fn view(state: &mut WaveformApp) -> ui::View<WaveformInteraction> {
    let title = format!(
        "{} | {} Hz | {} channel{} -> mono | {} frames | {:.1} ms visible",
        state.file.path.display(),
        state.file.sample_rate,
        state.file.channels,
        if state.file.channels == 1 { "" } else { "s" },
        state.file.frames,
        state.viewport.visible_seconds(state.file.sample_rate) * 1000.0,
    );

    ui::column([
        ui::text("Waveform").height(28.0).fill_width(),
        ui::text(title).height(24.0).fill_width().truncate(),
        waveform_viewport(
            Arc::clone(&state.file),
            state.viewport,
            Some(if state.playing {
                state.playhead_ratio
            } else {
                state.zoom_anchor_ratio
            }),
        )
        .id(10)
        .size(WAVEFORM_WIDTH as f32, WAVEFORM_HEIGHT as f32)
        .fill_width()
        .height(WAVEFORM_HEIGHT as f32),
        waveform_scrollbar(state),
        waveform_controls(),
        ui::spacer().fill(),
    ])
    .padding(16.0)
    .spacing(10.0)
    .fill()
}

fn waveform_viewport(
    file: Arc<WaveformFile>,
    viewport: WaveformViewport,
    cursor_ratio: Option<f32>,
) -> ui::View<WaveformInteraction> {
    ui::custom_widget(
        WaveformWidget::new(file, viewport, cursor_ratio),
        |output| output.typed_ref::<WaveformInteraction>().copied(),
    )
}

fn waveform_scrollbar(state: &WaveformApp) -> ui::View<WaveformInteraction> {
    if !state.viewport.is_zoomed_in(state.file.frames) {
        return ui::spacer().fill_width().height(14.0);
    }

    let mut scrollbar = ScrollbarWidget::new(
        0,
        ScrollbarAxis::Horizontal,
        WidgetSizing::fixed(Vector2::new(WAVEFORM_WIDTH as f32, 14.0)),
    );
    scrollbar.props.viewport_fraction = state.viewport.visible_fraction(state.file.frames);
    scrollbar.state.offset_fraction = state.viewport.offset_fraction(state.file.frames);
    ui::custom_widget(scrollbar, |output| {
        output
            .typed_ref::<ScrollbarMessage>()
            .copied()
            .map(|message| match message {
                ScrollbarMessage::OffsetChanged { offset_fraction } => {
                    WaveformInteraction::ScrollTo { offset_fraction }
                }
            })
    })
    .fill_width()
    .height(14.0)
}

fn waveform_controls() -> ui::View<WaveformInteraction> {
    ui::row([
        ui::button("Zoom -")
            .subtle()
            .message(WaveformInteraction::Zoom { factor: 2.0 }),
        ui::button("Zoom +")
            .primary()
            .message(WaveformInteraction::Zoom { factor: 0.5 }),
        ui::button("Pan <")
            .subtle()
            .message(WaveformInteraction::Pan {
                visible_fraction: -0.25,
            }),
        ui::button("Pan >")
            .subtle()
            .message(WaveformInteraction::Pan {
                visible_fraction: 0.25,
            }),
        ui::button("Play")
            .subtle()
            .message(WaveformInteraction::TogglePlayback),
        ui::button("Reset")
            .subtle()
            .message(WaveformInteraction::Reset),
        ui::spacer().fill(),
    ])
    .spacing(8.0)
    .fill_width()
    .height(40.0)
}

#[derive(Clone, Debug)]
struct WaveformWidget {
    common: WidgetCommon,
    file: Arc<WaveformFile>,
    viewport: WaveformViewport,
}

impl WaveformWidget {
    fn new(
        file: Arc<WaveformFile>,
        viewport: WaveformViewport,
        _cursor_ratio: Option<f32>,
    ) -> Self {
        let mut common = WidgetCommon::new(
            0,
            WidgetSizing::fixed(Vector2::new(WAVEFORM_WIDTH as f32, WAVEFORM_HEIGHT as f32)),
        );
        common.focus = FocusBehavior::Pointer;
        common.paint.bounds = PaintBounds::ClipToRect;
        common.paint.paints_focus = false;
        common.paint.paints_state_layers = false;
        Self {
            common,
            file,
            viewport,
        }
    }

    fn ratio_from_position(&self, bounds: Rect, position: Point) -> f32 {
        ((position.x - bounds.min.x) / bounds.width().max(1.0)).clamp(0.0, 1.0)
    }
}

impl Widget for WaveformWidget {
    fn common(&self) -> &WidgetCommon {
        &self.common
    }

    fn common_mut(&mut self) -> &mut WidgetCommon {
        &mut self.common
    }

    fn handle_input(&mut self, bounds: Rect, input: WidgetInput) -> Option<WidgetOutput> {
        match input {
            WidgetInput::PointerMove { position } if bounds.contains(position) => {
                self.common.state.hovered = true;
                None
            }
            WidgetInput::PointerMove { .. } => {
                self.common.state.hovered = false;
                None
            }
            WidgetInput::Wheel { position, delta } if bounds.contains(position) => {
                Some(WidgetOutput::typed(WaveformInteraction::Wheel {
                    delta,
                    anchor_ratio: self.ratio_from_position(bounds, position),
                }))
            }
            _ => None,
        }
    }

    fn append_paint(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        bounds: Rect,
        _layout: &LayoutOutput,
        _theme: &ThemeTokens,
    ) {
        primitives.push(PaintPrimitive::GpuSurface(PaintGpuSurface {
            widget_id: self.common.id,
            key: self.file.path_hash(),
            revision: 0,
            rect: bounds,
            content: GpuSurfaceContent::SignalSummaryBands {
                frames: self.file.frames,
                band_count: BAND_COUNT,
                frame_range: [self.viewport.start as f32, self.viewport.end as f32],
                summary: Arc::clone(&self.file.gpu_signal_summary),
            },
            capabilities: GpuSurfaceCapabilities {
                fast_pointer_move: true,
                coalesce_vertical_wheel: true,
                native_hover_cursor: Some(GpuHoverCursor {
                    color: Rgba8 {
                        r: 255,
                        g: 255,
                        b: 255,
                        a: 235,
                    },
                    width: 1.5,
                }),
            },
            overlays: Vec::new(),
        }));
    }
}

fn resolve_sample_path() -> Option<PathBuf> {
    std::env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .or_else(|| std::env::var_os(WAVEFORM_PATH_ENV_VAR).map(PathBuf::from))
}

fn load_waveform_source(path: Option<PathBuf>) -> Result<WaveformFile, String> {
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

fn waveform_file_from_mono_samples(
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

fn interleaved_band_samples(bands: &[WaveformBand; BAND_COUNT]) -> Arc<[f32]> {
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

fn downmix_to_mono(samples: &[f32], channels: usize, frames: usize) -> Vec<f32> {
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
fn render_waveform_image(
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
struct BandStats {
    peak: f32,
    rms: f32,
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
    fn new(samples: Vec<f32>) -> Self {
        let summary = WaveformSummary::from_samples(&samples);
        Self { samples, summary }
    }

    fn stats(&self, start: usize, end: usize) -> BandStats {
        self.summary.stats(&self.samples, start, end)
    }
}

#[allow(dead_code)]
impl WaveformSummary {
    fn from_samples(samples: &[f32]) -> Self {
        let blocks = samples
            .chunks(SUMMARY_BLOCK_FRAMES)
            .map(SummaryBlock::from_samples)
            .collect();
        Self { blocks }
    }

    fn stats(&self, samples: &[f32], start: usize, end: usize) -> BandStats {
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
    fn from_samples(samples: &[f32]) -> Self {
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
fn band_stats(samples: &[f32], start: usize, end: usize) -> BandStats {
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
    fn full(frames: usize) -> Self {
        Self {
            start: 0,
            end: frames.max(1),
        }
    }

    fn visible_frames(self) -> usize {
        self.end.saturating_sub(self.start).max(1)
    }

    fn visible_seconds(self, sample_rate: u32) -> f32 {
        self.visible_frames() as f32 / sample_rate.max(1) as f32
    }

    fn visible_fraction(self, total_frames: usize) -> f32 {
        self.visible_frames() as f32 / total_frames.max(1) as f32
    }

    fn offset_fraction(self, total_frames: usize) -> f32 {
        let total_frames = total_frames.max(1);
        let free_frames = total_frames.saturating_sub(self.visible_frames());
        if free_frames == 0 {
            0.0
        } else {
            self.start as f32 / free_frames as f32
        }
    }

    fn is_zoomed_in(self, total_frames: usize) -> bool {
        self.visible_frames() < total_frames.max(1)
    }

    fn clamp(self, total_frames: usize) -> Self {
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

impl WaveformApp {
    fn apply_interaction(&mut self, interaction: WaveformInteraction) {
        match interaction {
            WaveformInteraction::Wheel {
                delta,
                anchor_ratio,
            } => {
                self.zoom_anchor_ratio = anchor_ratio;
                self.handle_wheel(delta, anchor_ratio);
            }
            WaveformInteraction::ScrollTo { offset_fraction } => {
                self.set_offset_fraction(offset_fraction)
            }
            WaveformInteraction::Zoom { factor } => {
                self.zoom_around_anchor(factor, self.zoom_anchor_ratio)
            }
            WaveformInteraction::Pan { visible_fraction } => {
                self.pan_by_visible_fraction(visible_fraction)
            }
            WaveformInteraction::TogglePlayback => {
                self.playing = !self.playing;
            }
            WaveformInteraction::Frame => {
                if self.playing {
                    self.playhead_ratio += 0.01;
                    if self.playhead_ratio > 1.0 {
                        self.playhead_ratio = 0.0;
                    }
                    self.zoom_anchor_ratio = self.playhead_ratio;
                }
            }
            WaveformInteraction::Reset => {
                self.viewport = WaveformViewport::full(self.file.frames);
                self.playhead_ratio = 0.5;
            }
        }
    }

    fn handle_wheel(&mut self, delta: Vector2, anchor_ratio: f32) {
        if delta.x.abs() > delta.y.abs() && delta.x.abs() > f32::EPSILON {
            self.pan_by_visible_fraction(delta.x / WAVEFORM_WIDTH as f32);
            return;
        }
        if delta.y < -f32::EPSILON {
            self.zoom_around_anchor(0.82, anchor_ratio);
        } else if delta.y > f32::EPSILON {
            self.zoom_around_anchor(1.22, anchor_ratio);
        }
    }

    fn zoom_around_anchor(&mut self, factor: f32, anchor_ratio: f32) {
        let total = self.file.frames.max(1);
        let current = self.viewport.clamp(total);
        let anchor_ratio = anchor_ratio.clamp(0.0, 1.0);
        let anchor_frame = current.start as f32 + current.visible_frames() as f32 * anchor_ratio;
        let next_visible = ((current.visible_frames() as f32) * factor)
            .round()
            .clamp(MIN_VISIBLE_FRAMES.min(total) as f32, total as f32)
            as usize;
        let start = (anchor_frame - next_visible as f32 * anchor_ratio)
            .round()
            .max(0.0) as usize;
        self.viewport = WaveformViewport {
            start,
            end: start + next_visible,
        }
        .clamp(total);
    }

    fn pan_by_visible_fraction(&mut self, fraction: f32) {
        let total = self.file.frames.max(1);
        let current = self.viewport.clamp(total);
        let delta = (current.visible_frames() as f32 * fraction).round() as isize;
        let start = current.start.saturating_add_signed(delta);
        self.viewport = WaveformViewport {
            start,
            end: start + current.visible_frames(),
        }
        .clamp(total);
    }

    fn set_offset_fraction(&mut self, offset_fraction: f32) {
        let total = self.file.frames.max(1);
        let current = self.viewport.clamp(total);
        let visible = current.visible_frames();
        let free_frames = total.saturating_sub(visible);
        let start = (free_frames as f32 * offset_fraction.clamp(0.0, 1.0)).round() as usize;
        self.viewport = WaveformViewport {
            start,
            end: start + visible,
        }
        .clamp(total);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn synthetic_file(mono_samples: Vec<f32>, sample_rate: u32, channels: usize) -> WaveformFile {
        waveform_file_from_mono_samples(
            PathBuf::from("synthetic-test-waveform"),
            sample_rate,
            channels,
            mono_samples,
        )
    }

    #[test]
    fn stereo_samples_downmix_to_single_mono_stream() {
        let mono = downmix_to_mono(&[1.0, -1.0, 0.6, 0.2], 2, 2);

        assert_eq!(mono, vec![0.0, 0.4]);
    }

    #[test]
    fn gpu_low_band_projection_avoids_thin_cuts_without_flattening_detail() {
        let frame_count = 65_536;
        let low_samples: Vec<f32> = (0..frame_count)
            .map(|index| {
                let carrier = (index as f32 / 34.0).sin();
                let contour = 0.28 + 0.58 * (index as f32 / 12_000.0).sin().abs();
                (carrier * contour).clamp(-1.0, 1.0)
            })
            .collect();
        let bands = [
            WaveformBand::new(low_samples.clone()),
            WaveformBand::new(vec![0.0; frame_count]),
            WaveformBand::new(vec![0.0; frame_count]),
            WaveformBand::new(vec![0.0; frame_count]),
        ];

        let gpu_samples = interleaved_band_samples(&bands);
        let low_gpu_samples: Vec<f32> = gpu_samples
            .chunks_exact(BAND_COUNT)
            .map(|frame| frame[0])
            .collect();
        let extents = shader_projected_band_extents(&low_gpu_samples, 192, 0);
        let isolated_cuts = isolated_cut_count(&extents);
        let isolated_spikes = isolated_spike_count(&extents);
        let roughness = extent_roughness(&extents);
        let max_step = max_adjacent_step(&extents);
        let detail_range = extent_range(&extents);

        assert!(
            isolated_cuts <= 1,
            "low-frequency projection should not contain repeated one-column zero-crossing cuts; extents: {extents:?}"
        );
        assert!(
            isolated_spikes <= 1,
            "low-frequency projection should not contain repeated one-column crest spikes; extents: {extents:?}"
        );
        assert!(
            roughness < 0.012,
            "low-frequency projection should stay continuous at full zoom-out"
        );
        assert!(
            max_step < 0.16,
            "low-frequency projection should not contain long vertical edge jumps"
        );
        assert!(
            detail_range > 0.18,
            "low-frequency projection should retain amplitude contour detail, not flatten into a rectangle"
        );
    }

    fn shader_projected_band_extents(samples: &[f32], columns: usize, _band: usize) -> Vec<f32> {
        let frames_per_pixel = samples.len() as f32 / columns.max(1) as f32;
        (0..columns)
            .map(|column| {
                let peak = smoothed_test_peak(samples, columns, column);
                let left = smoothed_test_peak(samples, columns, column.saturating_sub(1));
                let right = smoothed_test_peak(
                    samples,
                    columns,
                    (column + 1).min(columns.saturating_sub(1)),
                );
                let neighbor = left.max(right);
                let corner_limit =
                    0.24 + (0.095 - 0.24) * smoothstep_test(18.0, 260.0, frames_per_pixel);
                let corner_delta = (peak - neighbor).max(0.0);
                let corner_strength =
                    smoothstep_test(corner_limit, corner_limit * 2.8, corner_delta);
                peak + (neighbor + corner_limit - peak) * corner_strength * 0.82
            })
            .collect()
    }

    fn smoothed_test_peak(samples: &[f32], columns: usize, column: usize) -> f32 {
        weighted_test_projection(samples, columns, column, test_peak_extent)
    }

    fn weighted_test_projection(
        samples: &[f32],
        columns: usize,
        column: usize,
        project: fn(&[f32], usize, usize) -> f32,
    ) -> f32 {
        let taps = [
            (column.saturating_sub(1), 0.24),
            (column, 0.52),
            ((column + 1).min(columns.saturating_sub(1)), 0.24),
        ];
        taps.iter()
            .map(|(tap, weight)| project(samples, columns, *tap) * weight)
            .sum()
    }

    fn test_peak_extent(samples: &[f32], columns: usize, column: usize) -> f32 {
        test_column_samples(samples, columns, column)
            .map(f32::abs)
            .fold(0.0_f32, f32::max)
    }

    fn test_column_samples(
        samples: &[f32],
        columns: usize,
        column: usize,
    ) -> impl Iterator<Item = f32> + '_ {
        let start = column * samples.len() / columns.max(1);
        let end = ((column + 1) * samples.len() / columns.max(1))
            .max(start + 1)
            .min(samples.len());
        let span = end.saturating_sub(start).max(1);
        let step = (span / 40).max(1);
        (start..end).step_by(step).map(|frame| samples[frame])
    }

    fn smoothstep_test(edge0: f32, edge1: f32, value: f32) -> f32 {
        let t = ((value - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
        t * t * (3.0 - 2.0 * t)
    }

    fn isolated_cut_count(extents: &[f32]) -> usize {
        extents
            .windows(3)
            .filter(|window| {
                let neighbor_floor = window[0].min(window[2]);
                neighbor_floor > 0.24 && window[1] < neighbor_floor * 0.54
            })
            .count()
    }

    fn isolated_spike_count(extents: &[f32]) -> usize {
        extents
            .windows(3)
            .filter(|window| {
                let neighbor_ceiling = window[0].max(window[2]);
                window[1] > 0.32 && window[1] > neighbor_ceiling * 1.42
            })
            .count()
    }

    fn extent_range(extents: &[f32]) -> f32 {
        let min = extents.iter().copied().fold(f32::INFINITY, f32::min);
        let max = extents.iter().copied().fold(0.0_f32, f32::max);
        max - min
    }

    fn extent_roughness(extents: &[f32]) -> f32 {
        if extents.len() < 3 {
            return 0.0;
        }
        let total = extents
            .windows(3)
            .map(|window| (window[1] * 2.0 - window[0] - window[2]).abs())
            .sum::<f32>();
        total / (extents.len() - 2) as f32
    }

    fn max_adjacent_step(extents: &[f32]) -> f32 {
        extents
            .windows(2)
            .map(|window| (window[1] - window[0]).abs())
            .fold(0.0_f32, f32::max)
    }

    #[test]
    fn synthetic_waveform_renders_nonblank_mono_image() {
        let mono_samples: Vec<f32> = (0..512)
            .map(|index| ((index as f32 / 16.0).sin() * 0.8).clamp(-1.0, 1.0))
            .collect();
        let file = synthetic_file(mono_samples, 48_000, 2);

        let image = render_waveform_image(&file, WaveformViewport::full(file.frames), 128, 48);

        assert_eq!(image.width, 128);
        assert_eq!(image.height, 48);
        assert!(
            image
                .pixels
                .chunks_exact(4)
                .any(|pixel| pixel[0] > 240 && pixel[1] > 180 && pixel[2] > 150),
            "waveform ridge should produce visible bright pixels"
        );
    }

    #[test]
    fn waveform_widget_paints_cached_body_and_cursor_overlay() {
        let mono_samples: Vec<f32> = (0..512)
            .map(|index| ((index as f32 / 16.0).sin() * 0.8).clamp(-1.0, 1.0))
            .collect();
        let file = Arc::new(synthetic_file(mono_samples, 48_000, 2));
        let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(320.0, 96.0));
        let mut widget =
            WaveformWidget::new(Arc::clone(&file), WaveformViewport::full(file.frames), None);
        let mut primitives = Vec::new();

        assert_eq!(
            widget.handle_input(
                bounds,
                WidgetInput::PointerMove {
                    position: Point::new(160.0, 48.0)
                }
            ),
            None,
            "hover cursor updates should stay local to the widget"
        );
        widget.append_paint(
            &mut primitives,
            bounds,
            &LayoutOutput::default(),
            &ThemeTokens::default(),
        );

        assert!(
            primitives.iter().any(|primitive| matches!(
                primitive,
                PaintPrimitive::GpuSurface(PaintGpuSurface {
                    capabilities,
                    ..
                }) if capabilities.native_hover_cursor.is_some()
                    && capabilities.fast_pointer_move
                    && capabilities.coalesce_vertical_wheel
            )),
            "waveform body should use a GPU signal primitive so zoom does not regenerate pixels"
        );
        assert!(
            !primitives
                .iter()
                .any(|primitive| matches!(primitive, PaintPrimitive::FillPolygon(_)))
        );
        assert!(
            !primitives
                .iter()
                .any(|primitive| matches!(primitive, PaintPrimitive::StrokePolyline(_))),
            "cursor line should be handled by the GPU waveform shader"
        );
    }

    #[test]
    fn zoom_and_pan_keep_viewport_inside_sample() {
        let mono_samples = vec![0.0; 20_000];
        let file = Arc::new(synthetic_file(mono_samples, 48_000, 1));
        let mut app = WaveformApp {
            file,
            viewport: WaveformViewport::full(20_000),
            zoom_anchor_ratio: 0.5,
            playing: false,
            playhead_ratio: 0.5,
        };

        app.zoom_around_anchor(0.5, 0.5);
        assert!(app.viewport.visible_frames() < 20_000);
        app.pan_by_visible_fraction(100.0);
        assert_eq!(app.viewport.end, 20_000);
        app.pan_by_visible_fraction(-100.0);
        assert_eq!(app.viewport.start, 0);
    }

    #[test]
    fn wheel_zoom_and_scrollbar_offset_update_viewport() {
        let mono_samples = vec![0.0; 20_000];
        let file = Arc::new(synthetic_file(mono_samples, 48_000, 1));
        let mut app = WaveformApp {
            file,
            viewport: WaveformViewport::full(20_000),
            zoom_anchor_ratio: 0.5,
            playing: false,
            playhead_ratio: 0.5,
        };

        app.apply_interaction(WaveformInteraction::Wheel {
            delta: Vector2::new(0.0, -40.0),
            anchor_ratio: 0.25,
        });
        assert!(app.viewport.is_zoomed_in(20_000));

        app.apply_interaction(WaveformInteraction::ScrollTo {
            offset_fraction: 1.0,
        });
        assert_eq!(app.viewport.end, 20_000);
    }

    #[test]
    fn zoom_around_anchor_keeps_anchor_frame_at_same_ratio() {
        let mono_samples = vec![0.0; 20_000];
        let file = Arc::new(synthetic_file(mono_samples, 48_000, 1));
        let mut app = WaveformApp {
            file,
            viewport: WaveformViewport {
                start: 2_000,
                end: 12_000,
            },
            zoom_anchor_ratio: 0.5,
            playing: false,
            playhead_ratio: 0.5,
        };
        let ratio = 0.25;
        let before_anchor =
            app.viewport.start as f32 + app.viewport.visible_frames() as f32 * ratio;

        app.zoom_around_anchor(0.5, ratio);

        let after_anchor = app.viewport.start as f32 + app.viewport.visible_frames() as f32 * ratio;
        assert!((before_anchor - after_anchor).abs() <= 1.0);
    }

    #[test]
    fn summary_stats_match_raw_range_stats() {
        let samples: Vec<f32> = (0..4096)
            .map(|index| ((index as f32 / 13.0).sin() * 0.7).clamp(-1.0, 1.0))
            .collect();
        let summary = WaveformSummary::from_samples(&samples);

        let summarized = summary.stats(&samples, 37, 3901);
        let raw = band_stats(&samples, 37, 3901);
        assert!((summarized.peak - raw.peak).abs() < 0.000_001);
        assert!((summarized.rms - raw.rms).abs() < 0.000_001);
    }

    #[test]
    fn default_waveform_source_uses_synthetic_signal_without_input_path() {
        let file = load_waveform_source(None).expect("synthetic waveform should load");

        assert!(file.sample_rate > 0);
        assert!(!file.mono_samples.is_empty());
        assert_eq!(file.frames, file.mono_samples.len());
        let image = render_waveform_image(&file, WaveformViewport::full(file.frames), 320, 96);
        assert_eq!(image.width, 320);
    }
}
