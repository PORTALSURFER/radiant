//! Graphical EQ editor sandbox for plugin-style GUI interaction.

use radiant::prelude::*;
use radiant::{
    runtime::{PaintFillPolygon, PaintStrokePolyline},
    widgets::PaintBounds,
};

use std::sync::Arc;

const EQ_WIDGET_ID: u64 = 70;
const STATUS_WIDGET_ID: u64 = 71;
const MIN_FREQ_HZ: f32 = 20.0;
const MAX_FREQ_HZ: f32 = 20_000.0;
const MIN_GAIN_DB: f32 = -24.0;
const MAX_GAIN_DB: f32 = 24.0;
const HANDLE_SIZE: f32 = 12.0;

#[derive(Clone, Debug)]
struct EqEditorState {
    bypassed: bool,
    analyzer: bool,
    selected_band: u32,
    bands: Vec<EqBand>,
    status: String,
}

impl Default for EqEditorState {
    fn default() -> Self {
        Self {
            bypassed: false,
            analyzer: true,
            selected_band: 2,
            bands: vec![
                EqBand::new(1, "HP", 80.0, 0.0, 0.70, EqBandKind::HighPass),
                EqBand::new(2, "Bell", 420.0, 4.5, 1.10, EqBandKind::Bell),
                EqBand::new(3, "Bell", 2_400.0, -5.0, 1.35, EqBandKind::Bell),
                EqBand::new(4, "Shelf", 10_000.0, 3.0, 0.85, EqBandKind::HighShelf),
            ],
            status: "ready".into(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct EqBand {
    id: u32,
    label: &'static str,
    freq_hz: f32,
    gain_db: f32,
    q: f32,
    kind: EqBandKind,
    enabled: bool,
}

impl EqBand {
    fn new(
        id: u32,
        label: &'static str,
        freq_hz: f32,
        gain_db: f32,
        q: f32,
        kind: EqBandKind,
    ) -> Self {
        Self {
            id,
            label,
            freq_hz,
            gain_db,
            q,
            kind,
            enabled: true,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum EqBandKind {
    Bell,
    HighPass,
    HighShelf,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum EqMessage {
    Editor(EqEditorMessage),
    ToggleBypass,
    ToggleAnalyzer,
    ToggleSelectedBand,
    NudgeGain(f32),
    NudgeQ(f32),
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum EqEditorMessage {
    SelectBand(u32),
    MoveBand { id: u32, freq_hz: f32, gain_db: f32 },
}

fn main() -> radiant::Result {
    radiant::app(EqEditorState::default())
        .title("Radiant Graphical EQ Editor")
        .size(920, 560)
        .min_size(680, 430)
        .view(project_surface)
        .update(update)
        .run()
}

fn project_surface(state: &mut EqEditorState) -> View<EqMessage> {
    let selected = selected_band(state).copied();

    column([
        row([
            text("Graphical EQ").height(30.0).fill_width(),
            toggle("Analyzer", state.analyzer)
                .message(|_| EqMessage::ToggleAnalyzer)
                .size(118.0, 30.0),
            toggle("Bypass", state.bypassed)
                .message(|_| EqMessage::ToggleBypass)
                .size(104.0, 30.0),
        ])
        .fill_width()
        .spacing(10.0),
        custom_widget_mapped(
            EqEditorWidget::new(state.bands.clone(), state.selected_band, state.analyzer),
            EqMessage::Editor,
        )
        .id(EQ_WIDGET_ID)
        .height(300.0)
        .fill_width(),
        row([
            selected_band_tile(selected),
            button("- Gain")
                .subtle()
                .message(EqMessage::NudgeGain(-0.5))
                .size(90.0, 30.0),
            button("+ Gain")
                .primary()
                .message(EqMessage::NudgeGain(0.5))
                .size(90.0, 30.0),
            button("- Q")
                .subtle()
                .message(EqMessage::NudgeQ(-0.05))
                .size(72.0, 30.0),
            button("+ Q")
                .primary()
                .message(EqMessage::NudgeQ(0.05))
                .size(72.0, 30.0),
            button("Band")
                .subtle()
                .message(EqMessage::ToggleSelectedBand)
                .size(78.0, 30.0),
            text(state.status.clone())
                .id(STATUS_WIDGET_ID)
                .height(30.0)
                .fill_width(),
        ])
        .fill_width()
        .spacing(10.0),
        grid_with_gaps(
            state
                .bands
                .iter()
                .map(|band| band_summary(*band, band.id == state.selected_band))
                .collect::<Vec<_>>(),
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

fn selected_band_tile(selected: Option<EqBand>) -> View<EqMessage> {
    let label = selected
        .map(|band| {
            format!(
                "{} {:.0} Hz / {:+.1} dB",
                band.label, band.freq_hz, band.gain_db
            )
        })
        .unwrap_or_else(|| "No band".into());
    text(label).height(30.0).fill_width()
}

fn band_summary(band: EqBand, selected: bool) -> View<EqMessage> {
    let state = if band.enabled { "on" } else { "off" };
    let style = if selected {
        WidgetStyle {
            tone: WidgetTone::Accent,
            prominence: WidgetProminence::Subtle,
        }
    } else {
        WidgetStyle {
            tone: WidgetTone::Neutral,
            prominence: WidgetProminence::Subtle,
        }
    };

    column([
        text(format!("{} {}", band.id, band.label))
            .height(22.0)
            .fill_width(),
        text(format!("{:.0} Hz", band.freq_hz))
            .height(22.0)
            .fill_width(),
        text(format!(
            "{:+.1} dB / Q {:.2} / {state}",
            band.gain_db, band.q
        ))
        .height(22.0)
        .fill_width(),
    ])
    .style(style)
    .padding(10.0)
    .spacing(4.0)
    .height(92.0)
    .fill_width()
}

fn update(state: &mut EqEditorState, message: EqMessage) {
    match message {
        EqMessage::Editor(EqEditorMessage::SelectBand(id)) => {
            state.selected_band = id;
            state.status = format!("selected band {id}");
        }
        EqMessage::Editor(EqEditorMessage::MoveBand {
            id,
            freq_hz,
            gain_db,
        }) => {
            state.selected_band = id;
            if let Some(band) = state.bands.iter_mut().find(|band| band.id == id) {
                band.freq_hz = freq_hz.clamp(MIN_FREQ_HZ, MAX_FREQ_HZ);
                band.gain_db = gain_db.clamp(MIN_GAIN_DB, MAX_GAIN_DB);
                state.status = format!(
                    "band {id} moved to {:.0} Hz / {:+.1} dB",
                    band.freq_hz, band.gain_db
                );
            }
        }
        EqMessage::ToggleBypass => {
            state.bypassed = !state.bypassed;
            state.status = if state.bypassed {
                "bypassed".into()
            } else {
                "active".into()
            };
        }
        EqMessage::ToggleAnalyzer => {
            state.analyzer = !state.analyzer;
            state.status = if state.analyzer {
                "analyzer overlay visible".into()
            } else {
                "analyzer overlay hidden".into()
            };
        }
        EqMessage::ToggleSelectedBand => {
            if let Some(band) = selected_band_mut(state) {
                band.enabled = !band.enabled;
                state.status = format!(
                    "band {} {}",
                    band.id,
                    if band.enabled { "enabled" } else { "disabled" }
                );
            }
        }
        EqMessage::NudgeGain(delta) => {
            if let Some(band) = selected_band_mut(state) {
                band.gain_db = (band.gain_db + delta).clamp(MIN_GAIN_DB, MAX_GAIN_DB);
                state.status = format!("band {} gain {:+.1} dB", band.id, band.gain_db);
            }
        }
        EqMessage::NudgeQ(delta) => {
            if let Some(band) = selected_band_mut(state) {
                band.q = (band.q + delta).clamp(0.20, 8.0);
                state.status = format!("band {} Q {:.2}", band.id, band.q);
            }
        }
    }
}

fn selected_band(state: &EqEditorState) -> Option<&EqBand> {
    state
        .bands
        .iter()
        .find(|band| band.id == state.selected_band)
}

fn selected_band_mut(state: &mut EqEditorState) -> Option<&mut EqBand> {
    state
        .bands
        .iter_mut()
        .find(|band| band.id == state.selected_band)
}

#[derive(Clone, Debug)]
struct EqEditorWidget {
    common: WidgetCommon,
    bands: Vec<EqBand>,
    selected_band: u32,
    analyzer: bool,
    hover_band: Option<u32>,
    drag_band: Option<u32>,
    hover_position: Option<Point>,
}

impl EqEditorWidget {
    fn new(bands: Vec<EqBand>, selected_band: u32, analyzer: bool) -> Self {
        let mut common = WidgetCommon::new(
            0,
            WidgetSizing::new(Vector2::new(520.0, 260.0), Vector2::new(880.0, 300.0)),
        );
        common.focus = FocusBehavior::Pointer;
        common.paint.bounds = PaintBounds::ClipToRect;
        common.paint.paints_focus = false;
        common.paint.paints_state_layers = false;
        Self {
            common,
            bands,
            selected_band,
            analyzer,
            hover_band: None,
            drag_band: None,
            hover_position: None,
        }
    }

    fn plot_rect(&self, bounds: Rect) -> Rect {
        Rect::from_min_max(
            Point::new(bounds.min.x + 48.0, bounds.min.y + 22.0),
            Point::new(bounds.max.x - 18.0, bounds.max.y - 34.0),
        )
    }

    fn handle_center(&self, plot: Rect, band: EqBand) -> Point {
        Point::new(
            x_for_freq(plot, band.freq_hz),
            y_for_gain(plot, band.gain_db),
        )
    }

    fn band_at_position(&self, plot: Rect, position: Point) -> Option<u32> {
        self.bands
            .iter()
            .filter(|band| band.enabled)
            .map(|band| {
                let center = self.handle_center(plot, *band);
                let dx = center.x - position.x;
                let dy = center.y - position.y;
                (band.id, dx * dx + dy * dy)
            })
            .filter(|(_, distance)| *distance <= 18.0 * 18.0)
            .min_by(|(_, a), (_, b)| a.total_cmp(b))
            .map(|(id, _)| id)
    }
}

impl Widget for EqEditorWidget {
    fn common(&self) -> &WidgetCommon {
        &self.common
    }

    fn common_mut(&mut self) -> &mut WidgetCommon {
        &mut self.common
    }

    fn handle_input(&mut self, bounds: Rect, input: WidgetInput) -> Option<WidgetOutput> {
        let plot = self.plot_rect(bounds);
        match input {
            WidgetInput::PointerMove { position } => {
                self.common.state.hovered = bounds.contains(position);
                self.hover_position = bounds.contains(position).then_some(position);
                if let Some(id) = self.drag_band {
                    return Some(WidgetOutput::custom(EqEditorMessage::MoveBand {
                        id,
                        freq_hz: freq_for_x(plot, position.x),
                        gain_db: gain_for_y(plot, position.y),
                    }));
                }
                self.hover_band = self.band_at_position(plot, position);
                None
            }
            WidgetInput::PointerPress {
                position,
                button: PointerButton::Primary,
                ..
            } if bounds.contains(position) => {
                let id = self
                    .band_at_position(plot, position)
                    .unwrap_or(self.selected_band);
                self.drag_band = Some(id);
                self.selected_band = id;
                self.hover_band = Some(id);
                Some(WidgetOutput::custom(EqEditorMessage::SelectBand(id)))
            }
            WidgetInput::PointerRelease {
                position,
                button: PointerButton::Primary,
                ..
            }
            | WidgetInput::PointerDrop {
                position,
                button: PointerButton::Primary,
                ..
            } => {
                let drag = self.drag_band.take();
                self.hover_band = bounds
                    .contains(position)
                    .then(|| self.band_at_position(plot, position))
                    .flatten();
                drag.map(|id| {
                    WidgetOutput::custom(EqEditorMessage::MoveBand {
                        id,
                        freq_hz: freq_for_x(plot, position.x),
                        gain_db: gain_for_y(plot, position.y),
                    })
                })
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
            self.hover_band = previous.hover_band;
            self.drag_band = previous.drag_band;
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
        push_rect(primitives, self.common.id, plot, theme.surface_base);
        push_stroke(primitives, self.common.id, plot, theme.border_emphasis, 1.0);
        self.append_grid(primitives, plot, theme);
        if self.analyzer {
            self.append_analyzer(primitives, plot, theme);
        }
        self.append_curve(primitives, plot, theme);
        self.append_band_handles(primitives, plot, theme);
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
        if !bounds.contains(position) {
            return;
        }
        let plot = self.plot_rect(bounds);
        if plot.contains(position) {
            push_rect(
                primitives,
                self.common.id,
                Rect::from_min_max(
                    Point::new(position.x, plot.min.y),
                    Point::new(position.x + 1.0, plot.max.y),
                ),
                translucent(theme.text_muted, 110),
            );
            push_rect(
                primitives,
                self.common.id,
                Rect::from_min_max(
                    Point::new(plot.min.x, position.y),
                    Point::new(plot.max.x, position.y + 1.0),
                ),
                translucent(theme.text_muted, 80),
            );
        }
    }
}

impl EqEditorWidget {
    fn append_grid(&self, primitives: &mut Vec<PaintPrimitive>, plot: Rect, theme: &ThemeTokens) {
        for freq in [
            20.0, 50.0, 100.0, 200.0, 500.0, 1_000.0, 2_000.0, 5_000.0, 10_000.0, 20_000.0,
        ] {
            let x = x_for_freq(plot, freq);
            push_rect(
                primitives,
                self.common.id,
                Rect::from_min_max(Point::new(x, plot.min.y), Point::new(x + 1.0, plot.max.y)),
                if freq == 1_000.0 {
                    theme.grid_strong
                } else {
                    theme.grid_soft
                },
            );
        }
        for gain in [-24.0, -12.0, 0.0, 12.0, 24.0] {
            let y = y_for_gain(plot, gain);
            push_rect(
                primitives,
                self.common.id,
                Rect::from_min_max(Point::new(plot.min.x, y), Point::new(plot.max.x, y + 1.0)),
                if gain == 0.0 {
                    theme.grid_strong
                } else {
                    theme.grid_soft
                },
            );
            push_text(
                primitives,
                self.common.id,
                format!("{gain:+.0}"),
                Rect::from_min_max(
                    Point::new(plot.min.x - 42.0, y - 9.0),
                    Point::new(plot.min.x - 6.0, y + 12.0),
                ),
                theme.text_muted,
                PaintTextAlign::Right,
            );
        }
        for (label, freq) in [
            ("20", 20.0),
            ("100", 100.0),
            ("1k", 1_000.0),
            ("10k", 10_000.0),
            ("20k", 20_000.0),
        ] {
            let x = x_for_freq(plot, freq);
            push_text(
                primitives,
                self.common.id,
                label,
                Rect::from_min_max(
                    Point::new(x - 22.0, plot.max.y + 8.0),
                    Point::new(x + 22.0, plot.max.y + 28.0),
                ),
                theme.text_muted,
                PaintTextAlign::Center,
            );
        }
    }

    fn append_analyzer(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        plot: Rect,
        theme: &ThemeTokens,
    ) {
        let floor = plot.max.y;
        let points = (0..96)
            .map(|index| {
                let ratio = index as f32 / 95.0;
                let x = plot.min.x + plot.width() * ratio;
                let wave = ((ratio * 5.8).sin() * 0.45 + (ratio * 18.0).sin() * 0.12).max(-0.9);
                let height = plot.height() * (0.18 + (1.0 - ratio).powf(0.7) * 0.32 + wave * 0.08);
                Point::new(x, floor - height)
            })
            .chain([Point::new(plot.max.x, floor), Point::new(plot.min.x, floor)])
            .collect::<Vec<_>>();
        primitives.push(PaintPrimitive::FillPolygon(PaintFillPolygon {
            widget_id: self.common.id,
            points: Arc::from(points),
            color: translucent(theme.highlight_blue, 46),
        }));
    }

    fn append_curve(&self, primitives: &mut Vec<PaintPrimitive>, plot: Rect, theme: &ThemeTokens) {
        let points = (0..160)
            .map(|index| {
                let ratio = index as f32 / 159.0;
                let freq = freq_for_ratio(ratio);
                Point::new(
                    plot.min.x + plot.width() * ratio,
                    y_for_gain(plot, response_gain_db(&self.bands, freq)),
                )
            })
            .collect::<Vec<_>>();
        primitives.push(PaintPrimitive::StrokePolyline(PaintStrokePolyline {
            widget_id: self.common.id,
            points: Arc::from(points),
            color: theme.accent_mint,
            width: 3.0,
        }));
    }

    fn append_band_handles(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        plot: Rect,
        theme: &ThemeTokens,
    ) {
        for band in &self.bands {
            let center = self.handle_center(plot, *band);
            let active = band.id == self.selected_band || Some(band.id) == self.hover_band;
            let fill = if !band.enabled {
                theme.text_muted
            } else if active {
                theme.accent_mint
            } else {
                theme.surface_raised
            };
            let rect = Rect::from_min_size(
                Point::new(center.x - HANDLE_SIZE * 0.5, center.y - HANDLE_SIZE * 0.5),
                Vector2::new(HANDLE_SIZE, HANDLE_SIZE),
            );
            push_rect(primitives, self.common.id, rect, fill);
            push_stroke(primitives, self.common.id, rect, theme.text_primary, 1.0);
            push_text(
                primitives,
                self.common.id,
                band.id.to_string(),
                Rect::from_min_max(
                    Point::new(center.x - 14.0, center.y - 25.0),
                    Point::new(center.x + 14.0, center.y - 7.0),
                ),
                theme.text_primary,
                PaintTextAlign::Center,
            );
        }
    }
}

fn response_gain_db(bands: &[EqBand], freq_hz: f32) -> f32 {
    bands
        .iter()
        .filter(|band| band.enabled)
        .map(|band| band_visual_gain(*band, freq_hz))
        .sum::<f32>()
        .clamp(MIN_GAIN_DB, MAX_GAIN_DB)
}

fn band_visual_gain(band: EqBand, freq_hz: f32) -> f32 {
    let octave_delta = (freq_hz / band.freq_hz).max(0.001).log2();
    match band.kind {
        EqBandKind::Bell => {
            let width = (1.1 / band.q.max(0.2)).max(0.14);
            band.gain_db * (-(octave_delta * octave_delta) / (2.0 * width * width)).exp()
        }
        EqBandKind::HighPass => -18.0 / (1.0 + ((freq_hz / band.freq_hz).max(0.001)).powf(5.0)),
        EqBandKind::HighShelf => {
            let blend = 1.0 / (1.0 + (-octave_delta * 3.0).exp());
            band.gain_db * blend
        }
    }
}

fn x_for_freq(plot: Rect, freq_hz: f32) -> f32 {
    plot.min.x + plot.width() * ratio_for_freq(freq_hz)
}

fn y_for_gain(plot: Rect, gain_db: f32) -> f32 {
    let ratio = ((gain_db.clamp(MIN_GAIN_DB, MAX_GAIN_DB) - MIN_GAIN_DB)
        / (MAX_GAIN_DB - MIN_GAIN_DB))
        .clamp(0.0, 1.0);
    plot.max.y - plot.height() * ratio
}

fn freq_for_x(plot: Rect, x: f32) -> f32 {
    freq_for_ratio(((x - plot.min.x) / plot.width().max(1.0)).clamp(0.0, 1.0))
}

fn gain_for_y(plot: Rect, y: f32) -> f32 {
    let ratio = ((plot.max.y - y) / plot.height().max(1.0)).clamp(0.0, 1.0);
    MIN_GAIN_DB + (MAX_GAIN_DB - MIN_GAIN_DB) * ratio
}

fn ratio_for_freq(freq_hz: f32) -> f32 {
    let min = MIN_FREQ_HZ.log10();
    let max = MAX_FREQ_HZ.log10();
    ((freq_hz.clamp(MIN_FREQ_HZ, MAX_FREQ_HZ).log10() - min) / (max - min)).clamp(0.0, 1.0)
}

fn freq_for_ratio(ratio: f32) -> f32 {
    let min = MIN_FREQ_HZ.log10();
    let max = MAX_FREQ_HZ.log10();
    10.0_f32.powf(min + (max - min) * ratio.clamp(0.0, 1.0))
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
    use radiant::widgets::PointerModifiers;

    #[test]
    fn eq_widget_paints_curve_analyzer_and_band_handles() {
        let state = EqEditorState::default();
        let widget = EqEditorWidget::new(state.bands, state.selected_band, true);
        let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(700.0, 300.0));
        let mut primitives = Vec::new();

        widget.append_paint(
            &mut primitives,
            bounds,
            &LayoutOutput::default(),
            &ThemeTokens::default(),
        );

        assert!(
            primitives.iter().any(|primitive| matches!(
                primitive,
                PaintPrimitive::StrokePolyline(PaintStrokePolyline { points, width, .. })
                    if points.len() == 160 && *width == 3.0
            )),
            "EQ response should paint as a sampled visual curve"
        );
        assert!(
            primitives
                .iter()
                .any(|primitive| matches!(primitive, PaintPrimitive::FillPolygon(_))),
            "analyzer overlay should be a normal GUI paint primitive"
        );
        assert!(
            primitives
                .iter()
                .filter(|primitive| matches!(primitive, PaintPrimitive::StrokeRect(_)))
                .count()
                >= 5,
            "plot and band handles should produce visible chrome"
        );
    }

    #[test]
    fn eq_widget_routes_select_and_drag_messages_without_dsp() {
        let state = EqEditorState::default();
        let mut widget = EqEditorWidget::new(state.bands.clone(), state.selected_band, true);
        let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(700.0, 300.0));
        let plot = widget.plot_rect(bounds);
        let band = state.bands[1];
        let center = widget.handle_center(plot, band);

        let select = widget
            .handle_input(
                bounds,
                WidgetInput::PointerPress {
                    position: center,
                    button: PointerButton::Primary,
                    modifiers: PointerModifiers::default(),
                },
            )
            .expect("pressing a band should emit selection");
        assert_eq!(
            select.typed_ref::<EqEditorMessage>(),
            Some(&EqEditorMessage::SelectBand(band.id))
        );

        let drag = widget
            .handle_input(
                bounds,
                WidgetInput::PointerMove {
                    position: Point::new(x_for_freq(plot, 1_000.0), y_for_gain(plot, 6.0)),
                },
            )
            .expect("dragging a band should emit a parameter-style GUI message");
        assert!(matches!(
            drag.typed_ref::<EqEditorMessage>(),
            Some(EqEditorMessage::MoveBand {
                id,
                freq_hz,
                gain_db,
            }) if *id == band.id && (*freq_hz - 1_000.0).abs() < 2.0 && (*gain_db - 6.0).abs() < 0.1
        ));
    }

    #[test]
    fn eq_update_applies_gui_parameter_messages() {
        let mut state = EqEditorState::default();

        update(
            &mut state,
            EqMessage::Editor(EqEditorMessage::MoveBand {
                id: 2,
                freq_hz: 1_200.0,
                gain_db: 8.0,
            }),
        );

        let band = selected_band(&state).expect("moved band should remain selected");
        assert_eq!(band.id, 2);
        assert_eq!(band.freq_hz, 1_200.0);
        assert_eq!(band.gain_db, 8.0);
        assert!(state.status.contains("moved"));
    }
}
