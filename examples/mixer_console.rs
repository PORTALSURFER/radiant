//! Eight-channel mixer console sandbox for DAW-style GUI interaction.

use radiant::prelude::*;
use radiant::{
    runtime::{PaintFillRect, PaintStrokeRect},
    widgets::PaintBounds,
};

const MIXER_WIDGET_ID: u64 = 90;
const STATUS_WIDGET_ID: u64 = 91;
const CHANNEL_COUNT: usize = 8;
const MIN_GAIN_DB: f32 = -60.0;
const MAX_GAIN_DB: f32 = 6.0;
const DATA_SOURCE_NOTE: &str = "without_dsp";

#[derive(Clone, Debug)]
struct MixerState {
    running: bool,
    frame: u64,
    selected_channel: usize,
    channels: [MixerChannel; CHANNEL_COUNT],
}

impl Default for MixerState {
    fn default() -> Self {
        let mut state = Self {
            running: true,
            frame: 0,
            selected_channel: 0,
            channels: std::array::from_fn(MixerChannel::new),
        };
        state.tick();
        state
    }
}

impl MixerState {
    fn tick(&mut self) {
        if !self.running {
            return;
        }
        self.frame = self.frame.saturating_add(1);
        for channel in &mut self.channels {
            channel.tick(self.frame);
        }
    }

    fn reset(&mut self) {
        self.frame = 0;
        self.running = true;
        self.channels = std::array::from_fn(MixerChannel::new);
        self.tick();
    }

    fn selected(&self) -> MixerChannel {
        self.channels[self.selected_channel]
    }

