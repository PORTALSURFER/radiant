//! Load one WAV file and display it as an interactive mono waveform view.

use radiant::prelude as ui;
use radiant::{
    gui::types::{Point, Rect, Rgba8, Vector2},
    layout::LayoutOutput,
    runtime::{
        GpuHoverCursor, GpuSurfaceCapabilities, GpuSurfaceContent, PaintGpuSurface, PaintPrimitive,
    },
    theme::ThemeTokens,
    widgets::{
        FocusBehavior, PaintBounds, ScrollbarAxis, ScrollbarMessage, ScrollbarWidget, Widget,
        WidgetCommon, WidgetInput, WidgetOutput, WidgetSizing,
    },
};
use std::sync::Arc;

const WAVEFORM_WIDTH: usize = 1200;
const WAVEFORM_HEIGHT: usize = 320;

#[path = "waveform_view/source.rs"]
mod source;
use source::*;
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
    use std::path::PathBuf;

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
