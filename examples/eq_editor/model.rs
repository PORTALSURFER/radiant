pub(super) const MIN_FREQ_HZ: f32 = 20.0;
pub(super) const MAX_FREQ_HZ: f32 = 20_000.0;
pub(super) const MIN_GAIN_DB: f32 = -24.0;
pub(super) const MAX_GAIN_DB: f32 = 24.0;

#[derive(Clone, Debug)]
pub(super) struct EqEditorState {
    pub(super) bypassed: bool,
    pub(super) analyzer: bool,
    pub(super) selected_band: u32,
    pub(super) bands: Vec<EqBand>,
    pub(super) status: String,
}

impl Default for EqEditorState {
    fn default() -> Self {
        Self {
            bypassed: false,
            analyzer: true,
            selected_band: 2,
            bands: vec![
                EqBand::new(1, "HP", 80.0, 0.0, 0.70, EqBandKind::HighPass),
                EqBand::new(2, "Bell", 420.0, 4.5, 1.10, EqBandKind::Bell),
                EqBand::new(3, "Bell", 2_400.0, -5.0, 1.35, EqBandKind::Bell),
                EqBand::new(4, "Shelf", 10_000.0, 3.0, 0.85, EqBandKind::HighShelf),
            ],
            status: "ready".into(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct EqBand {
    pub(super) id: u32,
    pub(super) label: &'static str,
    pub(super) freq_hz: f32,
    pub(super) gain_db: f32,
    pub(super) q: f32,
    pub(super) kind: EqBandKind,
    pub(super) enabled: bool,
}

impl EqBand {
    fn new(
        id: u32,
        label: &'static str,
        freq_hz: f32,
        gain_db: f32,
        q: f32,
        kind: EqBandKind,
    ) -> Self {
        Self {
            id,
            label,
            freq_hz,
            gain_db,
            q,
            kind,
            enabled: true,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum EqBandKind {
    Bell,
    HighPass,
    HighShelf,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum EqMessage {
    Editor(EqEditorMessage),
    ToggleBypass,
    ToggleAnalyzer,
    ToggleSelectedBand,
    NudgeGain(f32),
    NudgeQ(f32),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum EqEditorMessage {
    SelectBand(u32),
    MoveBand { id: u32, freq_hz: f32, gain_db: f32 },
}

pub(super) fn update(state: &mut EqEditorState, message: EqMessage) {
    match message {
        EqMessage::Editor(EqEditorMessage::SelectBand(id)) => {
            state.selected_band = id;
            state.status = format!("selected band {id}");
        }
        EqMessage::Editor(EqEditorMessage::MoveBand {
            id,
            freq_hz,
            gain_db,
        }) => move_band(state, id, freq_hz, gain_db),
        EqMessage::ToggleBypass => toggle_bypass(state),
        EqMessage::ToggleAnalyzer => toggle_analyzer(state),
        EqMessage::ToggleSelectedBand => toggle_selected_band(state),
        EqMessage::NudgeGain(delta) => nudge_selected_gain(state, delta),
        EqMessage::NudgeQ(delta) => nudge_selected_q(state, delta),
    }
}

pub(super) fn selected_band(state: &EqEditorState) -> Option<&EqBand> {
    state
        .bands
        .iter()
        .find(|band| band.id == state.selected_band)
}

fn selected_band_mut(state: &mut EqEditorState) -> Option<&mut EqBand> {
    state
        .bands
        .iter_mut()
        .find(|band| band.id == state.selected_band)
}

fn move_band(state: &mut EqEditorState, id: u32, freq_hz: f32, gain_db: f32) {
    state.selected_band = id;
    if let Some(band) = state.bands.iter_mut().find(|band| band.id == id) {
        band.freq_hz = freq_hz.clamp(MIN_FREQ_HZ, MAX_FREQ_HZ);
        band.gain_db = gain_db.clamp(MIN_GAIN_DB, MAX_GAIN_DB);
        state.status = format!(
            "band {id} moved to {:.0} Hz / {:+.1} dB",
            band.freq_hz, band.gain_db
        );
    }
}

fn toggle_bypass(state: &mut EqEditorState) {
    state.bypassed = !state.bypassed;
    state.status = if state.bypassed {
        "bypassed".into()
    } else {
        "active".into()
    };
}

fn toggle_analyzer(state: &mut EqEditorState) {
    state.analyzer = !state.analyzer;
    state.status = if state.analyzer {
        "analyzer overlay visible".into()
    } else {
        "analyzer overlay hidden".into()
    };
}

fn toggle_selected_band(state: &mut EqEditorState) {
    if let Some(band) = selected_band_mut(state) {
        band.enabled = !band.enabled;
        state.status = format!(
            "band {} {}",
            band.id,
            if band.enabled { "enabled" } else { "disabled" }
        );
    }
}

fn nudge_selected_gain(state: &mut EqEditorState, delta: f32) {
    if let Some(band) = selected_band_mut(state) {
        band.gain_db = (band.gain_db + delta).clamp(MIN_GAIN_DB, MAX_GAIN_DB);
        state.status = format!("band {} gain {:+.1} dB", band.id, band.gain_db);
    }
}

fn nudge_selected_q(state: &mut EqEditorState, delta: f32) {
    if let Some(band) = selected_band_mut(state) {
        band.q = (band.q + delta).clamp(0.20, 8.0);
        state.status = format!("band {} Q {:.2}", band.id, band.q);
    }
}
