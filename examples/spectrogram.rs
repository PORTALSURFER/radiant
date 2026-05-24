//! Realtime spectrogram sandbox for DAW-style GUI interaction.

use radiant::prelude::*;
use radiant::{
    runtime::{PaintFillRect, PaintStrokeRect},
    widgets::PaintBounds,
};

use std::collections::VecDeque;

const SPECTROGRAM_WIDGET_ID: u64 = 80;
const STATUS_WIDGET_ID: u64 = 81;
const COLUMNS: usize = 96;
const BINS: usize = 48;
const MIN_FREQ_HZ: f32 = 40.0;
const MAX_FREQ_HZ: f32 = 18_000.0;
const DATA_SOURCE_NOTE: &str = "without_dsp";

#[derive(Clone, Debug)]
struct SpectrogramState {
    running: bool,
    frame: u64,
    intensity: f32,
    speed: u32,
    columns: VecDeque<SpectralColumn>,
}

impl Default for SpectrogramState {
    fn default() -> Self {
        let mut state = Self {
            running: true,
            frame: 0,
            intensity: 0.82,
            speed: 2,
            columns: VecDeque::with_capacity(COLUMNS),
        };
        for _ in 0..COLUMNS {
            state.push_next_column();
        }
        state
    }
}

impl SpectrogramState {
    fn tick(&mut self) {
        if !self.running {
            return;
        }
        for _ in 0..self.speed {
            self.push_next_column();
        }
    }

    fn push_next_column(&mut self) {
        self.frame = self.frame.saturating_add(1);
        if self.columns.len() == COLUMNS {
            self.columns.pop_front();
        }
        self.columns
            .push_back(generate_spectral_column(self.frame, self.intensity));
    }

    fn reset(&mut self) {
        self.frame = 0;
        self.columns.clear();
        for _ in 0..COLUMNS {
            self.push_next_column();
        }
    }

    fn status(&self) -> String {
        let transport = if self.running { "running" } else { "paused" };
        format!(
            "{transport} | frame {} | speed {}x | synthetic GUI data",
            self.frame, self.speed
        )
    }

    fn adjust_intensity(&mut self, delta: f32) {
        self.intensity = (self.intensity + delta).clamp(0.35, 1.35);
        self.reset();
    }

    fn cycle_speed(&mut self) {
        self.speed = match self.speed {
            1 => 2,
            2 => 4,
            _ => 1,
        };
    }
}

