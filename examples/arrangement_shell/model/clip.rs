use super::TRACKS;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct ArrangementClip {
    pub(crate) id: u32,
    pub(crate) track: usize,
    pub(crate) start_beat: f32,
    pub(crate) length_beats: f32,
    pub(crate) label: &'static str,
}

impl ArrangementClip {
    const fn new(
        id: u32,
        track: usize,
        start_beat: f32,
        length_beats: f32,
        label: &'static str,
    ) -> Self {
        Self {
            id,
            track,
            start_beat,
            length_beats,
            label,
        }
    }

    pub(crate) fn end_beat(self) -> f32 {
        self.start_beat + self.length_beats
    }
}

pub(super) fn default_clips() -> Vec<ArrangementClip> {
    vec![
        ArrangementClip::new(1, 0, 0.0, 4.0, "Intro"),
        ArrangementClip::new(2, 1, 2.0, 6.0, "Bass A"),
        ArrangementClip::new(3, 2, 4.0, 4.0, "Keys A"),
        ArrangementClip::new(4, 3, 8.0, 8.0, "Pad Rise"),
        ArrangementClip::new(5, 4, 10.0, 4.0, "Lead"),
        ArrangementClip::new(6, 1, 14.0, 8.0, "Bass B"),
        ArrangementClip::new(7, 2, 16.0, 6.0, "Keys B"),
        ArrangementClip::new(8, 0, 24.0, 4.0, "Outro"),
    ]
}

pub(super) fn selected_clip_status(clip: ArrangementClip) -> String {
    format!(
        "{} on {} beat {:.1}",
        clip.label, TRACKS[clip.track], clip.start_beat
    )
}