    fn status(&self) -> String {
        let selected = self.selected();
        let transport = if self.running { "running" } else { "paused" };
        format!(
            "{transport} | {} | fader {:+.1} dB | meter {:+.1} dB | synthetic GUI data",
            selected.label, selected.gain_db, selected.meter_db
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct MixerChannel {
    id: usize,
    label: &'static str,
    gain_db: f32,
    pan: f32,
    meter_db: f32,
    peak_db: f32,
    muted: bool,
    solo: bool,
    armed: bool,
}

impl MixerChannel {
    fn new(id: usize) -> Self {
        Self {
            id,
            label: CHANNEL_LABELS[id],
            gain_db: DEFAULT_GAINS[id],
            pan: DEFAULT_PANS[id],
            meter_db: MIN_GAIN_DB,
            peak_db: MIN_GAIN_DB,
            muted: false,
            solo: false,
            armed: id == 0,
        }
    }

    fn tick(&mut self, frame: u64) {
        let level = synthetic_level(frame, self.id, self.gain_db, self.muted);
        let target_db = level_to_db(level);
        self.meter_db = self.meter_db * 0.72 + target_db * 0.28;
        self.peak_db = if target_db > self.peak_db {
            target_db
        } else {
            (self.peak_db - 0.42).max(self.meter_db)
        };
    }

    fn set_gain_from_ratio(&mut self, ratio: f32) {
        self.gain_db = gain_for_ratio(ratio);
        if ratio <= 0.001 {
            self.meter_db = MIN_GAIN_DB;
            self.peak_db = MIN_GAIN_DB;
        }
    }

    fn gain_ratio(&self) -> f32 {
        ratio_for_gain(self.gain_db)
    }

    fn is_visually_dimmed_by_solo(&self, solo_active: bool) -> bool {
        solo_active && !self.solo
    }
}

const CHANNEL_LABELS: [&str; CHANNEL_COUNT] =
    ["Kick", "Snare", "Hat", "Bass", "Keys", "Pad", "Lead", "Vox"];
const DEFAULT_GAINS: [f32; CHANNEL_COUNT] = [-5.0, -7.5, -12.0, -8.0, -9.0, -14.0, -10.0, -6.5];
const DEFAULT_PANS: [f32; CHANNEL_COUNT] = [0.0, 0.0, -0.34, -0.08, 0.22, 0.38, -0.18, 0.06];

#[derive(Clone, Copy, Debug, PartialEq)]
enum MixerMessage {
    Frame,
    ToggleRun,
    Reset,
    Panel(MixerPanelMessage),
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum MixerPanelMessage {
    Select(usize),
    SetGain { channel: usize, ratio: f32 },
    ToggleMute(usize),
    ToggleSolo(usize),
    ToggleArm(usize),
}

fn main() -> radiant::Result {
    radiant::app(MixerState::default())
        .title("Radiant Mixer Console")
        .size(1040, 620)
        .min_size(820, 500)
        .view(project_surface)
        .animation(|state| state.running)
        .on_frame(|| MixerMessage::Frame)
        .update(update)
        .run()
}

fn project_surface(state: &mut MixerState) -> View<MixerMessage> {
    let selected = state.selected();
    column([
        row([
            text("8-Channel Mixer").height(30.0).fill_width(),
            button(if state.running { "Pause" } else { "Run" })
                .primary()
                .message(MixerMessage::ToggleRun)
                .size(88.0, 30.0),
            button("Reset")
                .subtle()
                .message(MixerMessage::Reset)
                .size(82.0, 30.0),
        ])
        .fill_width()
        .spacing(10.0),
        custom_widget_mapped(
            MixerPanelWidget::new(state.channels, state.selected_channel, state.frame),
            MixerMessage::Panel,
        )
        .id(MIXER_WIDGET_ID)
        .height(380.0)
        .fill_width(),
        row([
            channel_summary_tile(selected),
            stat_tile("Source", DATA_SOURCE_NOTE),
            stat_tile("Peak", format!("{:+.1} dB", selected.peak_db)),
            stat_tile("Pan", format!("{:+.0}%", selected.pan * 100.0)),
            text(state.status())
                .id(STATUS_WIDGET_ID)
                .height(68.0)
                .fill_width(),
        ])
        .fill_width()
        .spacing(10.0),
    ])
    .style(WidgetStyle::default())
    .padding(16.0)
    .spacing(12.0)
    .fill()
}

fn channel_summary_tile(channel: MixerChannel) -> View<MixerMessage> {
    stat_tile(
        format!("Selected {}", channel.label),
        format!("{:+.1} dB fader", channel.gain_db),
    )
}

fn stat_tile(label: impl Into<String>, value: impl Into<String>) -> View<MixerMessage> {
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
    .height(68.0)
    .fill_width()
}

fn update(state: &mut MixerState, message: MixerMessage) {
    match message {
        MixerMessage::Frame => state.tick(),
        MixerMessage::ToggleRun => {
            state.running = !state.running;
        }
        MixerMessage::Reset => state.reset(),
        MixerMessage::Panel(message) => update_panel(state, message),
    }
}

fn update_panel(state: &mut MixerState, message: MixerPanelMessage) {
    match message {
        MixerPanelMessage::Select(channel) => {
            state.selected_channel = channel.min(CHANNEL_COUNT - 1);
        }
        MixerPanelMessage::SetGain { channel, ratio } => {
            if let Some(channel_state) = state.channels.get_mut(channel) {
                channel_state.set_gain_from_ratio(ratio);
                state.selected_channel = channel;
            }
        }
        MixerPanelMessage::ToggleMute(channel) => {
            if let Some(channel_state) = state.channels.get_mut(channel) {
                channel_state.muted = !channel_state.muted;
                state.selected_channel = channel;
            }
        }
        MixerPanelMessage::ToggleSolo(channel) => {
            if let Some(channel_state) = state.channels.get_mut(channel) {
                channel_state.solo = !channel_state.solo;
                state.selected_channel = channel;
            }
        }
        MixerPanelMessage::ToggleArm(channel) => {
            if let Some(channel_state) = state.channels.get_mut(channel) {
                channel_state.armed = !channel_state.armed;
                state.selected_channel = channel;
            }
        }
    }
}

#[derive(Clone, Debug)]
struct MixerPanelWidget {
    common: WidgetCommon,
    channels: [MixerChannel; CHANNEL_COUNT],
    selected_channel: usize,
    frame: u64,
    hover_channel: Option<usize>,
    hover_position: Option<Point>,
    drag_channel: Option<usize>,
}

impl MixerPanelWidget {
    fn new(channels: [MixerChannel; CHANNEL_COUNT], selected_channel: usize, frame: u64) -> Self {
        let mut common = WidgetCommon::new(
            0,
            WidgetSizing::new(Vector2::new(760.0, 340.0), Vector2::new(1000.0, 380.0)),
        );
        common.focus = FocusBehavior::Pointer;
        common.paint.bounds = PaintBounds::ClipToRect;
        common.paint.paints_focus = false;
        common.paint.paints_state_layers = false;
        Self {
            common,
            channels,
            selected_channel,
            frame,
            hover_channel: None,
            hover_position: None,
            drag_channel: None,
        }
    }

    fn console_rect(&self, bounds: Rect) -> Rect {
        Rect::from_min_max(
            Point::new(bounds.min.x + 12.0, bounds.min.y + 12.0),
            Point::new(bounds.max.x - 12.0, bounds.max.y - 12.0),
        )
    }

    fn strip_rect(&self, bounds: Rect, channel: usize) -> Rect {
        let console = self.console_rect(bounds);
        let gap = 8.0;
        let strip_width =
            (console.width() - gap * (CHANNEL_COUNT - 1) as f32) / CHANNEL_COUNT as f32;
        let x = console.min.x + channel as f32 * (strip_width + gap);
        Rect::from_min_size(
            Point::new(x, console.min.y),
            Vector2::new(strip_width.max(1.0), console.height()),
        )
    }

    fn meter_rect(&self, strip: Rect) -> Rect {
        Rect::from_min_max(
            Point::new(strip.min.x + strip.width() * 0.20, strip.min.y + 48.0),
            Point::new(strip.min.x + strip.width() * 0.46, strip.max.y - 112.0),
        )
    }

    fn fader_rect(&self, strip: Rect) -> Rect {
        Rect::from_min_max(
            Point::new(strip.min.x + strip.width() * 0.58, strip.min.y + 56.0),
            Point::new(strip.min.x + strip.width() * 0.84, strip.max.y - 116.0),
        )
    }

    fn button_rect(&self, strip: Rect, index: usize) -> Rect {
        let width = (strip.width() - 22.0) / 3.0;
        let x = strip.min.x + 8.0 + index as f32 * (width + 3.0);
        Rect::from_min_size(
            Point::new(x, strip.max.y - 82.0),
            Vector2::new(width.max(18.0), 24.0),
        )
    }

    fn channel_at(&self, bounds: Rect, position: Point) -> Option<usize> {
        (0..CHANNEL_COUNT).find(|channel| self.strip_rect(bounds, *channel).contains(position))
    }

    fn fader_ratio_at(&self, strip: Rect, position: Point) -> f32 {
        let fader = self.fader_rect(strip);
        ((fader.max.y - position.y) / fader.height().max(1.0)).clamp(0.0, 1.0)
    }
}

impl Widget for MixerPanelWidget {
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
                self.hover_position = bounds.contains(position).then_some(position);
                self.hover_channel = self.channel_at(bounds, position);
                if let Some(channel) = self.drag_channel {
                    let strip = self.strip_rect(bounds, channel);
                    return Some(WidgetOutput::custom(MixerPanelMessage::SetGain {
                        channel,
                        ratio: self.fader_ratio_at(strip, position),
                    }));
                }
                None
            }
            WidgetInput::PointerPress {
                position,
                button: PointerButton::Primary,
                ..
            } if bounds.contains(position) => {
                let channel = self.channel_at(bounds, position)?;
                let strip = self.strip_rect(bounds, channel);
                self.selected_channel = channel;
                self.hover_channel = Some(channel);
                if self.fader_rect(strip).contains(position) {
                    self.drag_channel = Some(channel);
                    return Some(WidgetOutput::custom(MixerPanelMessage::SetGain {
                        channel,
                        ratio: self.fader_ratio_at(strip, position),
                    }));
                }
                Some(WidgetOutput::custom(button_or_select_message(
                    self, strip, channel, position,
                )))
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
                let drag = self.drag_channel.take();
                self.hover_channel = self.channel_at(bounds, position);
                drag.map(|channel| {
                    let strip = self.strip_rect(bounds, channel);
                    WidgetOutput::custom(MixerPanelMessage::SetGain {
                        channel,
                        ratio: self.fader_ratio_at(strip, position),
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
        self.drag_channel.is_none()
    }

    fn synchronize_from_previous(&mut self, previous: &dyn Widget) {
        if let Some(previous) = previous.as_any().downcast_ref::<Self>() {
            self.common.state = previous.common.state;
            self.hover_channel = previous.hover_channel;
            self.hover_position = previous.hover_position;
            self.drag_channel = previous.drag_channel;
        }
    }

    fn append_paint(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        bounds: Rect,
        _layout: &LayoutOutput,
        theme: &ThemeTokens,
    ) {
        push_rect(primitives, self.common.id, bounds, theme.bg_secondary);
        for channel in 0..CHANNEL_COUNT {
            self.append_strip(primitives, bounds, channel, theme);
        }
        push_text(
            primitives,
            self.common.id,
            format!("frame {}", self.frame),
            Rect::from_min_max(
                Point::new(bounds.max.x - 120.0, bounds.min.y + 12.0),
                Point::new(bounds.max.x - 20.0, bounds.min.y + 32.0),
            ),
            theme.text_muted,
            PaintTextAlign::Right,
        );
    }

    fn append_runtime_overlay_paint(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        bounds: Rect,
        _layout: &LayoutOutput,
        theme: &ThemeTokens,
    ) {
        let Some(channel) = self.hover_channel else {
            return;
        };
        let strip = self.strip_rect(bounds, channel);
        push_stroke(
            primitives,
            self.common.id,
            strip,
            translucent(theme.highlight_cyan, 170),
            2.0,
        );
    }
}

impl MixerPanelWidget {
    fn append_strip(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        bounds: Rect,
        channel_index: usize,
        theme: &ThemeTokens,
    ) {
        let channel = self.channels[channel_index];
        let strip = self.strip_rect(bounds, channel_index);
        let selected = channel_index == self.selected_channel;
        let solo_active = self.channels.iter().any(|channel| channel.solo);
        let solo_dimmed = channel.is_visually_dimmed_by_solo(solo_active);
        let fill = if selected {
            blend_color(theme.surface_raised, theme.highlight_blue, 0.20)
        } else if solo_dimmed {
            blend_color(theme.surface_base, theme.bg_primary, 0.42)
        } else {
            theme.surface_base
        };
        push_rect(primitives, self.common.id, strip, fill);
        push_stroke(primitives, self.common.id, strip, theme.border, 1.0);
        push_text(
            primitives,
            self.common.id,
            channel.label,
            Rect::from_min_size(
                Point::new(strip.min.x + 8.0, strip.min.y + 10.0),
                Vector2::new(strip.width() - 16.0, 22.0),
            ),
            if solo_dimmed {
                theme.text_muted
            } else {
                theme.text_primary
            },
            PaintTextAlign::Center,
        );
        self.append_meter(primitives, channel, strip, solo_dimmed, theme);
        self.append_fader(primitives, channel, strip, solo_dimmed, theme);
        self.append_channel_buttons(primitives, channel, strip, theme);
        push_text(
            primitives,
            self.common.id,
            format!("{:+.1} dB", channel.gain_db),
            Rect::from_min_size(
                Point::new(strip.min.x + 6.0, strip.max.y - 48.0),
                Vector2::new(strip.width() - 12.0, 18.0),
            ),
            if solo_dimmed {
                translucent(theme.text_muted, 150)
            } else {
                theme.text_muted
            },
            PaintTextAlign::Center,
        );
        push_text(
            primitives,
            self.common.id,
            format!("{:+.0}", channel.pan * 100.0),
            Rect::from_min_size(
                Point::new(strip.min.x + 6.0, strip.max.y - 28.0),
                Vector2::new(strip.width() - 12.0, 18.0),
            ),
            theme.text_muted,
            PaintTextAlign::Center,
        );
    }

    fn append_meter(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        channel: MixerChannel,
        strip: Rect,
        solo_dimmed: bool,
        theme: &ThemeTokens,
    ) {
        let meter = self.meter_rect(strip);
        push_rect(
            primitives,
            self.common.id,
            meter,
            if solo_dimmed {
                rgba(14, 15, 17, 255)
            } else {
                rgba(8, 13, 18, 255)
            },
        );
        for fraction in [0.25, 0.5, 0.75] {
            let y = meter.max.y - meter.height() * fraction;
            push_rect(
                primitives,
                self.common.id,
                Rect::from_min_max(Point::new(meter.min.x, y), Point::new(meter.max.x, y + 1.0)),
                translucent(theme.grid_soft, 120),
            );
        }
        let meter_ratio = ratio_for_meter_db(channel.meter_db);
        let meter_fill = Rect::from_min_max(
            Point::new(
                meter.min.x + 3.0,
                meter.max.y - (meter.height() - 6.0) * meter_ratio,
            ),
            Point::new(meter.max.x - 3.0, meter.max.y - 3.0),
        );
        push_rect(
            primitives,
            self.common.id,
            meter_fill,
            if solo_dimmed {
                rgba(75, 80, 86, 180)
            } else {
                meter_color(channel.meter_db)
            },
        );
        let peak_y = meter.max.y - meter.height() * ratio_for_meter_db(channel.peak_db);
        push_rect(
            primitives,
            self.common.id,
            Rect::from_min_max(
                Point::new(meter.min.x + 2.0, peak_y),
                Point::new(meter.max.x - 2.0, peak_y + 2.0),
            ),
            if solo_dimmed {
                rgba(90, 94, 98, 160)
            } else {
                theme.highlight_orange
            },
        );
        push_text(
            primitives,
            self.common.id,
            format!("{:+.0}", channel.meter_db),
            Rect::from_min_size(
                Point::new(meter.min.x - 16.0, meter.max.y + 8.0),
                Vector2::new(meter.width() + 32.0, 18.0),
            ),
            if solo_dimmed {
                rgba(118, 123, 128, 180)
            } else {
                theme.text_muted
            },
            PaintTextAlign::Center,
        );
    }

    fn append_fader(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        channel: MixerChannel,
        strip: Rect,
        solo_dimmed: bool,
        theme: &ThemeTokens,
    ) {
        let fader = self.fader_rect(strip);
        let center_x = fader.center().x;
        push_rect(
            primitives,
            self.common.id,
            Rect::from_min_max(
                Point::new(center_x - 2.0, fader.min.y),
                Point::new(center_x + 2.0, fader.max.y),
            ),
            if solo_dimmed {
                translucent(theme.grid_soft, 130)
            } else {
                theme.grid_strong
            },
        );
        for db in [-48.0, -24.0, -12.0, 0.0, 6.0] {
            let y = fader.max.y - fader.height() * ratio_for_gain(db);
            push_rect(
                primitives,
                self.common.id,
                Rect::from_min_max(
                    Point::new(center_x - 10.0, y),
                    Point::new(center_x + 10.0, y + 1.0),
                ),
                theme.grid_soft,
            );
        }
        let knob_y = fader.max.y - fader.height() * channel.gain_ratio();
        let knob = Rect::from_min_size(
            Point::new(fader.min.x, knob_y - 8.0),
            Vector2::new(fader.width(), 16.0),
        );
        push_rect(
            primitives,
            self.common.id,
            knob,
            if solo_dimmed {
                rgba(86, 92, 100, 220)
            } else {
                theme.highlight_blue
            },
        );
        push_stroke(primitives, self.common.id, knob, theme.border_emphasis, 1.0);
    }

    fn append_channel_buttons(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        channel: MixerChannel,
        strip: Rect,
        theme: &ThemeTokens,
    ) {
        for (index, label, active, active_color) in [
            (0, "M", channel.muted, theme.accent_danger),
            (1, "S", channel.solo, theme.accent_warning),
            (2, "R", channel.armed, theme.highlight_cyan),
        ] {
            let rect = self.button_rect(strip, index);
            push_rect(
                primitives,
                self.common.id,
                rect,
                if active {
                    active_color
                } else {
                    theme.bg_tertiary
                },
            );
            push_stroke(primitives, self.common.id, rect, theme.border, 1.0);
            push_text(
                primitives,
                self.common.id,
                label,
                rect,
                theme.text_primary,
                PaintTextAlign::Center,
            );
        }
    }
}

fn button_or_select_message(
    widget: &MixerPanelWidget,
    strip: Rect,
    channel: usize,
    position: Point,
) -> MixerPanelMessage {
    if widget.button_rect(strip, 0).contains(position) {
        MixerPanelMessage::ToggleMute(channel)
    } else if widget.button_rect(strip, 1).contains(position) {
        MixerPanelMessage::ToggleSolo(channel)
    } else if widget.button_rect(strip, 2).contains(position) {
        MixerPanelMessage::ToggleArm(channel)
    } else {
        MixerPanelMessage::Select(channel)
    }
}

fn synthetic_level(frame: u64, channel: usize, gain_db: f32, muted: bool) -> f32 {
    if muted {
        return 0.0;
    }
    let phase = frame as f32 * (0.034 + channel as f32 * 0.004);
    let pulse = (phase.sin() * 0.5 + 0.5).powf(1.7);
    let wobble = ((phase * 0.37 + channel as f32).cos() * 0.5 + 0.5) * 0.32;
    let transient = if (frame + channel as u64 * 11) % (34 + channel as u64 * 3) < 4 {
        0.38
    } else {
        0.0
    };
    let fader_gain = db_to_linear(gain_db);
    ((0.08 + pulse * 0.50 + wobble + transient).min(1.0) * fader_gain).min(1.0)
}

fn level_to_db(level: f32) -> f32 {
    if level <= 0.001 {
        MIN_GAIN_DB
    } else {
        (20.0 * level.clamp(0.001, 1.0).log10()).clamp(MIN_GAIN_DB, MAX_GAIN_DB)
    }
}

fn ratio_for_meter_db(db: f32) -> f32 {
    ((db.clamp(MIN_GAIN_DB, 0.0) - MIN_GAIN_DB) / (0.0 - MIN_GAIN_DB)).clamp(0.0, 1.0)
}

fn ratio_for_gain(db: f32) -> f32 {
    ((db.clamp(MIN_GAIN_DB, MAX_GAIN_DB) - MIN_GAIN_DB) / (MAX_GAIN_DB - MIN_GAIN_DB))
        .clamp(0.0, 1.0)
}

fn gain_for_ratio(ratio: f32) -> f32 {
    MIN_GAIN_DB + (MAX_GAIN_DB - MIN_GAIN_DB) * ratio.clamp(0.0, 1.0)
}

fn db_to_linear(db: f32) -> f32 {
    10.0_f32.powf(db.clamp(MIN_GAIN_DB, MAX_GAIN_DB) / 20.0)
}

fn meter_color(db: f32) -> Rgba8 {
    if db > -3.0 {
        rgba(255, 82, 52, 255)
    } else if db > -10.0 {
        rgba(255, 190, 72, 255)
    } else {
        rgba(60, 214, 154, 255)
    }
}

fn blend_color(a: Rgba8, b: Rgba8, t: f32) -> Rgba8 {
    let t = t.clamp(0.0, 1.0);
    rgba(
        (a.r as f32 + (b.r as f32 - a.r as f32) * t).round() as u8,
        (a.g as f32 + (b.g as f32 - a.g as f32) * t).round() as u8,
        (a.b as f32 + (b.b as f32 - a.b as f32) * t).round() as u8,
        255,
    )
}

fn rgba(r: u8, g: u8, b: u8, a: u8) -> Rgba8 {
    Rgba8 { r, g, b, a }
}

fn translucent(mut color: Rgba8, alpha: u8) -> Rgba8 {
    color.a = alpha;
    color
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

#[cfg(test)]
mod tests {
    use super::*;
    use radiant::runtime::{RuntimeBridge, SurfaceRuntime};

    #[test]
    fn mixer_tick_animates_synthetic_decibel_meters_without_dsp() {
        let mut state = MixerState::default();
        let initial = state.channels.map(|channel| channel.meter_db);

        for _ in 0..8 {
            state.tick();
        }

        assert_eq!(state.channels.len(), CHANNEL_COUNT);
        assert_ne!(state.channels.map(|channel| channel.meter_db), initial);
        assert!(state.channels.iter().all(|channel| channel.meter_db <= 0.0));
    }

    #[test]
    fn mixer_fader_down_drives_meter_to_silence() {
        let mut state = MixerState::default();
        update_panel(
            &mut state,
            MixerPanelMessage::SetGain {
                channel: 3,
                ratio: 0.0,
            },
        );

        assert_eq!(state.channels[3].gain_db, MIN_GAIN_DB);
        assert_eq!(state.channels[3].meter_db, MIN_GAIN_DB);
        assert_eq!(state.channels[3].peak_db, MIN_GAIN_DB);

        state.tick();

        assert_eq!(state.channels[3].meter_db, MIN_GAIN_DB);
    }

    #[test]
    fn mixer_solo_keeps_non_solo_meters_active_for_visual_information() {
        let mut state = MixerState::default();
        update_panel(&mut state, MixerPanelMessage::ToggleSolo(1));

        for _ in 0..24 {
            state.tick();
        }

        assert!(state.channels[1].solo);
        assert!(
            state.channels[0].meter_db > MIN_GAIN_DB,
            "non-solo channels should keep showing pre-solo visual meter information"
        );
        assert!(
            state.channels[1].meter_db > MIN_GAIN_DB,
            "soloed channel should remain visually active"
        );
    }

    #[test]
    fn mixer_solo_grays_non_solo_meter_paint() {
        let mut state = MixerState::default();
        update_panel(&mut state, MixerPanelMessage::ToggleSolo(1));
        let widget = MixerPanelWidget::new(state.channels, state.selected_channel, state.frame);
        let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(960.0, 380.0));
        let mut primitives = Vec::new();

        widget.append_paint(
            &mut primitives,
            bounds,
            &LayoutOutput::default(),
            &ThemeTokens::default(),
        );

        assert!(
            primitives.iter().any(|primitive| {
                matches!(primitive, PaintPrimitive::FillRect(fill) if fill.color == rgba(75, 80, 86, 180))
            }),
            "solo mode should gray out non-solo meter fills"
        );
    }

    #[test]
    fn mixer_panel_paints_eight_channel_strips_and_db_labels() {
        let state = MixerState::default();
        let widget = MixerPanelWidget::new(state.channels, state.selected_channel, state.frame);
        let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(960.0, 380.0));
        let mut primitives = Vec::new();

        widget.append_paint(
            &mut primitives,
            bounds,
            &LayoutOutput::default(),
            &ThemeTokens::default(),
        );

        let label_count = primitives
            .iter()
            .filter(|primitive| {
                matches!(
                    primitive,
                    PaintPrimitive::Text(text) if CHANNEL_LABELS.contains(&text.text.as_str())
                )
            })
            .count();
        assert_eq!(label_count, CHANNEL_COUNT);
        assert!(
            primitives
                .iter()
                .any(|primitive| matches!(primitive, PaintPrimitive::Text(text) if text.text.as_str().contains("dB"))),
            "mixer should paint decibel readouts"
        );
    }

    #[test]
    fn mixer_panel_hover_uses_paint_only_runtime_overlay() {
        let state = MixerState::default();
        let mut widget = MixerPanelWidget::new(state.channels, state.selected_channel, state.frame);
        let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(960.0, 380.0));
        let strip = widget.strip_rect(bounds, 2);

        let output = widget.handle_input(
            bounds,
            WidgetInput::PointerMove {
                position: strip.center(),
            },
        );

        assert!(output.is_none());
        assert_eq!(widget.hover_channel, Some(2));
        assert!(widget.prefers_pointer_move_paint_only());
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
                .any(|primitive| matches!(primitive, PaintPrimitive::StrokeRect(_))),
            "hover strip should paint as a lightweight runtime overlay"
        );
    }

    #[test]
    fn mixer_panel_fader_drag_routes_gain_change() {
        let state = MixerState::default();
        let mut widget = MixerPanelWidget::new(state.channels, state.selected_channel, state.frame);
        let bounds = Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(960.0, 380.0));
        let strip = widget.strip_rect(bounds, 4);
        let fader = widget.fader_rect(strip);

        let output = widget.handle_input(
            bounds,
            WidgetInput::PointerPress {
                position: Point::new(fader.center().x, fader.min.y),
                button: PointerButton::Primary,
                modifiers: Default::default(),
            },
        );

        assert_eq!(
            output.and_then(|output| output.typed_ref::<MixerPanelMessage>().copied()),
            Some(MixerPanelMessage::SetGain {
                channel: 4,
                ratio: 1.0
            })
        );
        assert!(!widget.prefers_pointer_move_paint_only());
    }

    #[test]
    fn mixer_runtime_hover_does_not_refresh_surface() {
        let bridge = mixer_test_bridge(MixerState::default());
        let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(1040.0, 620.0));
        let bounds = runtime.layout().rects[&MIXER_WIDGET_ID];
        let first = runtime
            .dispatch_pointer_move_with_outcome(Point::new(bounds.min.x + 80.0, bounds.center().y));
        let second = runtime.dispatch_pointer_move_with_outcome(Point::new(
            bounds.min.x + 180.0,
            bounds.center().y,
        ));

        assert!(first.needs_scene_rebuild());
        assert!(second.paint_only_requested);
        assert!(
            !second.needs_scene_rebuild(),
            "stable mixer hover should avoid reprojection and full scene rebuilds"
        );
    }

    #[test]
    fn mixer_runtime_frame_messages_advance_status() {
        let bridge = mixer_test_bridge(MixerState::default());
        let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(1040.0, 620.0));
        let initial_status = status_text(&runtime);

        assert!(runtime.bridge_mut().needs_animation());
        assert!(runtime.bridge_mut().queue_animation_frame());
        let outcome = runtime.drain_runtime_messages();

        assert_eq!(outcome.messages_dispatched, 1);
        assert_ne!(status_text(&runtime), initial_status);
    }

    fn mixer_test_bridge(state: MixerState) -> impl RuntimeBridge<MixerMessage> {
        radiant::app(state)
            .view(project_surface)
            .animation(|state| state.running)
            .on_frame(|| MixerMessage::Frame)
            .update(update)
            .into_bridge()
    }

    fn status_text<Bridge>(runtime: &SurfaceRuntime<Bridge, MixerMessage>) -> String
    where
        Bridge: RuntimeBridge<MixerMessage>,
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
