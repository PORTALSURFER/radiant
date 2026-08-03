use super::{
    NativeFrameTimingDiagnostics, NativeGpuSurfaceDiagnostics, NativeRetainedSurfaceDiagnostics,
    NativeSceneDiagnostics, NativeTextDiagnostics,
};

/// Opaque identity for one native window runner within one native runtime run.
///
/// The value is allocated by the native runtime and can only be inspected
/// through [`Self::get`]. Pair it with a frame sequence when correlating
/// diagnostics across primary and auxiliary windows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NativeWindowDiagnosticIdentity(u64);

impl NativeWindowDiagnosticIdentity {
    /// Return the numeric identity for host diagnostics or export.
    pub const fn get(self) -> u64 {
        self.0
    }

    pub(crate) const fn from_runtime_value(value: u64) -> Self {
        Self(value)
    }
}

/// Cumulative, bounded observations of native surface recovery for one window.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct NativeSurfaceRecoveryDiagnostics {
    /// Number of surface-acquisition failures reported as lost.
    pub lost: u64,
    /// Number of surface-acquisition failures reported as outdated.
    pub outdated: u64,
    /// Number of surface-acquisition failures reported as timeouts.
    pub timeouts: u64,
    /// Number of surface-acquisition failures reported as other errors.
    pub others: u64,
    /// Number of forced surface reconfigurations that completed.
    pub completed_reconfigures: u64,
    /// Number of lost or outdated acquisitions deferred while the window had
    /// a zero width or height.
    pub zero_size_deferrals: u64,
    /// Number of redraw retries requested after a completed reconfiguration.
    pub retry_requests: u64,
    /// Number of one-shot redraw retries requested after a timeout.
    pub timeout_retry_requests: u64,
    /// Number of one-shot redraw retries requested after an other error.
    pub other_retry_requests: u64,
}

/// The latest bounded CPU scheduler-turn disposition for one native window.
///
/// `Unknown` is the conservative value when no fairness ledger state is
/// available for the window.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum NativeCpuFrameFairnessDisposition {
    /// No bounded scheduler-turn state is available for this window.
    #[default]
    Unknown,
    /// The window was present in the turn but had no due work.
    NotDue,
    /// The window was selected by the existing scheduler cursor.
    Selected,
    /// The window had due work but another key was selected.
    DueButDeferred,
}

/// Bounded, observational CPU fairness evidence for one native window.
///
/// This summary describes the existing scheduler's recent turn observations;
/// it does not change selection, admission, quotas, deadlines, or rendering.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct NativeCpuFrameFairnessDiagnostics {
    /// Whether a bounded scheduler-turn state exists for this window.
    pub available: bool,
    /// Disposition recorded by the latest observed scheduler turn.
    pub latest_disposition: NativeCpuFrameFairnessDisposition,
    /// Native target FPS requested before runtime activity caps.
    pub requested_target_fps: u32,
    /// Effective target FPS used by the existing cadence policy.
    pub effective_target_fps: u32,
    /// Saturating microseconds by which the latest scheduler turn was already
    /// past its original cadence `due_at` boundary. This is observational
    /// missed-deadline evidence and does not change scheduling policy.
    pub latest_due_lateness_us: Option<u64>,
    /// Saturating number of observed turns with no due work.
    pub not_due_turns: u64,
    /// Saturating number of observed turns where this key was selected.
    pub selected_turns: u64,
    /// Saturating number of observed turns where due work was deferred.
    pub due_but_deferred_turns: u64,
    /// Saturating number of exact cursor admissions for this key.
    pub cursor_admissions: u64,
    /// Whether the latest selected turn reached the cursor-admission boundary.
    pub latest_selected_was_admitted: bool,
}

/// Completion vocabulary for one latest bounded CPU frame observation.
///
/// `Unknown` is the conservative value when no completed observation is
/// available for the window.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum NativeCpuFrameCompletionOutcome {
    /// No completed observation is available for this window.
    #[default]
    Unknown,
    /// The observed redraw reached a successful presentation.
    SuccessfulPresentation,
    /// The observed redraw was admitted but did not start or complete frame
    /// work.
    SkippedOrVetoed,
    /// The observed redraw started frame work but did not present it.
    Incomplete,
    /// The observed redraw failed without entering the recovery outcome.
    Failed,
    /// The observed redraw triggered native recovery.
    RecoveryTriggered,
}

