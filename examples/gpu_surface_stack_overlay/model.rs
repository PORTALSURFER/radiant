#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ResizeHandle {
    Start,
    End,
}

#[derive(Debug)]
pub(super) struct DemoState {
    pub(super) selected: bool,
    pub(super) running: bool,
    pub(super) selection_start: f32,
    pub(super) selection_end: f32,
}

impl Default for DemoState {
    fn default() -> Self {
        Self {
            selected: false,
            running: true,
            selection_start: 0.22,
            selection_end: 0.68,
        }
    }
}

impl DemoState {
    pub(super) fn commit_selection(&mut self, start: f32, end: f32) {
        self.selection_start = start;
        self.selection_end = end;
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum DemoMessage {
    ToggleSelection,
    ToggleAnimation,
    CommitResize { start: f32, end: f32 },
}