#[derive(Clone, Debug, PartialEq)]
struct SpectralColumn {
    bins: Vec<f32>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SpectrogramMessage {
    Frame,
    ToggleRun,
    Reset,
    IncreaseIntensity,
    DecreaseIntensity,
    CycleSpeed,
}

fn main() -> radiant::Result {
    radiant::app(SpectrogramState::default())
        .title("Radiant Realtime Spectrogram")
        .size(980, 560)
        .min_size(720, 420)
        .view(project_surface)
        .animation(|state| state.running)
        .on_frame(|| SpectrogramMessage::Frame)
        .update(update)
        .run()
}

fn project_surface(state: &mut SpectrogramState) -> View<SpectrogramMessage> {
    column([
        row([
            text("Realtime Spectrogram").height(30.0).fill_width(),
            button(if state.running { "Pause" } else { "Run" })
                .primary()
                .message(SpectrogramMessage::ToggleRun)
                .size(88.0, 30.0),
            button("Reset")
                .subtle()
                .message(SpectrogramMessage::Reset)
                .size(82.0, 30.0),
        ])
        .fill_width()
        .spacing(10.0),
        custom_widget_mapped(
            SpectrogramWidget::new(state.columns.iter().cloned().collect(), state.frame),
            |message| message,
        )
        .id(SPECTROGRAM_WIDGET_ID)
        .height(320.0)
        .fill_width(),
        row([
            button("- Energy")
                .subtle()
                .message(SpectrogramMessage::DecreaseIntensity)
                .size(104.0, 30.0),
            button("+ Energy")
                .primary()
                .message(SpectrogramMessage::IncreaseIntensity)
                .size(104.0, 30.0),
            button(format!("Speed {}x", state.speed))
                .subtle()
                .message(SpectrogramMessage::CycleSpeed)
                .size(112.0, 30.0),
            text(format!("{:.0} Hz", MIN_FREQ_HZ))
                .height(30.0)
                .fill_width(),
            text(state.status())
                .id(STATUS_WIDGET_ID)
                .height(30.0)
                .fill_width(),
        ])
        .fill_width()
        .spacing(10.0),
        grid_with_gaps(
            [
                stat_tile("Bins", BINS.to_string()),
                stat_tile("History", format!("{COLUMNS} columns")),
                stat_tile("Source", DATA_SOURCE_NOTE),
                stat_tile(
                    "Range",
                    format!("{:.0} Hz - {:.0} kHz", MIN_FREQ_HZ, MAX_FREQ_HZ / 1_000.0),
                ),
            ],
            4,
            10.0,
            10.0,
        )
        .fill_width(),
    ])
    .style(WidgetStyle::default())
    .padding(16.0)
    .spacing(12.0)
    .fill()
}

fn stat_tile(label: impl Into<String>, value: impl Into<String>) -> View<SpectrogramMessage> {
    column([
        text(label.into()).height(22.0).fill_width(),
        text(value.into()).height(24.0).fill_width(),
    ])
    .style(WidgetStyle {
        tone: WidgetTone::Neutral,
        prominence: WidgetProminence::Subtle,
    })
    .padding(10.0)
    .spacing(4.0)
    .height(76.0)
    .fill_width()
}

fn update(state: &mut SpectrogramState, message: SpectrogramMessage) {
    match message {
        SpectrogramMessage::Frame => state.tick(),
        SpectrogramMessage::ToggleRun => {
            state.running = !state.running;
        }
        SpectrogramMessage::Reset => {
            state.running = false;
            state.reset();
        }
        SpectrogramMessage::IncreaseIntensity => state.adjust_intensity(0.08),
        SpectrogramMessage::DecreaseIntensity => state.adjust_intensity(-0.08),
        SpectrogramMessage::CycleSpeed => state.cycle_speed(),
    }
}

#[derive(Clone, Debug)]
struct SpectrogramWidget {
    common: WidgetCommon,
    columns: Vec<SpectralColumn>,
    frame: u64,
    hover_column: Option<usize>,
    hover_position: Option<Point>,
}

impl SpectrogramWidget {
    fn new(columns: Vec<SpectralColumn>, frame: u64) -> Self {
        let mut common = WidgetCommon::new(
            0,
            WidgetSizing::new(Vector2::new(560.0, 280.0), Vector2::new(940.0, 320.0)),
        );
        common.focus = FocusBehavior::Pointer;
        common.paint.bounds = PaintBounds::ClipToRect;
        common.paint.paints_focus = false;
        common.paint.paints_state_layers = false;
        Self {
            common,
            columns,
            frame,
            hover_column: None,
            hover_position: None,
        }
    }

    fn plot_rect(&self, bounds: Rect) -> Rect {
        Rect::from_min_max(
            Point::new(bounds.min.x + 54.0, bounds.min.y + 18.0),
            Point::new(bounds.max.x - 18.0, bounds.max.y - 36.0),
        )
    }

    fn column_at_position(&self, plot: Rect, position: Point) -> Option<usize> {
        if !plot.contains(position) || self.columns.is_empty() {
            return None;
        }
        let ratio = ((position.x - plot.min.x) / plot.width().max(1.0)).clamp(0.0, 0.999);
        Some((ratio * self.columns.len() as f32).floor() as usize)
    }
}

impl Widget for SpectrogramWidget {
    fn common(&self) -> &WidgetCommon {
        &self.common
    }

    fn common_mut(&mut self) -> &mut WidgetCommon {
        &mut self.common
    }

    fn handle_input(&mut self, bounds: Rect, input: WidgetInput) -> Option<WidgetOutput> {
        match input {
            WidgetInput::PointerMove { position } => {
                self.common.state.hovered = bounds.contains(position);
                let plot = self.plot_rect(bounds);
                self.hover_column = self.column_at_position(plot, position);
                self.hover_position = plot.contains(position).then_some(position);
                None
            }
            WidgetInput::PointerDrop { .. } => {
                self.hover_column = None;
                self.hover_position = None;
                None
            }
            WidgetInput::FocusChanged(focused) => {
                self.common.state.focused = focused;
                None
            }
            _ => None,
        }
    }

    fn prefers_pointer_move_paint_only(&self) -> bool {
        true
    }

    fn synchronize_from_previous(&mut self, previous: &dyn Widget) {
        if let Some(previous) = previous.as_any().downcast_ref::<Self>() {
            self.common.state = previous.common.state;
            self.hover_column = previous.hover_column;
            self.hover_position = previous.hover_position;
        }
    }

