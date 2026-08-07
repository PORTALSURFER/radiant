//! macOS-only live acceptance harness for the public frame-profile contract.

#[cfg(any(target_os = "macos", test))]
use radiant::prelude::*;
#[cfg(any(target_os = "macos", test))]
use radiant::runtime::{FrameProfile, FrameProfileGpuTimingStatus, ProfilingOptions};

#[cfg(target_os = "macos")]
use std::sync::Arc;

#[cfg(target_os = "macos")]
#[allow(clippy::arc_with_non_send_sync)]
fn main() -> radiant::Result {
    let config = ProfileConfig::from_args(std::env::args().skip(1))?;

    radiant::app(AcceptanceState::new(config))
        .title("Radiant macOS Frame Profile Acceptance")
        .size(760, 520)
        .min_size(560, 360)
        .profiling(config.main.profiling_options())
        .view(project_main_view)
        .animation(|_| true)
        .on_frame(|| AcceptanceMessage::FrameTick)
        .on_frame_profile(|state: &mut AcceptanceState, profile| {
            state.recorder.observe(profile);
        })
        .auxiliary_windows(|state: &mut AcceptanceState| {
            if !state.auxiliary_visible || !state.auxiliary_window_is_ready() {
                return Vec::new();
            }
            let mut window = AuxiliaryWindow::utility(
                "frame-profile-auxiliary",
                "Radiant Auxiliary Frame Profile",
                420.0,
                260.0,
                Arc::new(project_auxiliary_view(state).into_surface()),
            )
            .on_close(AcceptanceMessage::CloseAuxiliary);
            window.options.frame.profiling = state.config.aux.profiling_options();
            vec![window]
        })
        .update(|state, message| match message {
            AcceptanceMessage::RecordClick => {
                state.click_count = state.click_count.saturating_add(1);
            }
            AcceptanceMessage::RecordAuxiliaryClick => {
                state.auxiliary_click_count = state.auxiliary_click_count.saturating_add(1);
            }
            AcceptanceMessage::FrameTick => {
                state.frame_tick_count = state.frame_tick_count.saturating_add(1);
            }
            AcceptanceMessage::CloseAuxiliary => {
                state.auxiliary_visible = false;
            }
        })
        .run()
}

#[cfg(not(target_os = "macos"))]
fn main() -> radiant::Result {
    Err("macos_frame_profile_acceptance is macOS-only".to_owned())
}

#[cfg(any(target_os = "macos", test))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ProfileMode {
    Off,
    Frame,
}

#[cfg(any(target_os = "macos", test))]
impl ProfileMode {
    fn parse(value: &str) -> Option<Self> {
        match value {
            "off" => Some(Self::Off),
            "frame" => Some(Self::Frame),
            _ => None,
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::Frame => "frame",
        }
    }

    fn profiling_options(self) -> ProfilingOptions {
        match self {
            Self::Off => ProfilingOptions::off(),
            Self::Frame => ProfilingOptions::frame(),
        }
    }

    fn is_frame(self) -> bool {
        matches!(self, Self::Frame)
    }
}

#[cfg(any(target_os = "macos", test))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct ProfileConfig {
    main: ProfileMode,
    aux: ProfileMode,
}

#[cfg(any(target_os = "macos", test))]
impl ProfileConfig {
    fn from_args<I, S>(args: I) -> radiant::Result<Self>
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let mut config = Self {
            main: ProfileMode::Off,
            aux: ProfileMode::Off,
        };
        let mut main_seen = false;
        let mut aux_seen = false;

        for argument in args {
            let argument = argument.into();
            let (name, value) = argument
                .split_once('=')
                .ok_or_else(|| format!("unsupported argument `{argument}`"))?;
            let mode = ProfileMode::parse(value)
                .ok_or_else(|| format!("{name} must be `off` or `frame`, got `{value}`"))?;
            match name {
                "--main" if !main_seen => {
                    config.main = mode;
                    main_seen = true;
                }
                "--aux" if !aux_seen => {
                    config.aux = mode;
                    aux_seen = true;
                }
                "--main" => return Err("--main may only be supplied once".to_owned()),
                "--aux" => return Err("--aux may only be supplied once".to_owned()),
                _ => return Err(format!("unsupported argument `{argument}`")),
            }
        }

        Ok(config)
    }
}

