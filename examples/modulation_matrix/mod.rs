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

pub(crate) use model::ModulationMatrixState;
pub(crate) use view::project_surface;
pub(crate) use widget::ModulationMatrixWidget;

pub(crate) const MATRIX_WIDGET_ID: u64 = 94;
pub(crate) const STATUS_WIDGET_ID: u64 = 95;
pub(crate) const SOURCE_COUNT: usize = 6;
pub(crate) const DESTINATION_COUNT: usize = 8;
pub(crate) const DATA_SOURCE_NOTE: &str = "without_synth_or_dsp";

pub(crate) const SOURCES: [&str; SOURCE_COUNT] = ["LFO 1", "LFO 2", "Env 1", "Env 2", "Vel", "Mod"];
pub(crate) const DESTINATIONS: [&str; DESTINATION_COUNT] = [
    "Cutoff", "Res", "Drive", "Pan", "Pitch", "PWM", "Mix", "Level",
];

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum AppMessage {
    Frame,
    ToggleRun,
    Reset,
    Matrix(MatrixMessage),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum MatrixMessage {
    SetAmount { cell: MatrixCell, amount: f32 },
    ClearSelected,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct MatrixCell {
    pub(crate) source: usize,
    pub(crate) destination: usize,
}

impl MatrixCell {
    pub(crate) fn clamped(self) -> Self {
        Self {
            source: self.source.min(SOURCE_COUNT - 1),
            destination: self.destination.min(DESTINATION_COUNT - 1),
        }
    }
}

pub(crate) fn update(state: &mut ModulationMatrixState, message: AppMessage) {
    match message {
        AppMessage::Frame => state.tick(),
        AppMessage::ToggleRun => {
            state.running = !state.running;
        }
        AppMessage::Reset => state.reset(),
        AppMessage::Matrix(message) => state.apply_matrix_message(message),
    }
}
