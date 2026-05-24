use super::{DESTINATION_COUNT, DESTINATIONS, MatrixCell, MatrixMessage, SOURCE_COUNT, SOURCES};

#[derive(Clone, Debug)]
pub(crate) struct ModulationMatrixState {
    pub(crate) running: bool,
    pub(crate) frame: u64,
    pub(crate) activity_phase: f32,
    pub(crate) selected: MatrixCell,
    pub(crate) amounts: [[f32; DESTINATION_COUNT]; SOURCE_COUNT],
}

impl Default for ModulationMatrixState {
    fn default() -> Self {
        Self {
            running: true,
            frame: 0,
            activity_phase: 0.0,
            selected: MatrixCell {
                source: 0,
                destination: 2,
            },
            amounts: seeded_amounts(),
        }
    }
}

impl ModulationMatrixState {
    pub(crate) fn tick(&mut self) {
        if !self.running {
            return;
        }
        self.frame = self.frame.saturating_add(1);
        self.activity_phase = (self.activity_phase + 0.035) % 1.0;
    }

    pub(crate) fn reset(&mut self) {
        *self = Self::default();
    }

    pub(crate) fn selected_amount(&self) -> f32 {
        self.amounts[self.selected.source][self.selected.destination]
    }

    pub(crate) fn status(&self) -> String {
        let transport = if self.running { "running" } else { "paused" };
        format!(
            "{transport} | frame {} | {} -> {} | {:+.0}% | synthetic GUI routing",
            self.frame,
            SOURCES[self.selected.source],
            DESTINATIONS[self.selected.destination],
            self.selected_amount() * 100.0
        )
    }

    pub(crate) fn apply_matrix_message(&mut self, message: MatrixMessage) {
        match message {
            MatrixMessage::SetAmount { cell, amount } => {
                let cell = cell.clamped();
                self.amounts[cell.source][cell.destination] = amount.clamp(-1.0, 1.0);
                self.selected = cell;
            }
            MatrixMessage::ClearSelected => {
                self.amounts[self.selected.source][self.selected.destination] = 0.0;
            }
        }
    }
}

pub(crate) fn seeded_amounts() -> [[f32; DESTINATION_COUNT]; SOURCE_COUNT] {
    let mut amounts = [[0.0; DESTINATION_COUNT]; SOURCE_COUNT];
    amounts[0][0] = 0.64;
    amounts[0][3] = 0.28;
    amounts[1][4] = -0.32;
    amounts[2][0] = 0.78;
    amounts[2][6] = 0.36;
    amounts[3][2] = -0.46;
    amounts[4][7] = 0.52;
    amounts[5][1] = 0.24;
    amounts
}
