use radiant::prelude::*;

#[path = "view/components.rs"]
mod components;

use super::{
    ARRANGEMENT_WIDGET_ID, AppMessage, ArrangementOverviewWidget, BROWSER_ITEMS, DATA_SOURCE_NOTE,
    STATUS_WIDGET_ID, ShellMessage, TRACKS, model::ArrangementShellState,
};
use components::{
    collapsed_panel, meter_tile, panel_style, panel_toggle_label, stat_tile, subtle_style,
    track_button_style,
};

pub(crate) fn project_surface(state: &mut ArrangementShellState) -> View<AppMessage> {
    column([
        transport_bar(state),
        row([
            browser_panel(state),
            arrangement_panel(state),
            inspector_panel(state),
        ])
        .fill()
        .spacing(10.0),
        mixer_strip(state),
    ])
    .style(WidgetStyle::default())
    .padding(14.0)
    .spacing(10.0)
    .fill()
}

fn transport_bar(state: &ArrangementShellState) -> View<AppMessage> {
    row([
        text("Arrangement Shell").height(30.0).fill_width(),
        button(panel_toggle_label(state.panels.browser_open, "Browser"))
            .subtle()
            .message(AppMessage::Shell(ShellMessage::ToggleBrowser))
            .size(126.0, 30.0),
        button(panel_toggle_label(state.panels.inspector_open, "Inspector"))
            .subtle()
            .message(AppMessage::Shell(ShellMessage::ToggleInspector))
            .size(136.0, 30.0),
        button(if state.running { "Pause" } else { "Run" })
            .primary()
            .message(AppMessage::ToggleRun)
            .size(86.0, 30.0),
        button("Reset")
            .subtle()
            .message(AppMessage::Reset)
            .size(78.0, 30.0),
    ])
    .fill_width()
    .spacing(10.0)
}

fn browser_panel(state: &ArrangementShellState) -> View<AppMessage> {
    if !state.panels.browser_open {
        return collapsed_panel("Browser", 92.0);
    }
    let mut rows = Vec::new();
    rows.push(text("Browser").height(24.0).fill_width());
    for item in BROWSER_ITEMS {
        rows.push(text(item).height(26.0).fill_width().style(subtle_style()));
    }
    column(rows)
        .style(panel_style())
        .padding(10.0)
        .spacing(6.0)
        .size(168.0, 400.0)
}

fn arrangement_panel(state: &ArrangementShellState) -> View<AppMessage> {
    column([
        track_selector_row(state),
        custom_widget_mapped(
            ArrangementOverviewWidget::new(
                state.clips.clone(),
                state.selected_clip,
                state.playhead_beat,
            ),
            AppMessage::Shell,
        )
        .id(ARRANGEMENT_WIDGET_ID)
        .height(390.0)
        .fill_width(),
    ])
    .style(panel_style())
    .padding(10.0)
    .spacing(10.0)
    .fill()
}

fn inspector_panel(state: &ArrangementShellState) -> View<AppMessage> {
    if !state.panels.inspector_open {
        return collapsed_panel("Inspector", 104.0);
    }
    let selected = state.selected_clip();
    column([
        text("Inspector").height(24.0).fill_width(),
        stat_tile(
            "Selected",
            selected.map(|clip| clip.label).unwrap_or("Track"),
        ),
        stat_tile(
            "Track",
            selected
                .map(|clip| TRACKS[clip.track])
                .unwrap_or(TRACKS[state.selected_track]),
        ),
        stat_tile(
            "Beat",
            selected
                .map(|clip| format!("{:.1} - {:.1}", clip.start_beat, clip.end_beat()))
                .unwrap_or_else(|| format!("{:.1}", state.playhead_beat)),
        ),
        stat_tile("Source", DATA_SOURCE_NOTE),
    ])
    .style(panel_style())
    .padding(10.0)
    .spacing(8.0)
    .size(196.0, 400.0)
}

fn mixer_strip(state: &ArrangementShellState) -> View<AppMessage> {
    let mut tracks = Vec::new();
    tracks.push(
        text(state.status())
            .id(STATUS_WIDGET_ID)
            .height(64.0)
            .fill_width(),
    );
    for meter in state.mixer {
        tracks.push(meter_tile(meter));
    }
    row(tracks).fill_width().spacing(10.0)
}

fn track_selector_row(state: &ArrangementShellState) -> View<AppMessage> {
    row(TRACKS
        .iter()
        .enumerate()
        .map(|(track, label)| {
            button(*label)
                .style(track_button_style(track == state.selected_track))
                .message(AppMessage::Shell(ShellMessage::SelectTrack(track)))
                .height(28.0)
                .fill_width()
        })
        .collect::<Vec<_>>())
    .fill_width()
    .spacing(8.0)
}
