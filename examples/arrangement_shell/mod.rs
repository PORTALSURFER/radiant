#[path = "geometry.rs"]
mod geometry;
#[path = "model.rs"]
mod model;
#[path = "paint.rs"]
mod paint;
#[cfg(test)]
#[path = "tests.rs"]
mod tests;
#[path = "view.rs"]
mod view;
#[path = "widget.rs"]
mod widget;
#[path = "widget_paint.rs"]
mod widget_paint;

pub(crate) use model::ArrangementShellState;
pub(crate) use view::project_surface;
pub(crate) use widget::ArrangementOverviewWidget;

pub(crate) const ARRANGEMENT_WIDGET_ID: u64 = 96;
pub(crate) const STATUS_WIDGET_ID: u64 = 97;
pub(crate) const TRACK_COUNT: usize = 5;
pub(crate) const TOTAL_BEATS: f32 = 32.0;
pub(crate) const DATA_SOURCE_NOTE: &str = "without_audio_or_dsp";

pub(crate) const TRACKS: [&str; TRACK_COUNT] = ["Drums", "Bass", "Keys", "Pads", "Lead"];
pub(crate) const BROWSER_ITEMS: [&str; 6] = [
    "Kick Loop",
    "Snare Fill",
    "Sub Bass",
    "Chord Stab",
    "Pad Bed",
    "Lead Hook",
];

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum AppMessage {
    Frame,
    ToggleRun,
    Reset,
    Shell(ShellMessage),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum ShellMessage {
    SelectTrack(usize),
    SelectClip(u32),
    Seek { beat: f32 },
    ToggleBrowser,
    ToggleInspector,
}

pub(crate) fn update(state: &mut ArrangementShellState, message: AppMessage) {
    match message {
        AppMessage::Frame => state.tick(),
        AppMessage::ToggleRun => {
            state.running = !state.running;
        }
        AppMessage::Reset => state.reset(),
        AppMessage::Shell(message) => state.apply_shell_message(message),
    }
}