#[cfg(any(target_os = "macos", test))]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct ProfileSlot {
    callbacks: u32,
    identity: Option<u64>,
    first_sequence: Option<u64>,
    last_sequence: Option<u64>,
    sequence_violations: u32,
    missing_sequences: u32,
    gpu_unavailable: u32,
}

#[cfg(any(target_os = "macos", test))]
impl ProfileSlot {
    fn observe(&mut self, profile: FrameProfile) {
        self.callbacks = self.callbacks.saturating_add(1);
        if self.identity.is_none() {
            self.identity = profile.window_identity;
        } else if self.identity != profile.window_identity {
            self.sequence_violations = self.sequence_violations.saturating_add(1);
        }

        match profile.frame_sequence {
            Some(sequence) => {
                if self.first_sequence.is_none() {
                    self.first_sequence = Some(sequence);
                }
                if self
                    .last_sequence
                    .is_some_and(|previous| sequence <= previous)
                {
                    self.sequence_violations = self.sequence_violations.saturating_add(1);
                }
                self.last_sequence = Some(sequence);
            }
            None => {
                self.missing_sequences = self.missing_sequences.saturating_add(1);
            }
        }

        if profile.gpu_timing == FrameProfileGpuTimingStatus::Unavailable {
            self.gpu_unavailable = self.gpu_unavailable.saturating_add(1);
        }
    }

    fn stable_identity(self) -> bool {
        self.callbacks > 0 && self.identity.is_some() && self.sequence_violations == 0
    }

    fn strictly_increasing_sequences(self) -> bool {
        self.callbacks > 1
            && self.missing_sequences == 0
            && self.sequence_violations == 0
            && self.first_sequence.is_some()
            && self.last_sequence.is_some()
    }
}

#[cfg(any(target_os = "macos", test))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ProfileSlotKind {
    Primary,
    Auxiliary,
    Unclassified,
}

#[cfg(any(target_os = "macos", test))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct ProfileRecorder {
    main_mode: ProfileMode,
    aux_mode: ProfileMode,
    primary: ProfileSlot,
    auxiliary: ProfileSlot,
    auxiliary_handoffs: u32,
    unclassified_callbacks: u32,
}

#[cfg(any(target_os = "macos", test))]
impl ProfileRecorder {
    fn new(config: ProfileConfig) -> Self {
        Self {
            main_mode: config.main,
            aux_mode: config.aux,
            primary: ProfileSlot::default(),
            auxiliary: ProfileSlot::default(),
            auxiliary_handoffs: 0,
            unclassified_callbacks: 0,
        }
    }

    fn observe(&mut self, profile: FrameProfile) {
        match self.slot_kind(profile.window_identity) {
            ProfileSlotKind::Primary => self.primary.observe(profile),
            ProfileSlotKind::Auxiliary => {
                self.auxiliary.observe(profile);
                self.auxiliary_handoffs = self.auxiliary_handoffs.saturating_add(1);
            }
            ProfileSlotKind::Unclassified => {
                self.unclassified_callbacks = self.unclassified_callbacks.saturating_add(1);
            }
        }
    }

    fn slot_kind(&self, identity: Option<u64>) -> ProfileSlotKind {
        if self.main_mode.is_frame()
            && (self.primary.callbacks == 0 || self.primary.identity == identity)
        {
            return ProfileSlotKind::Primary;
        }
        if self.aux_mode.is_frame()
            && (self.auxiliary.callbacks == 0 || self.auxiliary.identity == identity)
        {
            return ProfileSlotKind::Auxiliary;
        }
        ProfileSlotKind::Unclassified
    }

    fn distinct_identities(self) -> bool {
        self.primary.identity.is_some()
            && self.auxiliary.identity.is_some()
            && self.primary.identity != self.auxiliary.identity
    }
}

#[cfg(any(target_os = "macos", test))]
struct AcceptanceState {
    config: ProfileConfig,
    recorder: ProfileRecorder,
    click_count: u32,
    auxiliary_click_count: u32,
    frame_tick_count: u32,
    auxiliary_visible: bool,
}

#[cfg(any(target_os = "macos", test))]
impl AcceptanceState {
    fn new(config: ProfileConfig) -> Self {
        Self {
            config,
            recorder: ProfileRecorder::new(config),
            click_count: 0,
            auxiliary_click_count: 0,
            frame_tick_count: 0,
            auxiliary_visible: true,
        }
    }