    fn append_paint(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        bounds: Rect,
        _layout: &LayoutOutput,
        theme: &ThemeTokens,
    ) {
        let plot = self.plot_rect(bounds);
        push_rect(primitives, self.common.id, bounds, theme.bg_secondary);
        push_rect(primitives, self.common.id, plot, rgba(7, 11, 18, 255));
        self.append_spectrogram_cells(primitives, plot);
        self.append_grid(primitives, plot, theme);
        push_stroke(primitives, self.common.id, plot, theme.border_emphasis, 1.0);
        self.append_labels(primitives, plot, theme);
    }

    fn append_runtime_overlay_paint(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        bounds: Rect,
        _layout: &LayoutOutput,
        theme: &ThemeTokens,
    ) {
        let Some(position) = self.hover_position else {
            return;
        };
        let plot = self.plot_rect(bounds);
        if plot.contains(position) {
            self.append_hover(primitives, plot, position.x, theme);
        }
    }
}

impl SpectrogramWidget {
    fn append_spectrogram_cells(&self, primitives: &mut Vec<PaintPrimitive>, plot: Rect) {
        if self.columns.is_empty() {
            return;
        }
        let cell_width = plot.width() / self.columns.len() as f32;
        let cell_height = plot.height() / BINS as f32;
        for (column_index, column) in self.columns.iter().enumerate() {
            let x0 = plot.min.x + column_index as f32 * cell_width;
            let x1 = if column_index + 1 == self.columns.len() {
                plot.max.x
            } else {
                x0 + cell_width + 0.5
            };
            for (bin_index, energy) in column.bins.iter().enumerate() {
                let y1 = plot.max.y - bin_index as f32 * cell_height;
                let y0 = (y1 - cell_height - 0.5).max(plot.min.y);
                push_rect(
                    primitives,
                    self.common.id,
                    Rect::from_min_max(Point::new(x0, y0), Point::new(x1, y1)),
                    spectrogram_color(*energy),
                );
            }
        }
    }

    fn append_grid(&self, primitives: &mut Vec<PaintPrimitive>, plot: Rect, theme: &ThemeTokens) {
        for fraction in [0.25, 0.5, 0.75] {
            let y = plot.max.y - plot.height() * fraction;
            push_rect(
                primitives,
                self.common.id,
                Rect::from_min_max(Point::new(plot.min.x, y), Point::new(plot.max.x, y + 1.0)),
                translucent(theme.grid_soft, 150),
            );
        }
        for fraction in [0.25, 0.5, 0.75] {
            let x = plot.min.x + plot.width() * fraction;
            push_rect(
                primitives,
                self.common.id,
                Rect::from_min_max(Point::new(x, plot.min.y), Point::new(x + 1.0, plot.max.y)),
                translucent(theme.grid_soft, 120),
            );
        }
    }

    fn append_hover(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        plot: Rect,
        x: f32,
        theme: &ThemeTokens,
    ) {
        push_rect(
            primitives,
            self.common.id,
            Rect::from_min_max(Point::new(x, plot.min.y), Point::new(x + 2.0, plot.max.y)),
            translucent(theme.accent_mint, 180),
        );
    }

