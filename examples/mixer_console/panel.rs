use super::model::{
    CHANNEL_COUNT, MAX_GAIN_DB, MixerChannel, SEND_COUNT, gain_for_ratio, ratio_for_gain,
};
use super::paint::{push_stroke, translucent};
use super::{MixerPanelMessage, is_reorder_noop};
use radiant::prelude::*;
use radiant::widgets::{PaintBounds, PointerModifiers};

#[derive(Clone, Debug)]
pub(crate) struct MixerPanelWidget {
    pub(super) common: WidgetCommon,
    pub(super) channels: [MixerChannel; CHANNEL_COUNT],
    pub(super) selection: ListSelectionController,
    pub(super) selected_channel: usize,
    pub(super) frame: u64,
    pub(crate) hover_channel: Option<usize>,
    hover_position: Option<Point>,
    pub(crate) drag_target: Option<MixerDragTarget>,
    pub(super) drag_preview_ratio: Option<f32>,
    drag_start_gains: Option<[f32; CHANNEL_COUNT]>,
    pub(crate) reorder_insert: Option<usize>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum MixerDragTarget {
    Fader(usize),
    Send { channel: usize, send: usize },
    Strip(usize),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct MeterReadout {
    pub(super) meter_db: f32,
    pub(super) peak_db: f32,
}

impl MixerPanelWidget {
    pub(crate) fn new(
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

    pub(crate) fn strip_rect(&self, bounds: Rect, channel: usize) -> Rect {
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

    pub(crate) fn fader_rect(&self, strip: Rect) -> Rect {
        Rect::from_min_max(
            Point::new(strip.min.x + strip.width() * 0.58, strip.min.y + 56.0),
            Point::new(strip.min.x + strip.width() * 0.86, strip.max.y - 150.0),
        )
    }

    pub(super) fn meter_rect(&self, strip: Rect) -> Rect {
        Rect::from_min_max(
            Point::new(strip.min.x + strip.width() * 0.14, strip.min.y + 50.0),
            Point::new(strip.min.x + strip.width() * 0.42, strip.max.y - 150.0),
        )
    }

    pub(super) fn send_rect(&self, strip: Rect, send: usize) -> Rect {
        let y = strip.max.y - 136.0 + send as f32 * 18.0;
        Rect::from_min_size(
            Point::new(strip.min.x + 5.0, y),
            Vector2::new(strip.width() - 10.0, 12.0),
        )
    }

    pub(super) fn button_rect(&self, strip: Rect, index: usize) -> Rect {
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

    pub(crate) fn insertion_index_at(&self, bounds: Rect, position: Point) -> usize {
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

    pub(crate) fn insertion_line_rect(&self, bounds: Rect, insert: usize) -> Rect {
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

    pub(super) fn fader_display_ratio(&self, channel: usize) -> f32 {
        ratio_for_gain(self.fader_display_db(channel))
    }

    pub(super) fn fader_display_db(&self, channel: usize) -> f32 {
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
            return Some(
                (start_gains[channel] + delta).clamp(super::model::MIN_GAIN_DB, MAX_GAIN_DB),
            );
        }
        None
    }

    pub(super) fn meter_display_db_for_drag(&self, channel: usize) -> Option<f32> {
        let channel_state = self.channels[channel];
        self.fader_display_db_for_drag(channel)
            .map(|gain_db| preview_meter_db(channel_state.meter_db, channel_state.gain_db, gain_db))
    }

    pub(super) fn peak_display_db_for_drag(&self, channel: usize) -> Option<f32> {
        let channel_state = self.channels[channel];
        self.fader_display_db_for_drag(channel)
            .map(|gain_db| preview_meter_db(channel_state.peak_db, channel_state.gain_db, gain_db))
    }

    pub(super) fn send_display_ratio(&self, channel: usize, send: usize) -> f32 {
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
            WidgetInput::PointerMove { position } => self.handle_pointer_move(bounds, position),
            WidgetInput::PointerPress {
                position,
                button: PointerButton::Primary,
                modifiers,
            } if bounds.contains(position) => {
                self.handle_primary_press(bounds, position, modifiers)
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
            } => self.handle_primary_release(bounds, position),
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
        self.append_console_paint(primitives, bounds, theme);
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
    fn handle_pointer_move(&mut self, bounds: Rect, position: Point) -> Option<WidgetOutput> {
        self.common.state.hovered = bounds.contains(position);
        self.hover_position = bounds.contains(position).then_some(position);
        self.hover_channel = self.channel_at(bounds, position);
        if let Some(target) = self.drag_target {
            match target {
                MixerDragTarget::Fader(_) | MixerDragTarget::Send { .. } => {
                    self.drag_preview_ratio = Some(self.drag_ratio(bounds, target, position));
                }
                MixerDragTarget::Strip(_) => {
                    self.reorder_insert = Some(self.insertion_index_at(bounds, position));
                }
            }
        }
        None
    }

    fn handle_primary_press(
        &mut self,
        bounds: Rect,
        position: Point,
        modifiers: PointerModifiers,
    ) -> Option<WidgetOutput> {
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

    fn handle_primary_release(&mut self, bounds: Rect, position: Point) -> Option<WidgetOutput> {
        let drag = self.drag_target.take();
        self.drag_preview_ratio = None;
        self.drag_start_gains = None;
        let reorder_insert = self.reorder_insert.take();
        self.hover_channel = self.channel_at(bounds, position);
        drag.and_then(|target| match target {
            MixerDragTarget::Fader(_) | MixerDragTarget::Send { .. } => Some(WidgetOutput::custom(
                self.drag_message(bounds, target, position),
            )),
            MixerDragTarget::Strip(channel) => {
                let insert =
                    reorder_insert.unwrap_or_else(|| self.insertion_index_at(bounds, position));
                (!is_reorder_noop(channel, insert)).then(|| {
                    WidgetOutput::custom(MixerPanelMessage::Reorder {
                        from: channel,
                        insert,
                    })
                })
            }
        })
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

fn preview_meter_db(current_meter_db: f32, current_gain_db: f32, preview_gain_db: f32) -> f32 {
    let delta = preview_gain_db - current_gain_db;
    if preview_gain_db <= super::model::MIN_GAIN_DB + 0.001 {
        super::model::MIN_GAIN_DB
    } else {
        (current_meter_db + delta).clamp(super::model::MIN_GAIN_DB, 0.0)
    }
}