    // Combined Frame/Frame must establish the primary identity before the
    // auxiliary window can publish a profile; otherwise callback arrival order
    // can classify the auxiliary identity as primary.
    fn auxiliary_window_is_ready(&self) -> bool {
        !matches!(
            (self.config.main, self.config.aux),
            (ProfileMode::Frame, ProfileMode::Frame)
        ) || self.recorder.primary.identity.is_some()
    }
}

#[cfg(target_os = "macos")]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum AcceptanceMessage {
    RecordClick,
    RecordAuxiliaryClick,
    FrameTick,
    CloseAuxiliary,
}

#[cfg(target_os = "macos")]
fn project_main_view(state: &AcceptanceState) -> View<AcceptanceMessage> {
    let recorder = state.recorder;
    column([
        text("macOS live frame-profile acceptance").primary(),
        text(format!(
            "Startup modes: main={} aux={} (restart to change; no runtime switching)",
            state.config.main.as_str(),
            state.config.aux.as_str()
        )),
        text("Click the primary and auxiliary controls as needed, close the auxiliary window to expose the primary, then resize the primary window. Inspect the recorder after each action.")
            .wrap(),
        button("Record click")
            .primary()
            .message(AcceptanceMessage::RecordClick),
        text(format!(
            "Frame ticks: {} | primary clicks: {} | auxiliary clicks: {} | auxiliary handoffs: {}",
            state.frame_tick_count,
            state.click_count,
            state.auxiliary_click_count,
            recorder.auxiliary_handoffs
        )),
        text(format!(
            "Primary successful-present profiles: {} | identity={:?} | stable identity={} | sequences={:?}->{:?} | strictly increasing={} | sequence violations={} | missing sequences={} | GPU unavailable={}",
            recorder.primary.callbacks,
            recorder.primary.identity,
            recorder.primary.stable_identity(),
            recorder.primary.first_sequence,
            recorder.primary.last_sequence,
            recorder.primary.strictly_increasing_sequences(),
            recorder.primary.sequence_violations,
            recorder.primary.missing_sequences,
            recorder.primary.gpu_unavailable
        ))
        .wrap(),
        text(format!(
            "Auxiliary successful-present profiles: {} | identity={:?} | stable identity={} | sequences={:?}->{:?} | strictly increasing={} | sequence violations={} | missing sequences={} | GPU unavailable={}",
            recorder.auxiliary.callbacks,
            recorder.auxiliary.identity,
            recorder.auxiliary.stable_identity(),
            recorder.auxiliary.first_sequence,
            recorder.auxiliary.last_sequence,
            recorder.auxiliary.strictly_increasing_sequences(),
            recorder.auxiliary.sequence_violations,
            recorder.auxiliary.missing_sequences,
            recorder.auxiliary.gpu_unavailable
        ))
        .wrap(),
        text(format!(
            "Auxiliary identity distinct from primary: {} | unclassified callbacks: {}",
            recorder.distinct_identities(),
            recorder.unclassified_callbacks
        )),
        text("Expected: Off produces zero callbacks; Frame produces at least two stable, increasing profiles with gpu=Unavailable.")
            .wrap(),
    ])
    .padding(20.0)
    .spacing(10.0)
}