    fn append_labels(&self, primitives: &mut Vec<PaintPrimitive>, plot: Rect, theme: &ThemeTokens) {
        for (label, ratio) in [
            ("18k", 1.0),
            ("8k", 0.78),
            ("2k", 0.55),
            ("500", 0.32),
            ("40", 0.0),
        ] {
            let y = plot.max.y - plot.height() * ratio;
            push_text(
                primitives,
                self.common.id,
                label,
                Rect::from_min_max(
                    Point::new(plot.min.x - 44.0, y - 9.0),
                    Point::new(plot.min.x - 6.0, y + 11.0),
                ),
                theme.text_muted,
                PaintTextAlign::Right,
            );
        }
        push_text(
            primitives,
            self.common.id,
            format!("frame {}", self.frame),
            Rect::from_min_max(
                Point::new(plot.max.x - 118.0, plot.max.y + 8.0),
                Point::new(plot.max.x, plot.max.y + 28.0),
            ),
            theme.text_muted,
            PaintTextAlign::Right,
        );
        push_text(
            primitives,
            self.common.id,
            "synthetic realtime spectrum",
            Rect::from_min_max(
                Point::new(plot.min.x, plot.max.y + 8.0),
                Point::new(plot.min.x + 180.0, plot.max.y + 28.0),
            ),
            theme.text_muted,
            PaintTextAlign::Left,
        );
    }
}

fn generate_spectral_column(frame: u64, intensity: f32) -> SpectralColumn {
    let mut bins = Vec::with_capacity(BINS);
    let frame_phase = frame as f32 * 0.043;
    let low_sweep = 0.20 + 0.14 * (frame_phase * 0.71).sin();
    let high_sweep = 0.68 + 0.18 * (frame_phase * 0.43).cos();
    for bin in 0..BINS {
        let ratio = bin as f32 / (BINS - 1) as f32;
        let low_band = gaussian(ratio, low_sweep, 0.055);
        let high_band = gaussian(ratio, high_sweep, 0.075) * 0.72;
        let harmonic = ((ratio * 18.0 + frame_phase * 2.4).sin() * 0.5 + 0.5) * 0.24;
        let noise = deterministic_noise(frame, bin as u64) * 0.22;
        let rolloff = (1.0 - ratio * 0.42).max(0.28);
        bins.push(
            ((low_band + high_band + harmonic + noise) * rolloff * intensity).clamp(0.0, 1.0),
        );
    }
    SpectralColumn { bins }
}

fn gaussian(value: f32, center: f32, width: f32) -> f32 {
    let delta = value - center;
    (-(delta * delta) / (2.0 * width * width)).exp()
}

fn deterministic_noise(frame: u64, bin: u64) -> f32 {
    let mut value = frame
        .wrapping_mul(0x9E37_79B9_7F4A_7C15)
        .wrapping_add(bin.wrapping_mul(0xBF58_476D_1CE4_E5B9));
    value ^= value >> 30;
    value = value.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    value ^= value >> 27;
    value = value.wrapping_mul(0x94D0_49BB_1331_11EB);
    value ^= value >> 31;
    ((value >> 40) as f32) / ((1_u64 << 24) as f32)
}

fn spectrogram_color(energy: f32) -> Rgba8 {
    let value = energy.clamp(0.0, 1.0);
    let cold = rgba(10, 18, 30, 255);
    let blue = rgba(16, 74, 118, 255);
    let green = rgba(36, 168, 116, 255);
    let amber = rgba(246, 176, 64, 255);
    let hot = rgba(255, 240, 184, 255);
    if value < 0.28 {
        lerp_color(cold, blue, value / 0.28)
    } else if value < 0.58 {
        lerp_color(blue, green, (value - 0.28) / 0.30)
    } else if value < 0.84 {
        lerp_color(green, amber, (value - 0.58) / 0.26)
    } else {
        lerp_color(amber, hot, (value - 0.84) / 0.16)
    }
}

fn lerp_color(a: Rgba8, b: Rgba8, t: f32) -> Rgba8 {
    let t = t.clamp(0.0, 1.0);
    rgba(
        lerp_channel(a.r, b.r, t),
        lerp_channel(a.g, b.g, t),
        lerp_channel(a.b, b.b, t),
        255,
    )
}

fn lerp_channel(a: u8, b: u8, t: f32) -> u8 {
    (a as f32 + (b as f32 - a as f32) * t).round() as u8
}

fn rgba(r: u8, g: u8, b: u8, a: u8) -> Rgba8 {
    Rgba8 { r, g, b, a }
}

fn push_rect(primitives: &mut Vec<PaintPrimitive>, widget_id: u64, rect: Rect, color: Rgba8) {
    primitives.push(PaintPrimitive::FillRect(PaintFillRect {
        widget_id,
        rect,
        color,
    }));
}

fn push_stroke(
    primitives: &mut Vec<PaintPrimitive>,
    widget_id: u64,
    rect: Rect,
    color: Rgba8,
    width: f32,
) {
    primitives.push(PaintPrimitive::StrokeRect(PaintStrokeRect {
        widget_id,
        rect,
        color,
        width,
    }));
}

fn push_text(
    primitives: &mut Vec<PaintPrimitive>,
    widget_id: u64,
    text: impl Into<String>,
    rect: Rect,
    color: Rgba8,
    align: PaintTextAlign,
) {
    primitives.push(PaintPrimitive::Text(PaintTextRun {
        widget_id,
        text: text.into().into(),
        rect,
        font_size: 12.0,
        baseline: Some(16.0),
        color,
        align,
        wrap: TextWrap::None,
    }));
}

fn translucent(mut color: Rgba8, alpha: u8) -> Rgba8 {
    color.a = alpha;
    color
}

#[cfg(test)]
mod tests {
    use super::*;
    use radiant::runtime::{RuntimeBridge, SurfaceRuntime};

    #[test]
    fn spectrogram_tick_scrolls_synthetic_columns_without_dsp() {
        let mut state = SpectrogramState::default();
        let initial_frame = state.frame;
        let first_column = state.columns.front().cloned();

        state.tick();

        assert_eq!(state.frame, initial_frame + state.speed as u64);
        assert_eq!(state.columns.len(), COLUMNS);
        assert_ne!(state.columns.front(), first_column.as_ref());
    }

