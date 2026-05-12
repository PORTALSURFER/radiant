#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum AnimationMessage {
    Toggle,
    Frame,
    Reset,
}

#[derive(Clone, Debug)]
pub(super) struct AnimationState {
    pub(super) running: bool,
    pub(super) frame: u64,
    pub(super) phase: f32,
}

impl Default for AnimationState {
    fn default() -> Self {
        Self {
            running: true,
            frame: 0,
            phase: 0.0,
        }
    }
}

impl AnimationState {
    pub(super) fn status(&self) -> String {
        if self.running {
            format!("Running | frame {} | phase {:.2}", self.frame, self.phase)
        } else {
            format!("Paused | frame {} | phase {:.2}", self.frame, self.phase)
        }
    }

    pub(super) fn tick(&mut self) {
        if self.running {
            self.frame = self.frame.saturating_add(1);
            self.phase = ((self.frame % 180) as f32) / 180.0;
        }
    }

    pub(super) fn reset(&mut self) {
        self.running = false;
        self.frame = 0;
        self.phase = 0.0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn animation_state_advances_meter_phase() {
        let mut state = AnimationState::default();

        state.tick();

        assert_eq!(state.frame, 1);
        assert!(state.phase > 0.0);
    }
}