#[cfg(target_os = "macos")]
fn project_auxiliary_view(state: &AcceptanceState) -> View<AcceptanceMessage> {
    column([
        text("Auxiliary FrameProfile handoff").primary(),
        text(format!(
            "Configured mode: {} (startup-selected)",
            state.config.aux.as_str()
        )),
        button("Record auxiliary click")
            .primary()
            .message(AcceptanceMessage::RecordAuxiliaryClick),
        text("This window has its own native identity. Click this control as needed, then resize the primary window to create fresh evidence.")
            .wrap(),
    ])
    .padding(16.0)
    .spacing(10.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn profile(identity: u64, sequence: u64) -> FrameProfile {
        FrameProfile {
            window_identity: Some(identity),
            frame_sequence: Some(sequence),
            gpu_timing: FrameProfileGpuTimingStatus::Unavailable,
            ..FrameProfile::default()
        }
    }

    #[test]
    fn profile_config_defaults_to_off_and_parses_startup_modes() {
        assert_eq!(
            ProfileConfig::from_args(std::iter::empty::<String>()).unwrap(),
            ProfileConfig {
                main: ProfileMode::Off,
                aux: ProfileMode::Off,
            }
        );
        assert_eq!(
            ProfileConfig::from_args(["--main=frame", "--aux=off"]).unwrap(),
            ProfileConfig {
                main: ProfileMode::Frame,
                aux: ProfileMode::Off,
            }
        );
        assert!(ProfileConfig::from_args(["--main=bogus"]).is_err());
        assert!(ProfileConfig::from_args(["--main=frame", "--main=off"]).is_err());
        assert!(ProfileConfig::from_args(["--switch=frame"]).is_err());
    }

    #[test]
    fn off_recorder_does_not_invent_profile_callbacks() {
        let config = ProfileConfig::from_args(std::iter::empty::<String>()).unwrap();
        let recorder = ProfileRecorder::new(config);

        assert_eq!(recorder.primary.callbacks, 0);
        assert_eq!(recorder.auxiliary.callbacks, 0);
        assert_eq!(recorder.auxiliary_handoffs, 0);
    }

    #[test]
    fn frame_recorder_keeps_bounded_monotonic_identity_and_gpu_evidence() {
        let config = ProfileConfig {
            main: ProfileMode::Frame,
            aux: ProfileMode::Off,
        };
        let mut recorder = ProfileRecorder::new(config);
        recorder.observe(profile(7, 10));
        recorder.observe(profile(7, 11));

        assert_eq!(recorder.primary.callbacks, 2);
        assert!(recorder.primary.stable_identity());
        assert!(recorder.primary.strictly_increasing_sequences());
        assert_eq!(recorder.primary.gpu_unavailable, 2);
        assert_eq!(recorder.unclassified_callbacks, 0);

        recorder.observe(profile(7, 11));
        assert_eq!(recorder.primary.sequence_violations, 1);
        assert!(!recorder.primary.strictly_increasing_sequences());
    }

    #[test]
    fn auxiliary_frame_profiles_are_handed_off_with_distinct_identity() {
        let config = ProfileConfig {
            main: ProfileMode::Frame,
            aux: ProfileMode::Frame,
        };
        let mut recorder = ProfileRecorder::new(config);
        recorder.observe(profile(7, 10));
        recorder.observe(profile(7, 11));
        recorder.observe(profile(9, 3));
        recorder.observe(profile(9, 4));

        assert!(recorder.primary.strictly_increasing_sequences());
        assert!(recorder.auxiliary.strictly_increasing_sequences());
        assert!(recorder.distinct_identities());
        assert_eq!(recorder.auxiliary_handoffs, 2);
        assert_eq!(recorder.unclassified_callbacks, 0);
    }

    #[test]
    fn auxiliary_frame_is_recorded_when_primary_is_off() {
        let config = ProfileConfig {
            main: ProfileMode::Off,
            aux: ProfileMode::Frame,
        };
        let mut recorder = ProfileRecorder::new(config);
        recorder.observe(profile(9, 1));
        recorder.observe(profile(9, 2));

        assert_eq!(recorder.primary.callbacks, 0);
        assert!(recorder.auxiliary.strictly_increasing_sequences());
        assert_eq!(recorder.auxiliary_handoffs, 2);
    }

    #[test]
    fn combined_frame_auxiliary_projection_waits_for_primary_identity() {
        let config = ProfileConfig {
            main: ProfileMode::Frame,
            aux: ProfileMode::Frame,
        };
        let mut state = AcceptanceState::new(config);

        // The admission closure must withhold this window until primary
        // identity is observed, so an auxiliary callback cannot win the slot.
        assert!(!state.auxiliary_window_is_ready());

        state.recorder.observe(profile(7, 10));

        assert_eq!(state.recorder.primary.identity, Some(7));
        assert!(state.auxiliary_window_is_ready());

        for config in [
            ProfileConfig {
                main: ProfileMode::Off,
                aux: ProfileMode::Off,
            },
            ProfileConfig {
                main: ProfileMode::Frame,
                aux: ProfileMode::Off,
            },
            ProfileConfig {
                main: ProfileMode::Off,
                aux: ProfileMode::Frame,
            },
        ] {
            assert!(AcceptanceState::new(config).auxiliary_window_is_ready());
        }
    }
}