    #[test]
    fn spectrogram_widget_paints_heatmap_grid_and_labels() {
        let state = SpectrogramState::default();
        let widget = SpectrogramWidget::new(state.columns.iter().cloned().collect(), state.frame);
        let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(760.0, 320.0));
        let mut primitives = Vec::new();

        widget.append_paint(
            &mut primitives,
            bounds,
            &LayoutOutput::default(),
            &ThemeTokens::default(),
        );

        let fill_count = primitives
            .iter()
            .filter(|primitive| matches!(primitive, PaintPrimitive::FillRect(_)))
            .count();
        assert!(
            fill_count >= COLUMNS * BINS,
            "spectrogram should paint one heatmap cell per visible bin"
        );
        assert!(
            primitives
                .iter()
                .any(|primitive| matches!(primitive, PaintPrimitive::StrokeRect(_))),
            "spectrogram should paint plot chrome"
        );
        assert!(
            primitives
                .iter()
                .any(|primitive| matches!(primitive, PaintPrimitive::Text(text) if text.text.as_str().contains("synthetic realtime spectrum"))),
            "spectrogram should label its GUI-only synthetic source"
        );
    }

    #[test]
    fn spectrogram_runtime_frame_messages_advance_visual_state() {
        let bridge = spectrogram_test_bridge(SpectrogramState::default());
        let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(980.0, 560.0));
        let initial_status = status_text(&runtime);

        assert!(runtime.bridge_mut().needs_animation());
        assert!(runtime.bridge_mut().queue_animation_frame());
        let outcome = runtime.drain_runtime_messages();

        assert_eq!(outcome.messages_dispatched, 1);
        assert_ne!(status_text(&runtime), initial_status);
    }

    #[test]
    fn spectrogram_hover_uses_paint_only_widget_local_state() {
        let mut widget = SpectrogramWidget::new(
            SpectrogramState::default().columns.into_iter().collect(),
            96,
        );
        let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(760.0, 320.0));
        let plot = widget.plot_rect(bounds);

        let output = widget.handle_input(
            bounds,
            WidgetInput::PointerMove {
                position: Point::new(plot.min.x + plot.width() * 0.5, plot.center().y),
            },
        );

        assert!(output.is_none());
        assert_eq!(widget.hover_column, Some(COLUMNS / 2));
        assert!(
            widget.prefers_pointer_move_paint_only(),
            "spectrogram hover should stay on the runtime-local paint-only path"
        );
        let mut overlay = Vec::new();
        widget.append_runtime_overlay_paint(
            &mut overlay,
            bounds,
            &LayoutOutput::default(),
            &ThemeTokens::default(),
        );
        assert!(
            overlay
                .iter()
                .any(|primitive| matches!(primitive, PaintPrimitive::FillRect(_))),
            "hover cursor should paint as a lightweight runtime overlay"
        );
    }

    #[test]
    fn spectrogram_runtime_hover_does_not_refresh_surface() {
        let bridge = spectrogram_test_bridge(SpectrogramState::default());
        let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(980.0, 560.0));
        let bounds = runtime.layout().rects[&SPECTROGRAM_WIDGET_ID];
        let first = runtime
            .dispatch_pointer_move_with_outcome(Point::new(bounds.min.x + 80.0, bounds.center().y));
        let second = runtime.dispatch_pointer_move_with_outcome(Point::new(
            bounds.min.x + 140.0,
            bounds.center().y,
        ));

        assert!(first.needs_scene_rebuild());
        assert!(second.paint_only_requested);
        assert!(
            !second.needs_scene_rebuild(),
            "stable spectrogram hover should avoid reprojection and full scene rebuilds"
        );
    }

    fn spectrogram_test_bridge(state: SpectrogramState) -> impl RuntimeBridge<SpectrogramMessage> {
        radiant::app(state)
            .view(project_surface)
            .animation(|state| state.running)
            .on_frame(|| SpectrogramMessage::Frame)
            .update(update)
            .into_bridge()
    }

    fn status_text<Bridge>(runtime: &SurfaceRuntime<Bridge, SpectrogramMessage>) -> String
    where
        Bridge: RuntimeBridge<SpectrogramMessage>,
    {
        runtime
            .paint_plan(&ThemeTokens::default())
            .primitives
            .iter()
            .find_map(|primitive| match primitive {
                PaintPrimitive::Text(text) if text.widget_id == STATUS_WIDGET_ID => {
                    Some(text.text.as_str().to_string())
                }
                _ => None,
            })
            .expect("status text should be painted")
    }
}
