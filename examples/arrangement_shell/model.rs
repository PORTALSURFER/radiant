use super::{ShellMessage, TOTAL_BEATS, TRACK_COUNT, TRACKS};

#[derive(Clone, Debug)]
pub(crate) struct ArrangementShellState {
    pub(crate) running: bool,
    pub(crate) frame: u64,
    pub(crate) playhead_beat: f32,
    pub(crate) selected_track: usize,
    pub(crate) selected_clip: Option<u32>,
    pub(crate) panels: PanelVisibility,
    pub(crate) clips: Vec<ArrangementClip>,
    pub(crate) mixer: [TrackMeter; TRACK_COUNT],
}

impl Default for ArrangementShellState {
    fn default() -> Self {
        let mut state = Self {
            running: true,
            frame: 0,
            playhead_beat: 0.0,
            selected_track: 1,
            selected_clip: Some(2),
            panels: PanelVisibility::default(),
            clips: default_clips(),
            mixer: std::array::from_fn(TrackMeter::new),
        };
        state.tick();
        state
    }
}

impl ArrangementShellState {
    pub(crate) fn tick(&mut self) {
        if !self.running {
            return;
        }
        self.frame = self.frame.saturating_add(1);
        self.playhead_beat = (self.playhead_beat + 0.045) % TOTAL_BEATS;
        for meter in &mut self.mixer {
            meter.tick(self.frame);
        }
    }

    pub(crate) fn reset(&mut self) {
        *self = Self::default();
    }

    pub(crate) fn status(&self) -> String {
        let selected = self
            .selected_clip()
            .map(selected_clip_status)
            .unwrap_or_else(|| format!("track {}", TRACKS[self.selected_track]));
        let transport = if self.running { "running" } else { "paused" };
        format!(
            "{transport} | frame {} | playhead {:.2} | {selected} | synthetic GUI data",
            self.frame, self.playhead_beat
        )
    }

    pub(crate) fn selected_clip(&self) -> Option<ArrangementClip> {
        self.selected_clip
            .and_then(|id| self.clips.iter().copied().find(|clip| clip.id == id))
    }

    pub(crate) fn apply_shell_message(&mut self, message: ShellMessage) {
        match message {
            ShellMessage::SelectTrack(track) => self.select_track(track),
            ShellMessage::SelectClip(id) => self.select_clip(id),
            ShellMessage::Seek { beat } => {
                self.playhead_beat = beat.clamp(0.0, TOTAL_BEATS);
            }
            ShellMessage::ToggleBrowser => self.panels.toggle_browser(),
            ShellMessage::ToggleInspector => self.panels.toggle_inspector(),
        }
    }

    fn select_track(&mut self, track: usize) {
        self.selected_track = track.min(TRACK_COUNT - 1);
        self.selected_clip = self
            .clips
            .iter()
            .find(|clip| clip.track == self.selected_track)
            .map(|clip| clip.id);
    }

    fn select_clip(&mut self, id: u32) {
        if let Some(clip) = self.clips.iter().find(|clip| clip.id == id) {
            self.selected_track = clip.track;
            self.selected_clip = Some(id);
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct PanelVisibility {
    pub(crate) browser_open: bool,
    pub(crate) inspector_open: bool,
}

impl Default for PanelVisibility {
    fn default() -> Self {
        Self {
            browser_open: true,
            inspector_open: true,
        }
    }
}

impl PanelVisibility {
    fn toggle_browser(&mut self) {
        self.browser_open = !self.browser_open;
    }

    fn toggle_inspector(&mut self) {
        self.inspector_open = !self.inspector_open;
    }
}

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

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct TrackMeter {
    pub(crate) track: usize,
    pub(crate) level: f32,
    pub(crate) peak: f32,
}

impl TrackMeter {
    fn new(track: usize) -> Self {
        Self {
            track,
            level: 0.0,
            peak: 0.0,
        }
    }

    fn tick(&mut self, frame: u64) {
        let phase = frame as f32 * (0.030 + self.track as f32 * 0.006);
        let pulse = (phase.sin() * 0.5 + 0.5).powf(1.8);
        let accent = if (frame + self.track as u64 * 9) % (42 + self.track as u64 * 4) < 5 {
            0.35
        } else {
            0.0
        };
        let target = (0.10 + pulse * 0.62 + accent).min(1.0);
        self.level = self.level * 0.70 + target * 0.30;
        self.peak = if target > self.peak {
            target
        } else {
            (self.peak - 0.012).max(self.level)
        };
    }
}

fn default_clips() -> Vec<ArrangementClip> {
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

fn selected_clip_status(clip: ArrangementClip) -> String {
    format!(
        "{} on {} beat {:.1}",
        clip.label, TRACKS[clip.track], clip.start_beat
    )
}
