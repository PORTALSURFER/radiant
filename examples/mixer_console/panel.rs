use super::model::{CHANNEL_COUNT, MAX_GAIN_DB, MixerChannel, gain_for_ratio, ratio_for_gain};
use radiant::prelude::*;
use radiant::widgets::PaintBounds;

#[path = "panel/geometry.rs"]
mod geometry;
#[path = "panel/input.rs"]
mod input;
#[path = "panel/interaction.rs"]
mod interaction;

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

fn preview_meter_db(current_meter_db: f32, current_gain_db: f32, preview_gain_db: f32) -> f32 {
    let delta = preview_gain_db - current_gain_db;
    if preview_gain_db <= super::model::MIN_GAIN_DB + 0.001 {
        super::model::MIN_GAIN_DB
    } else {
        (current_meter_db + delta).clamp(super::model::MIN_GAIN_DB, 0.0)
    }
}
