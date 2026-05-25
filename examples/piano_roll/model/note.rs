#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct PianoNote {
    pub(crate) id: u32,
    pub(crate) pitch: i32,
    pub(crate) start_beat: f32,
    pub(crate) length_beats: f32,
    pub(crate) velocity: f32,
}

impl PianoNote {
    pub(super) const fn new(
        id: u32,
        pitch: i32,
        start_beat: f32,
        length_beats: f32,
        velocity: f32,
    ) -> Self {
        Self {
            id,
            pitch,
            start_beat,
            length_beats,
            velocity,
        }
    }

    pub(crate) fn end_beat(self) -> f32 {
        self.start_beat + self.length_beats
    }
}
