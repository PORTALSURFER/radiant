//! Dense mixer console sandbox for DAW-style GUI interaction.

use radiant::prelude::*;
use radiant::{
    runtime::{PaintFillRect, PaintStrokeRect},
    widgets::{PaintBounds, PointerModifiers},
};

const MIXER_WIDGET_ID: u64 = 90;
const STATUS_WIDGET_ID: u64 = 91;
const CHANNEL_COUNT: usize = 32;
const SEND_COUNT: usize = 3;
const GROUP_COUNT: usize = 4;
const MIN_GAIN_DB: f32 = -60.0;
const MAX_GAIN_DB: f32 = 6.0;
const DATA_SOURCE_NOTE: &str = "without_dsp";

#[derive(Clone, Debug)]
struct MixerState {
    running: bool,
    frame: u64,
    selected_channel: usize,
    selection: ListSelectionController,
    channels: [MixerChannel; CHANNEL_COUNT],
}

impl Default for MixerState {
    fn default() -> Self {
        let mut selection = ListSelectionController::new();
        selection.select(0, CHANNEL_COUNT, ListSelectionModifiers::new());
        let mut state = Self {
            running: true,
            frame: 0,
            selected_channel: 0,
            selection,
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
        self.selected_channel = 0;
        self.selection.clear();
        self.selection.select(
            self.selected_channel,
            CHANNEL_COUNT,
            ListSelectionModifiers::new(),
        );
        self.channels = std::array::from_fn(MixerChannel::new);
        self.tick();
    }

    fn selected(&self) -> MixerChannel {
        self.channels[self.selected_channel]
    }

    fn status(&self) -> String {
        let selected = self.selected();
        let transport = if self.running { "running" } else { "paused" };
        let selected_count = self.selection.selected_indices().len().max(1);
        format!(
            "{transport} | {selected_count} selected | {} | group {} | fader {:+.1} dB | send A {:.0}% | meter {:+.1} dB | synthetic GUI data",
            selected.label,
            selected.group() + 1,
            selected.gain_db,
            selected.sends[0] * 100.0,
            selected.meter_db
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
    sends: [f32; SEND_COUNT],
    muted: bool,
    solo: bool,
    armed: bool,
}

impl MixerChannel {
    fn new(id: usize) -> Self {
        Self {
            id,
            label: CHANNEL_LABELS[id],
            gain_db: default_gain(id),
            pan: default_pan(id),
            meter_db: MIN_GAIN_DB,
            peak_db: MIN_GAIN_DB,
            sends: default_sends(id),
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
        self.set_gain_from_db(gain_for_ratio(ratio));
        if ratio <= 0.001 {
            self.meter_db = MIN_GAIN_DB;
            self.peak_db = MIN_GAIN_DB;
        }
    }

    fn set_gain_from_db(&mut self, db: f32) {
        self.gain_db = db.clamp(MIN_GAIN_DB, MAX_GAIN_DB);
        if self.gain_db <= MIN_GAIN_DB + 0.001 {
            self.meter_db = MIN_GAIN_DB;
            self.peak_db = MIN_GAIN_DB;
        }
    }

    fn group(&self) -> usize {
        self.id / (CHANNEL_COUNT / GROUP_COUNT)
    }

    fn is_visually_dimmed_by_solo(&self, solo_active: bool) -> bool {
        solo_active && !self.solo
    }
}

const CHANNEL_LABELS: [&str; CHANNEL_COUNT] = [
    "Kik", "Snr", "Hat", "Tom", "Rid", "Clp", "Shk", "Per", "Bass", "Sub", "Gtr1", "Gtr2", "Keys",
    "Pno", "Org", "Pad", "Ld1", "Ld2", "Plk", "Arp", "Vox1", "Vox2", "Bgv1", "Bgv2", "FX1", "FX2",
    "Amb", "Loop", "BusA", "BusB", "Print", "Ref",
];

fn default_gain(channel: usize) -> f32 {
    -5.0 - (channel % 8) as f32 * 1.4 - (channel / 8) as f32 * 0.8
}

fn default_pan(channel: usize) -> f32 {
    const PANS: [f32; 8] = [-0.55, -0.28, -0.08, 0.0, 0.10, 0.24, 0.42, 0.58];
    PANS[channel % PANS.len()]
}

fn default_sends(channel: usize) -> [f32; SEND_COUNT] {
    [
        0.14 + (channel % 5) as f32 * 0.035,
        0.08 + (channel % 7) as f32 * 0.025,
        0.05 + (channel % 4) as f32 * 0.045,
    ]
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum MixerMessage {
    Frame,
    ToggleRun,
    Reset,
    Panel(MixerPanelMessage),
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum MixerPanelMessage {
    Select {
        channel: usize,
        modifiers: ListSelectionModifiers,
    },
    SetGain {
        channel: usize,
        ratio: f32,
        selection: Option<ListSelectionModifiers>,
    },
    SetSend {
        channel: usize,
        send: usize,
        ratio: f32,
    },
    Reorder {
        from: usize,
        insert: usize,
    },
    ToggleMute(usize),
    ToggleSolo(usize),
    ToggleArm(usize),
}

fn main() -> radiant::Result {
    radiant::app(MixerState::default())
        .title("Radiant Mixer Console")
        .size(1440, 760)
        .min_size(1180, 620)
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
            text("32-Channel Mixer").height(30.0).fill_width(),
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
            MixerPanelWidget::new(
                state.channels,
                state.selection.clone(),
                state.selected_channel,
                state.frame,
            ),
            MixerMessage::Panel,
        )
        .id(MIXER_WIDGET_ID)
        .height(500.0)
        .fill_width(),
        row([
            channel_summary_tile(selected),
            stat_tile("Source", DATA_SOURCE_NOTE),
            stat_tile("Peak", format!("{:+.1} dB", selected.peak_db)),
            stat_tile("Send A", format!("{:.0}%", selected.sends[0] * 100.0)),
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
        MixerPanelMessage::Select { channel, modifiers } => {
            let channel = channel.min(CHANNEL_COUNT - 1);
            state.selected_channel = channel;
            state.selection.select(channel, CHANNEL_COUNT, modifiers);
        }
        MixerPanelMessage::SetGain {
            channel,
            ratio,
            selection,
        } => {
            let channel = channel.min(CHANNEL_COUNT - 1);
            if let Some(modifiers) = selection {
                state.selection.select(channel, CHANNEL_COUNT, modifiers);
            }
            if state.selection.is_selected(channel) && state.selection.selected_indices().len() > 1
            {
                let target_gain = gain_for_ratio(ratio);
                let source_gain = state.channels[channel].gain_db;
                let delta = target_gain - source_gain;
                let selected = state.selection.selected_indices().to_vec();
                for selected_channel in selected {
                    if let Some(channel_state) = state.channels.get_mut(selected_channel) {
                        channel_state.set_gain_from_db(channel_state.gain_db + delta);
                    }
                }
            } else if let Some(channel_state) = state.channels.get_mut(channel) {
                channel_state.set_gain_from_ratio(ratio);
                state.selected_channel = channel;
            }
            state.selected_channel = channel;
        }
        MixerPanelMessage::SetSend {
            channel,
            send,
            ratio,
        } => {
            if let Some(channel_state) = state.channels.get_mut(channel)
                && let Some(send_state) = channel_state.sends.get_mut(send)
            {
                *send_state = ratio.clamp(0.0, 1.0);
                state.selected_channel = channel;
            }
        }
        MixerPanelMessage::Reorder { from, insert } => {
            reorder_channels(state, from, insert);
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

fn reorder_channels(state: &mut MixerState, from: usize, insert: usize) {
    let from = from.min(CHANNEL_COUNT - 1);
    let insert = insert.min(CHANNEL_COUNT);
    if is_reorder_noop(from, insert) {
        return;
    }

    let selected_ids: Vec<_> = state
        .selection
        .selected_indices()
        .iter()
        .filter_map(|channel| state.channels.get(*channel))
        .map(|channel| channel.id)
        .collect();
    let focused_id = state.channels[state.selected_channel.min(CHANNEL_COUNT - 1)].id;
    let mut channels = state.channels.to_vec();
    let moved = channels.remove(from);
    let adjusted_insert = if insert > from { insert - 1 } else { insert };
    channels.insert(adjusted_insert, moved);
    state.channels = channels
        .try_into()
        .expect("mixer reorder should preserve channel count");

    state.selection.clear();
    let mut restored_any = false;
    for (index, channel) in state.channels.iter().enumerate() {
        if selected_ids.contains(&channel.id) {
            let modifiers = if restored_any {
                ListSelectionModifiers::toggle()
            } else {
                ListSelectionModifiers::new()
            };
            state.selection.select(index, CHANNEL_COUNT, modifiers);
            restored_any = true;
        }
    }
    state.selected_channel = state
        .channels
        .iter()
        .position(|channel| channel.id == focused_id)
        .unwrap_or(adjusted_insert);
    if !restored_any {
        state.selection.select(
            state.selected_channel,
            CHANNEL_COUNT,
            ListSelectionModifiers::new(),
        );
    }
}

fn is_reorder_noop(from: usize, insert: usize) -> bool {
    insert == from || insert == from + 1
}

#[derive(Clone, Debug)]
struct MixerPanelWidget {
    common: WidgetCommon,
    channels: [MixerChannel; CHANNEL_COUNT],
    selection: ListSelectionController,
    selected_channel: usize,
    frame: u64,
    hover_channel: Option<usize>,
    hover_position: Option<Point>,
    drag_target: Option<MixerDragTarget>,
    drag_preview_ratio: Option<f32>,
    drag_start_gains: Option<[f32; CHANNEL_COUNT]>,
    reorder_insert: Option<usize>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum MixerDragTarget {
    Fader(usize),
    Send { channel: usize, send: usize },
    Strip(usize),
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct MeterReadout {
    meter_db: f32,
    peak_db: f32,
}

impl MixerPanelWidget {
    fn new(
        channels: [MixerChannel; CHANNEL_COUNT],
        selection: ListSelectionController,
        selected_channel: usize,
        frame: u64,
    ) -> Self {
        let mut common = WidgetCommon::new(
            0,
            WidgetSizing::new(Vector2::new(1120.0, 460.0), Vector2::new(1400.0, 500.0)),
        );
        common.focus = FocusBehavior::Pointer;
        common.paint.bounds = PaintBounds::ClipToRect;
        common.paint.paints_focus = false;
        common.paint.paints_state_layers = false;
        Self {
            common,
            channels,
            selection,
            selected_channel,
            frame,
            hover_channel: None,
            hover_position: None,
            drag_target: None,
            drag_preview_ratio: None,
            drag_start_gains: None,
            reorder_insert: None,
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
        let gap = 4.0;
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
            Point::new(strip.min.x + strip.width() * 0.14, strip.min.y + 50.0),
            Point::new(strip.min.x + strip.width() * 0.42, strip.max.y - 150.0),
        )
    }

    fn fader_rect(&self, strip: Rect) -> Rect {
        Rect::from_min_max(
            Point::new(strip.min.x + strip.width() * 0.58, strip.min.y + 56.0),
            Point::new(strip.min.x + strip.width() * 0.86, strip.max.y - 150.0),
        )
    }

    fn send_rect(&self, strip: Rect, send: usize) -> Rect {
        let y = strip.max.y - 136.0 + send as f32 * 18.0;
        Rect::from_min_size(
            Point::new(strip.min.x + 5.0, y),
            Vector2::new(strip.width() - 10.0, 12.0),
        )
    }

    fn button_rect(&self, strip: Rect, index: usize) -> Rect {
        let width = (strip.width() - 10.0) / 3.0;
        let x = strip.min.x + 4.0 + index as f32 * (width + 1.0);
        Rect::from_min_size(
            Point::new(x, strip.max.y - 72.0),
            Vector2::new(width.max(1.0), 22.0),
        )
    }

    fn channel_at(&self, bounds: Rect, position: Point) -> Option<usize> {
        (0..CHANNEL_COUNT).find(|channel| self.strip_rect(bounds, *channel).contains(position))
    }

    fn insertion_index_at(&self, bounds: Rect, position: Point) -> usize {
        let console = self.console_rect(bounds);
        if position.x <= console.min.x {
            return 0;
        }
        if position.x >= console.max.x {
            return CHANNEL_COUNT;
        }
        for channel in 0..CHANNEL_COUNT {
            if position.x < self.strip_rect(bounds, channel).center().x {
                return channel;
            }
        }
        CHANNEL_COUNT
    }

    fn insertion_line_rect(&self, bounds: Rect, insert: usize) -> Rect {
        let console = self.console_rect(bounds);
        let insert = insert.min(CHANNEL_COUNT);
        let x = if insert == 0 {
            self.strip_rect(bounds, 0).min.x - 2.0
        } else if insert == CHANNEL_COUNT {
            self.strip_rect(bounds, CHANNEL_COUNT - 1).max.x + 2.0
        } else {
            let left = self.strip_rect(bounds, insert - 1);
            let right = self.strip_rect(bounds, insert);
            (left.max.x + right.min.x) * 0.5
        };
        Rect::from_min_max(
            Point::new(x - 2.0, console.min.y + 4.0),
            Point::new(x + 2.0, console.max.y - 4.0),
        )
    }

    fn fader_ratio_at(&self, strip: Rect, position: Point) -> f32 {
        let fader = self.fader_rect(strip);
        ((fader.max.y - position.y) / fader.height().max(1.0)).clamp(0.0, 1.0)
    }

    fn send_ratio_at(&self, strip: Rect, send: usize, position: Point) -> f32 {
        let send = self.send_rect(strip, send);
        ((position.x - send.min.x) / send.width().max(1.0)).clamp(0.0, 1.0)
    }

    fn send_at(&self, strip: Rect, position: Point) -> Option<usize> {
        (0..SEND_COUNT).find(|send| self.send_rect(strip, *send).contains(position))
    }

    fn apply_selection(&mut self, channel: usize, modifiers: PointerModifiers) {
        self.selected_channel = channel;
        self.selection
            .select(channel, CHANNEL_COUNT, list_selection_modifiers(modifiers));
    }

    fn drag_message(
        &self,
        bounds: Rect,
        target: MixerDragTarget,
        position: Point,
    ) -> MixerPanelMessage {
        match target {
            MixerDragTarget::Fader(channel) => {
                let strip = self.strip_rect(bounds, channel);
                MixerPanelMessage::SetGain {
                    channel,
                    ratio: self.fader_ratio_at(strip, position),
                    selection: None,
                }
            }
            MixerDragTarget::Send { channel, send } => {
                let strip = self.strip_rect(bounds, channel);
                MixerPanelMessage::SetSend {
                    channel,
                    send,
                    ratio: self.send_ratio_at(strip, send, position),
                }
            }
            MixerDragTarget::Strip(channel) => MixerPanelMessage::Reorder {
                from: channel,
                insert: self.insertion_index_at(bounds, position),
            },
        }
    }

    fn drag_ratio(&self, bounds: Rect, target: MixerDragTarget, position: Point) -> f32 {
        match target {
            MixerDragTarget::Fader(channel) => {
                let strip = self.strip_rect(bounds, channel);
                self.fader_ratio_at(strip, position)
            }
            MixerDragTarget::Send { channel, send } => {
                let strip = self.strip_rect(bounds, channel);
                self.send_ratio_at(strip, send, position)
            }
            MixerDragTarget::Strip(_) => 0.0,
        }
    }

    fn fader_display_ratio(&self, channel: usize) -> f32 {
        ratio_for_gain(self.fader_display_db(channel))
    }

    fn fader_display_db(&self, channel: usize) -> f32 {
        self.fader_display_db_for_drag(channel)
            .unwrap_or(self.channels[channel].gain_db)
    }

    fn fader_display_db_for_drag(&self, channel: usize) -> Option<f32> {
        if self.drag_target == Some(MixerDragTarget::Fader(channel))
            && let Some(ratio) = self.drag_preview_ratio
        {
            return Some(gain_for_ratio(ratio));
        }
        if let Some(MixerDragTarget::Fader(source_channel)) = self.drag_target
            && self.selection.is_selected(source_channel)
            && self.selection.is_selected(channel)
            && self.selection.selected_indices().len() > 1
            && let Some(ratio) = self.drag_preview_ratio
            && let Some(start_gains) = self.drag_start_gains
        {
            let delta = gain_for_ratio(ratio) - start_gains[source_channel];
            return Some((start_gains[channel] + delta).clamp(MIN_GAIN_DB, MAX_GAIN_DB));
        }
        None
    }

    fn meter_display_db_for_drag(&self, channel: usize) -> Option<f32> {
        let channel_state = self.channels[channel];
        self.fader_display_db_for_drag(channel)
            .map(|gain_db| preview_meter_db(channel_state.meter_db, channel_state.gain_db, gain_db))
    }

    fn peak_display_db_for_drag(&self, channel: usize) -> Option<f32> {
        let channel_state = self.channels[channel];
        self.fader_display_db_for_drag(channel)
            .map(|gain_db| preview_meter_db(channel_state.peak_db, channel_state.gain_db, gain_db))
    }

    fn send_display_ratio(&self, channel: usize, send: usize) -> f32 {
        if self.drag_target == Some(MixerDragTarget::Send { channel, send })
            && let Some(ratio) = self.drag_preview_ratio
        {
            ratio
        } else {
            self.channels[channel].sends[send]
        }
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
                if let Some(target) = self.drag_target {
                    match target {
                        MixerDragTarget::Fader(_) | MixerDragTarget::Send { .. } => {
                            self.drag_preview_ratio =
                                Some(self.drag_ratio(bounds, target, position));
                        }
                        MixerDragTarget::Strip(_) => {
                            self.reorder_insert = Some(self.insertion_index_at(bounds, position));
                        }
                    }
                }
                None
            }
            WidgetInput::PointerPress {
                position,
                button: PointerButton::Primary,
                modifiers,
            } if bounds.contains(position) => {
                let channel = self.channel_at(bounds, position)?;
                let strip = self.strip_rect(bounds, channel);
                self.hover_channel = Some(channel);
                if self.fader_rect(strip).contains(position) {
                    let selection_update = (!self.selection.is_selected(channel)
                        || modifiers.shift
                        || modifiers.command
                        || self.selection.selected_indices().len() <= 1)
                        .then_some(list_selection_modifiers(modifiers));
                    if selection_update.is_some() {
                        self.apply_selection(channel, modifiers);
                    }
                    self.drag_target = Some(MixerDragTarget::Fader(channel));
                    self.drag_preview_ratio = Some(self.fader_ratio_at(strip, position));
                    self.drag_start_gains = Some(self.channels.map(|channel| channel.gain_db));
                    return Some(WidgetOutput::custom(MixerPanelMessage::SetGain {
                        channel,
                        ratio: self.fader_ratio_at(strip, position),
                        selection: selection_update,
                    }));
                }
                if let Some(send) = self.send_at(strip, position) {
                    self.apply_selection(channel, modifiers);
                    self.drag_target = Some(MixerDragTarget::Send { channel, send });
                    self.drag_preview_ratio = Some(self.send_ratio_at(strip, send, position));
                    return Some(WidgetOutput::custom(MixerPanelMessage::SetSend {
                        channel,
                        send,
                        ratio: self.send_ratio_at(strip, send, position),
                    }));
                }
                let message = button_or_select_message(self, strip, channel, position, modifiers);
                self.apply_selection(channel, modifiers);
                if matches!(message, MixerPanelMessage::Select { .. }) {
                    self.drag_target = Some(MixerDragTarget::Strip(channel));
                    self.reorder_insert = Some(channel);
                }
                Some(WidgetOutput::custom(message))
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
                let drag = self.drag_target.take();
                self.drag_preview_ratio = None;
                self.drag_start_gains = None;
                let reorder_insert = self.reorder_insert.take();
                self.hover_channel = self.channel_at(bounds, position);
                drag.and_then(|target| match target {
                    MixerDragTarget::Fader(_) | MixerDragTarget::Send { .. } => Some(
                        WidgetOutput::custom(self.drag_message(bounds, target, position)),
                    ),
                    MixerDragTarget::Strip(channel) => {
                        let insert = reorder_insert
                            .unwrap_or_else(|| self.insertion_index_at(bounds, position));
                        (!is_reorder_noop(channel, insert)).then(|| {
                            WidgetOutput::custom(MixerPanelMessage::Reorder {
                                from: channel,
                                insert,
                            })
                        })
                    }
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
            self.hover_channel = previous.hover_channel;
            self.hover_position = previous.hover_position;
            self.drag_target = previous.drag_target;
            self.drag_preview_ratio = previous.drag_preview_ratio;
            self.drag_start_gains = previous.drag_start_gains;
            self.reorder_insert = previous.reorder_insert;
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
        if let Some(channel) = self.hover_channel {
            let strip = self.strip_rect(bounds, channel);
            push_stroke(
                primitives,
                self.common.id,
                strip,
                translucent(theme.highlight_cyan, 170),
                2.0,
            );
        }
        match self.drag_target {
            Some(MixerDragTarget::Fader(channel)) => {
                self.append_fader_drag_overlay(primitives, bounds, channel, theme);
            }
            Some(MixerDragTarget::Send { channel, send }) => {
                self.append_send_drag_overlay(primitives, bounds, channel, send, theme);
            }
            Some(MixerDragTarget::Strip(channel)) => {
                self.append_reorder_drag_overlay(primitives, bounds, channel, theme);
            }
            None => {}
        }
    }
}

impl MixerPanelWidget {
    fn should_paint_fader_overlay_for(&self, source_channel: usize, channel: usize) -> bool {
        if self.selection.is_selected(source_channel) && self.selection.selected_indices().len() > 1
        {
            self.selection.is_selected(channel)
        } else {
            channel == source_channel
        }
    }

    fn append_fader_drag_overlay(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        bounds: Rect,
        source_channel: usize,
        theme: &ThemeTokens,
    ) {
        for channel_index in 0..CHANNEL_COUNT {
            if !self.should_paint_fader_overlay_for(source_channel, channel_index) {
                continue;
            }
            let strip = self.strip_rect(bounds, channel_index);
            let fader = self.fader_rect(strip);
            let center_x = fader.center().x;
            self.append_meter_drag_overlay(primitives, channel_index, strip, theme);
            push_rect(
                primitives,
                self.common.id,
                Rect::from_min_max(
                    Point::new(fader.min.x - 2.0, fader.min.y - 12.0),
                    Point::new(fader.max.x + 2.0, fader.max.y + 12.0),
                ),
                translucent(theme.surface_base, 245),
            );
            push_rect(
                primitives,
                self.common.id,
                Rect::from_min_max(
                    Point::new(center_x - 2.0, fader.min.y),
                    Point::new(center_x + 2.0, fader.max.y),
                ),
                theme.grid_strong,
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
            let knob_y = fader.max.y - fader.height() * self.fader_display_ratio(channel_index);
            let knob = Rect::from_min_size(
                Point::new(fader.min.x, knob_y - 8.0),
                Vector2::new(fader.width(), 16.0),
            );
            push_rect(primitives, self.common.id, knob, theme.highlight_blue);
            push_stroke(primitives, self.common.id, knob, theme.border_emphasis, 1.0);
        }
    }

    fn append_meter_drag_overlay(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        channel_index: usize,
        strip: Rect,
        theme: &ThemeTokens,
    ) {
        let channel = self.channels[channel_index];
        let readout = MeterReadout {
            meter_db: self
                .meter_display_db_for_drag(channel_index)
                .unwrap_or(channel.meter_db),
            peak_db: self
                .peak_display_db_for_drag(channel_index)
                .unwrap_or(channel.peak_db),
        };
        self.append_meter_values(primitives, channel, strip, false, readout, theme);
    }

    fn append_send_drag_overlay(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        bounds: Rect,
        channel_index: usize,
        send: usize,
        theme: &ThemeTokens,
    ) {
        let strip = self.strip_rect(bounds, channel_index);
        let rect = self.send_rect(strip, send);
        push_rect(primitives, self.common.id, rect, theme.bg_tertiary);
        let fill = Rect::from_min_max(
            rect.min,
            Point::new(
                rect.min.x + rect.width() * self.send_display_ratio(channel_index, send),
                rect.max.y,
            ),
        );
        push_rect(primitives, self.common.id, fill, send_color(send, theme));
        push_stroke(primitives, self.common.id, rect, theme.border_emphasis, 1.0);
    }

    fn append_reorder_drag_overlay(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        bounds: Rect,
        source_channel: usize,
        theme: &ThemeTokens,
    ) {
        let source = self.strip_rect(bounds, source_channel);
        push_stroke(
            primitives,
            self.common.id,
            source,
            translucent(theme.text_primary, 135),
            2.0,
        );
        if let Some(insert) = self.reorder_insert {
            let line = self.insertion_line_rect(bounds, insert);
            push_rect(
                primitives,
                self.common.id,
                line,
                translucent(theme.highlight_cyan, 235),
            );
            push_stroke(primitives, self.common.id, line, theme.border_emphasis, 1.0);
        }
    }

    fn append_strip(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        bounds: Rect,
        channel_index: usize,
        theme: &ThemeTokens,
    ) {
        let channel = self.channels[channel_index];
        let strip = self.strip_rect(bounds, channel_index);
        let selected = self.selection.is_selected(channel_index);
        let solo_active = self.channels.iter().any(|channel| channel.solo);
        let solo_dimmed = channel.is_visually_dimmed_by_solo(solo_active);
        let group_tint = group_color(channel.group(), theme);
        let fill = if selected {
            blend_color(theme.surface_raised, theme.highlight_blue, 0.20)
        } else if solo_dimmed {
            blend_color(theme.surface_base, theme.bg_primary, 0.42)
        } else {
            blend_color(theme.surface_base, group_tint, 0.10)
        };
        push_rect(primitives, self.common.id, strip, fill);
        push_rect(
            primitives,
            self.common.id,
            Rect::from_min_max(
                Point::new(strip.min.x, strip.min.y),
                Point::new(strip.max.x, strip.min.y + 4.0),
            ),
            if solo_dimmed {
                rgba(78, 82, 88, 180)
            } else {
                group_tint
            },
        );
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
        self.append_fader(primitives, channel_index, strip, solo_dimmed, theme);
        self.append_sends(primitives, channel_index, strip, solo_dimmed, theme);
        self.append_channel_buttons(primitives, channel, strip, theme);
        push_text(
            primitives,
            self.common.id,
            format!("{:+.1} dB", self.fader_display_db(channel_index)),
            Rect::from_min_size(
                Point::new(strip.min.x + 4.0, strip.max.y - 44.0),
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
                Point::new(strip.min.x + 4.0, strip.max.y - 24.0),
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
        self.append_meter_values(
            primitives,
            channel,
            strip,
            solo_dimmed,
            MeterReadout {
                meter_db: channel.meter_db,
                peak_db: channel.peak_db,
            },
            theme,
        );
    }

    fn append_meter_values(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        channel: MixerChannel,
        strip: Rect,
        solo_dimmed: bool,
        readout: MeterReadout,
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
        let meter_ratio = ratio_for_meter_db(readout.meter_db);
        let left_ratio = meter_ratio
            * if channel.pan > 0.0 {
                1.0 - channel.pan * 0.55
            } else {
                1.0
            };
        let right_ratio = meter_ratio
            * if channel.pan < 0.0 {
                1.0 + channel.pan * 0.55
            } else {
                1.0
            };
        for (index, ratio) in [left_ratio, right_ratio].into_iter().enumerate() {
            let lane_gap = 2.0;
            let lane_width = ((meter.width() - 6.0 - lane_gap) / 2.0).max(1.0);
            let x = meter.min.x + 3.0 + index as f32 * (lane_width + lane_gap);
            let meter_fill = Rect::from_min_max(
                Point::new(x, meter.max.y - (meter.height() - 6.0) * ratio),
                Point::new(x + lane_width, meter.max.y - 3.0),
            );
            push_rect(
                primitives,
                self.common.id,
                meter_fill,
                if solo_dimmed {
                    rgba(75, 80, 86, 180)
                } else {
                    meter_color(readout.meter_db)
                },
            );
        }
        let peak_y = meter.max.y - meter.height() * ratio_for_meter_db(readout.peak_db);
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
            format!("{:+.0}", readout.meter_db),
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

    fn append_sends(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        channel_index: usize,
        strip: Rect,
        solo_dimmed: bool,
        theme: &ThemeTokens,
    ) {
        for send in 0..SEND_COUNT {
            let rect = self.send_rect(strip, send);
            push_rect(
                primitives,
                self.common.id,
                rect,
                if solo_dimmed {
                    rgba(24, 26, 29, 255)
                } else {
                    theme.bg_tertiary
                },
            );
            let fill = Rect::from_min_max(
                rect.min,
                Point::new(
                    rect.min.x + rect.width() * self.send_display_ratio(channel_index, send),
                    rect.max.y,
                ),
            );
            push_rect(
                primitives,
                self.common.id,
                fill,
                if solo_dimmed {
                    rgba(86, 92, 100, 170)
                } else {
                    send_color(send, theme)
                },
            );
            push_stroke(primitives, self.common.id, rect, theme.border, 1.0);
        }
    }

    fn append_fader(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        channel_index: usize,
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
        let knob_y = fader.max.y - fader.height() * self.fader_display_ratio(channel_index);
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
    modifiers: PointerModifiers,
) -> MixerPanelMessage {
    if widget.button_rect(strip, 0).contains(position) {
        MixerPanelMessage::ToggleMute(channel)
    } else if widget.button_rect(strip, 1).contains(position) {
        MixerPanelMessage::ToggleSolo(channel)
    } else if widget.button_rect(strip, 2).contains(position) {
        MixerPanelMessage::ToggleArm(channel)
    } else {
        MixerPanelMessage::Select {
            channel,
            modifiers: list_selection_modifiers(modifiers),
        }
    }
}

fn list_selection_modifiers(modifiers: PointerModifiers) -> ListSelectionModifiers {
    if modifiers.shift {
        ListSelectionModifiers::extend()
    } else if modifiers.command {
        ListSelectionModifiers::toggle()
    } else {
        ListSelectionModifiers::new()
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

fn preview_meter_db(current_meter_db: f32, current_gain_db: f32, preview_gain_db: f32) -> f32 {
    if preview_gain_db <= MIN_GAIN_DB + 0.001 {
        MIN_GAIN_DB
    } else {
        (current_meter_db + preview_gain_db - current_gain_db).clamp(MIN_GAIN_DB, 0.0)
    }
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

fn group_color(group: usize, theme: &ThemeTokens) -> Rgba8 {
    match group % GROUP_COUNT {
        0 => theme.highlight_cyan,
        1 => theme.highlight_blue,
        2 => theme.accent_warning,
        _ => theme.highlight_orange,
    }
}

fn send_color(send: usize, theme: &ThemeTokens) -> Rgba8 {
    match send % SEND_COUNT {
        0 => theme.highlight_cyan,
        1 => theme.highlight_blue,
        _ => theme.highlight_orange,
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
    use radiant::runtime::{Event, RuntimeBridge, SurfaceRuntime};

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
                selection: Some(ListSelectionModifiers::new()),
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
        let widget = MixerPanelWidget::new(
            state.channels,
            state.selection.clone(),
            state.selected_channel,
            state.frame,
        );
        let bounds = mixer_bounds();
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
    fn mixer_panel_paints_dense_channel_strips_sends_and_db_labels() {
        let state = MixerState::default();
        let widget = MixerPanelWidget::new(
            state.channels,
            state.selection.clone(),
            state.selected_channel,
            state.frame,
        );
        let bounds = mixer_bounds();
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
            primitives.len() > CHANNEL_COUNT * 24,
            "dense mixer should stress retained paint planning with many per-channel primitives"
        );
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
        let mut widget = MixerPanelWidget::new(
            state.channels,
            state.selection.clone(),
            state.selected_channel,
            state.frame,
        );
        let bounds = mixer_bounds();
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
        let mut widget = MixerPanelWidget::new(
            state.channels,
            state.selection.clone(),
            state.selected_channel,
            state.frame,
        );
        let bounds = mixer_bounds();
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
                ratio: 1.0,
                selection: Some(ListSelectionModifiers::new()),
            })
        );
        assert!(widget.prefers_pointer_move_paint_only());
    }

    #[test]
    fn mixer_panel_supports_shift_and_control_multi_channel_selection() {
        let state = MixerState::default();
        let mut widget = MixerPanelWidget::new(
            state.channels,
            state.selection.clone(),
            state.selected_channel,
            state.frame,
        );
        let bounds = mixer_bounds();

        press_strip_label(&mut widget, bounds, 2, PointerModifiers::default());
        press_strip_label(
            &mut widget,
            bounds,
            5,
            PointerModifiers {
                shift: true,
                ..Default::default()
            },
        );

        assert_eq!(widget.selection.selected_indices(), &[2, 3, 4, 5]);

        press_strip_label(
            &mut widget,
            bounds,
            7,
            PointerModifiers {
                command: true,
                ..Default::default()
            },
        );

        assert_eq!(widget.selection.selected_indices(), &[2, 3, 4, 5, 7]);
    }

    #[test]
    fn mixer_reorder_moves_channel_identity_and_preserves_selection() {
        let mut state = MixerState::default();
        update_panel(
            &mut state,
            MixerPanelMessage::Select {
                channel: 2,
                modifiers: ListSelectionModifiers::new(),
            },
        );
        update_panel(
            &mut state,
            MixerPanelMessage::Select {
                channel: 4,
                modifiers: ListSelectionModifiers::toggle(),
            },
        );
        let moved = state.channels[2];
        let also_selected = state.channels[4];

        update_panel(
            &mut state,
            MixerPanelMessage::Reorder { from: 2, insert: 7 },
        );

        assert_eq!(state.channels[6], moved);
        assert_eq!(state.selected().id, also_selected.id);
        assert_eq!(state.selection.selected_indices(), &[3, 6]);
    }

    #[test]
    fn mixer_strip_drag_paints_insertion_line_without_spreading_strips() {
        let state = MixerState::default();
        let mut widget = MixerPanelWidget::new(
            state.channels,
            state.selection.clone(),
            state.selected_channel,
            state.frame,
        );
        let bounds = mixer_bounds();
        let source = widget.strip_rect(bounds, 2);
        let target_line = widget.insertion_line_rect(bounds, 7);

        widget.handle_input(
            bounds,
            WidgetInput::PointerPress {
                position: Point::new(source.center().x, source.min.y + 22.0),
                button: PointerButton::Primary,
                modifiers: Default::default(),
            },
        );
        let move_output = widget.handle_input(
            bounds,
            WidgetInput::PointerMove {
                position: target_line.center(),
            },
        );

        assert!(move_output.is_none());
        assert_eq!(widget.drag_target, Some(MixerDragTarget::Strip(2)));
        assert_eq!(widget.reorder_insert, Some(7));
        let mut overlay = Vec::new();
        widget.append_runtime_overlay_paint(
            &mut overlay,
            bounds,
            &LayoutOutput::default(),
            &ThemeTokens::default(),
        );
        assert!(
            overlay.iter().any(|primitive| {
                matches!(
                    primitive,
                    PaintPrimitive::FillRect(fill)
                        if fill.rect == target_line
                            && fill.color == translucent(ThemeTokens::default().highlight_cyan, 235)
                )
            }),
            "strip reorder drag should light the insertion line without moving strips"
        );
    }

    #[test]
    fn mixer_strip_drag_drop_emits_reorder_message() {
        let state = MixerState::default();
        let mut widget = MixerPanelWidget::new(
            state.channels,
            state.selection.clone(),
            state.selected_channel,
            state.frame,
        );
        let bounds = mixer_bounds();
        let source = widget.strip_rect(bounds, 2);
        let target_line = widget.insertion_line_rect(bounds, 7);

        widget.handle_input(
            bounds,
            WidgetInput::PointerPress {
                position: Point::new(source.center().x, source.min.y + 22.0),
                button: PointerButton::Primary,
                modifiers: Default::default(),
            },
        );
        widget.handle_input(
            bounds,
            WidgetInput::PointerMove {
                position: target_line.center(),
            },
        );
        let output = widget.handle_input(
            bounds,
            WidgetInput::PointerRelease {
                position: target_line.center(),
                button: PointerButton::Primary,
                modifiers: Default::default(),
            },
        );

        assert_eq!(
            output.and_then(|output| output.typed_ref::<MixerPanelMessage>().copied()),
            Some(MixerPanelMessage::Reorder { from: 2, insert: 7 })
        );
    }

    #[test]
    fn mixer_group_fader_drag_applies_relative_gain_delta_to_selected_channels() {
        let mut state = MixerState::default();
        update_panel(
            &mut state,
            MixerPanelMessage::Select {
                channel: 4,
                modifiers: ListSelectionModifiers::new(),
            },
        );
        update_panel(
            &mut state,
            MixerPanelMessage::Select {
                channel: 5,
                modifiers: ListSelectionModifiers::toggle(),
            },
        );
        let initial_4 = state.channels[4].gain_db;
        let initial_5 = state.channels[5].gain_db;

        update_panel(
            &mut state,
            MixerPanelMessage::SetGain {
                channel: 4,
                ratio: 0.80,
                selection: None,
            },
        );

        let delta_4 = state.channels[4].gain_db - initial_4;
        let delta_5 = state.channels[5].gain_db - initial_5;
        assert_eq!(state.selection.selected_indices(), &[4, 5]);
        assert!((delta_4 - delta_5).abs() < 0.001);
    }

    #[test]
    fn mixer_panel_fader_drag_preview_survives_rebuild_without_jittering_to_stale_gain() {
        let state = MixerState::default();
        let mut widget = MixerPanelWidget::new(
            state.channels,
            state.selection.clone(),
            state.selected_channel,
            state.frame,
        );
        let bounds = mixer_bounds();
        let strip = widget.strip_rect(bounds, 4);
        let fader = widget.fader_rect(strip);

        widget.handle_input(
            bounds,
            WidgetInput::PointerPress {
                position: Point::new(fader.center().x, fader.min.y),
                button: PointerButton::Primary,
                modifiers: Default::default(),
            },
        );
        let drag_output = widget.handle_input(
            bounds,
            WidgetInput::PointerMove {
                position: Point::new(fader.center().x, fader.max.y),
            },
        );
        assert!(drag_output.is_none());

        let mut rebuilt = MixerPanelWidget::new(
            state.channels,
            state.selection.clone(),
            state.selected_channel,
            state.frame,
        );
        rebuilt.synchronize_from_previous(&widget);

        assert_eq!(rebuilt.drag_target, Some(MixerDragTarget::Fader(4)));
        assert_eq!(rebuilt.drag_preview_ratio, Some(0.0));
        assert_eq!(rebuilt.fader_display_ratio(4), 0.0);
        let mut overlay = Vec::new();
        rebuilt.append_runtime_overlay_paint(
            &mut overlay,
            bounds,
            &LayoutOutput::default(),
            &ThemeTokens::default(),
        );
        let expected_knob_y = fader.max.y;
        let meter = rebuilt.meter_rect(strip);
        assert!(
            overlay.iter().any(|primitive| {
                matches!(
                    primitive,
                    PaintPrimitive::FillRect(fill)
                        if fill.color == ThemeTokens::default().highlight_blue
                            && (fill.rect.min.y - (expected_knob_y - 8.0)).abs() < 0.001
                )
            }),
            "paint-only runtime overlay should draw the live fader knob preview"
        );
        assert!(
            overlay.iter().any(|primitive| {
                matches!(
                    primitive,
                    PaintPrimitive::Text(text)
                        if text.text.as_str() == "-60"
                            && text.rect.min.y > meter.max.y
                            && text.rect.min.x < meter.max.x
                            && text.rect.max.x > meter.min.x
                )
            }),
            "paint-only runtime overlay should redraw the meter readout from the preview gain"
        );
        assert_ne!(
            rebuilt.fader_display_ratio(4),
            ratio_for_gain(state.channels[4].gain_db),
            "active fader drags should paint from the pointer preview instead of stale host state"
        );
    }

    #[test]
    fn mixer_group_fader_drag_preview_moves_selected_channels_together() {
        let mut state = MixerState::default();
        state
            .selection
            .select(4, CHANNEL_COUNT, ListSelectionModifiers::new());
        state
            .selection
            .select(5, CHANNEL_COUNT, ListSelectionModifiers::toggle());
        state.selected_channel = 4;
        let mut widget = MixerPanelWidget::new(
            state.channels,
            state.selection.clone(),
            state.selected_channel,
            state.frame,
        );
        let bounds = mixer_bounds();
        let strip = widget.strip_rect(bounds, 4);
        let fader = widget.fader_rect(strip);

        widget.handle_input(
            bounds,
            WidgetInput::PointerPress {
                position: Point::new(fader.center().x, fader.min.y),
                button: PointerButton::Primary,
                modifiers: Default::default(),
            },
        );
        let drag_output = widget.handle_input(
            bounds,
            WidgetInput::PointerMove {
                position: Point::new(fader.center().x, fader.min.y + fader.height() * 0.20),
            },
        );
        assert!(drag_output.is_none());

        let mut rebuilt = MixerPanelWidget::new(
            state.channels,
            state.selection.clone(),
            state.selected_channel,
            state.frame,
        );
        rebuilt.synchronize_from_previous(&widget);

        let delta_4 = rebuilt.fader_display_db(4) - state.channels[4].gain_db;
        let delta_5 = rebuilt.fader_display_db(5) - state.channels[5].gain_db;
        assert_eq!(rebuilt.drag_target, Some(MixerDragTarget::Fader(4)));
        assert!((delta_4 - delta_5).abs() < 0.001);
    }

    #[test]
    fn mixer_panel_send_drag_routes_dense_aux_control_change() {
        let state = MixerState::default();
        let mut widget = MixerPanelWidget::new(
            state.channels,
            state.selection.clone(),
            state.selected_channel,
            state.frame,
        );
        let bounds = mixer_bounds();
        let strip = widget.strip_rect(bounds, 17);
        let send = widget.send_rect(strip, 2);

        let output = widget.handle_input(
            bounds,
            WidgetInput::PointerPress {
                position: Point::new(send.min.x + send.width() * 0.75, send.center().y),
                button: PointerButton::Primary,
                modifiers: Default::default(),
            },
        );

        assert_eq!(
            output.and_then(|output| output.typed_ref::<MixerPanelMessage>().copied()),
            Some(MixerPanelMessage::SetSend {
                channel: 17,
                send: 2,
                ratio: 0.75
            })
        );
        assert_eq!(
            widget.drag_target,
            Some(MixerDragTarget::Send {
                channel: 17,
                send: 2
            })
        );
        assert!(widget.prefers_pointer_move_paint_only());
    }

    #[test]
    fn mixer_runtime_hover_does_not_refresh_surface() {
        let bridge = mixer_test_bridge(MixerState::default());
        let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(1440.0, 760.0));
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
    fn mixer_runtime_fader_drag_motion_uses_paint_only_preview_until_release() {
        let state = MixerState::default();
        let bridge = mixer_test_bridge(state.clone());
        let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(1440.0, 760.0));
        let bounds = runtime.layout().rects[&MIXER_WIDGET_ID];
        let widget = MixerPanelWidget::new(
            state.channels,
            state.selection.clone(),
            state.selected_channel,
            state.frame,
        );
        let strip = widget.strip_rect(bounds, 4);
        let fader = widget.fader_rect(strip);
        let press = Point::new(fader.center().x, fader.min.y);
        let drag = Point::new(fader.center().x, fader.min.y + fader.height() * 0.35);

        runtime.dispatch_event(Event::PointerPress {
            position: press,
            button: PointerButton::Primary,
            modifiers: PointerModifiers::default(),
        });
        let _ = runtime.take_repaint_requested();
        let first_drag = runtime.dispatch_pointer_move_with_outcome(drag);
        assert!(
            first_drag.needs_scene_rebuild(),
            "the first drag move may establish captured hover state"
        );
        let move_outcome =
            runtime.dispatch_pointer_move_with_outcome(Point::new(drag.x, drag.y + 12.0));

        assert!(move_outcome.paint_only_requested);
        assert!(
            !move_outcome.needs_scene_rebuild(),
            "fader drag motion should repaint the local preview without reducer churn"
        );

        runtime.dispatch_event(Event::PointerRelease {
            position: drag,
            button: PointerButton::Primary,
            modifiers: PointerModifiers::default(),
        });
        assert!(
            runtime.take_repaint_requested(),
            "release should commit the final gain and request a normal surface refresh"
        );
    }

    #[test]
    fn mixer_runtime_frame_messages_advance_status() {
        let bridge = mixer_test_bridge(MixerState::default());
        let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(1440.0, 760.0));
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

    fn mixer_bounds() -> Rect {
        Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(1400.0, 500.0))
    }

    fn press_strip_label(
        widget: &mut MixerPanelWidget,
        bounds: Rect,
        channel: usize,
        modifiers: PointerModifiers,
    ) -> Option<MixerPanelMessage> {
        let strip = widget.strip_rect(bounds, channel);
        let position = Point::new(strip.center().x, strip.min.y + 22.0);
        let output = widget
            .handle_input(
                bounds,
                WidgetInput::PointerPress {
                    position,
                    button: PointerButton::Primary,
                    modifiers,
                },
            )
            .and_then(|output| output.typed_ref::<MixerPanelMessage>().copied());
        let _ = widget.handle_input(
            bounds,
            WidgetInput::PointerRelease {
                position,
                button: PointerButton::Primary,
                modifiers,
            },
        );
        output
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
