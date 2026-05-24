use super::{
    DATA_SOURCE_NOTE, MIXER_WIDGET_ID, MixerChannel, MixerMessage, MixerPanelWidget, MixerState,
    STATUS_WIDGET_ID,
};
use radiant::prelude::*;

pub(crate) fn project_surface(state: &mut MixerState) -> View<MixerMessage> {
    let selected = state.selected();
    column([
        header_row(state.running),
        mixer_panel(state),
        status_row(state, selected),
    ])
    .style(WidgetStyle::default())
    .padding(16.0)
    .spacing(12.0)
    .fill()
}

fn header_row(running: bool) -> View<MixerMessage> {
    row([
        text("32-Channel Mixer").height(30.0).fill_width(),
        button(if running { "Pause" } else { "Run" })
            .primary()
            .message(MixerMessage::ToggleRun)
            .size(88.0, 30.0),
        button("Reset")
            .subtle()
            .message(MixerMessage::Reset)
            .size(82.0, 30.0),
    ])
    .fill_width()
    .spacing(10.0)
}

fn mixer_panel(state: &MixerState) -> View<MixerMessage> {
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
    .fill_width()
}

fn status_row(state: &MixerState, selected: MixerChannel) -> View<MixerMessage> {
    row([
        channel_summary_tile(selected),
        stat_tile("Source", DATA_SOURCE_NOTE),
        stat_tile("Peak", format!("{:+.1} dB", selected.meter.peak_db)),
        stat_tile(
            "Send A",
            format!("{:.0}%", selected.controls.sends[0] * 100.0),
        ),
        stat_tile("Pan", format!("{:+.0}%", selected.controls.pan * 100.0)),
        text(state.status())
            .id(STATUS_WIDGET_ID)
            .height(68.0)
            .fill_width(),
    ])
    .fill_width()
    .spacing(10.0)
}

fn channel_summary_tile(channel: MixerChannel) -> View<MixerMessage> {
    stat_tile(
        format!("Selected {}", channel.label),
        format!("{:+.1} dB fader", channel.controls.gain_db),
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