/// Bounded, observational CPU frame evidence for one native window.
///
/// This value projects the existing parent-owned CPU frame observation ledger;
/// it does not select work, change admission, route input, render, or affect
/// publication ordering. The default/zero state means that bounded evidence is
/// unavailable, including when frame diagnostics are disabled or the window's
/// observation key is not retained.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct NativeCpuFrameObservationDiagnostics {
    /// Whether a completed bounded observation exists for this window.
    pub available: bool,
    /// Completion outcome recorded by the latest bounded observation.
    pub latest_outcome: NativeCpuFrameCompletionOutcome,
    /// Whether the latest observed frame carried exact routed interaction
    /// evidence.
    pub latest_exact_interaction: bool,
    /// Saturating number of redraws admitted to the bounded ledger.
    pub admitted_redraws: u64,
    /// Saturating number of redraws that reached successful presentation.
    pub successful_presentations: u64,
    /// Saturating number of admitted redraws that were skipped or vetoed.
    pub skipped_or_vetoed_redraws: u64,
    /// Saturating number of redraws that started but did not complete a frame.
    pub incomplete_frames: u64,
    /// Saturating number of redraws that failed, including recovery-triggered
    /// redraws as recorded by the existing ledger.
    pub failed_frames: u64,
    /// Saturating number of redraws that triggered native recovery.
    pub recovery_triggered_frames: u64,
}

/// Structured diagnostics for one native presentation frame.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct NativeFrameDiagnostics {
    /// Opaque identity of the native window runner that presented this frame.
    /// Pair it with [`Self::frame_sequence`] to correlate frames across
    /// primary and auxiliary windows.
    pub window_identity: Option<NativeWindowDiagnosticIdentity>,
    /// Monotonic sequence for this native window's successfully presented
    /// frames. It starts at one and remains scoped to the window across
    /// recovery; `None` means no presentation has occurred yet or the `u64`
    /// counter is exhausted without wrapping or reusing a value.
    pub frame_sequence: Option<u64>,
    /// Opt-in, saturating microseconds from Radiant's native event-loop
    /// arrival of the latest tracked interactive event to this successful
    /// presentation. This begins at event-loop arrival and does not include
    /// platform queue time before that boundary. The value is `None` when no
    /// tracked interactive event preceded this presentation or diagnostics
    /// were disabled.
    pub input_to_present_latency_us: Option<u64>,
    /// Opt-in, bounded CPU scheduler-turn fairness observations for this
    /// window. The default is unavailable/no state.
    pub cpu_fairness: NativeCpuFrameFairnessDiagnostics,
    /// Opt-in, bounded CPU frame admission/completion observations for this
    /// window. The default is unavailable/no state.
    pub cpu_observation: NativeCpuFrameObservationDiagnostics,
    /// Redraw routing metadata for the presented native frame.
    pub presentation: NativeFramePresentationDiagnostics,
    /// Cumulative native surface recovery observations for the window.
    pub surface_recovery: NativeSurfaceRecoveryDiagnostics,
    /// Scene and retained-surface encoding counters.
    pub scene: NativeSceneDiagnostics,
    /// Native text layout cache activity.
    pub text: NativeTextDiagnostics,
    /// Retained custom-surface cache state and activity.
    pub retained_surfaces: NativeRetainedSurfaceDiagnostics,
    /// GPU-surface cache and render activity.
    pub gpu_surfaces: NativeGpuSurfaceDiagnostics,
    /// Coarse timing buckets for presentation work.
    pub timings: NativeFrameTimingDiagnostics,
}

/// Native redraw routing metadata for one presented frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NativeFramePresentationDiagnostics {
    /// Coarse frame-work kind selected by native event/runtime routing.
    pub frame_work_kind: &'static str,
    /// Stable reason label for the frame-work request.
    pub frame_work_reason: &'static str,
    /// Typed surface invalidation stage selected by the runtime.
    pub surface_invalidation: &'static str,
    /// Whether the frame-work path stayed on paint-only redraw.
    pub paint_only: bool,
    /// Whether the frame-work path required a scene rebuild.
    pub scene_rebuild: bool,
}

impl Default for NativeFramePresentationDiagnostics {
    fn default() -> Self {
        Self {
            frame_work_kind: "none",
            frame_work_reason: "none",
            surface_invalidation: "none",
            paint_only: false,
            scene_rebuild: false,
        }
    }
}
