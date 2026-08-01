//! Focused state groups owned by the generic native Vello runner.

use super::NativeAdapterGeneration;
use super::PendingGpuSurfaceWheel;
use super::PendingScrollbarDrag;
use super::input::NativePointerGestureLatch;
use super::window_environment::{AccessibilityDisplaySnapshot, MonitorFingerprint};
use super::{FrameWork, FrameWorkReason};
use crate::gui::types::Point;
use crate::gui::types::Vector2;
use crate::gui_runtime::native_vello::startup::StartupTimingProfile;
use crate::widgets::WidgetCursor;
use std::{
    collections::VecDeque,
    sync::Arc,
    time::{Duration, Instant},
};
use vello::{Renderer, util::RenderSurface, wgpu};
use winit::{
    dpi::PhysicalSize,
    keyboard::ModifiersState,
    window::{Window, WindowId},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum SurfaceAcquirePolicy {
    ReconfigureAndRetry,
    Defer,
    Timeout,
    Terminal,
    ConservativeFence,
}

pub(super) const fn surface_acquire_policy(
    error: wgpu::SurfaceError,
    size: PhysicalSize<u32>,
) -> SurfaceAcquirePolicy {
    match error {
        wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated
            if size.width > 0 && size.height > 0 =>
        {
            SurfaceAcquirePolicy::ReconfigureAndRetry
        }
        wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated => SurfaceAcquirePolicy::Defer,
        wgpu::SurfaceError::Timeout => SurfaceAcquirePolicy::Timeout,
        wgpu::SurfaceError::OutOfMemory => SurfaceAcquirePolicy::Terminal,
        wgpu::SurfaceError::Other => SurfaceAcquirePolicy::ConservativeFence,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct NativeSurfaceRecoveryState {
    lost: u64,
    outdated: u64,
    timeouts: u64,
    others: u64,
    completed_reconfigures: u64,
    zero_size_deferrals: u64,
    retry_requests: u64,
    timeout_retry_requests: u64,
    other_retry_requests: u64,
    transient_retry_armed: bool,
}

impl Default for NativeSurfaceRecoveryState {
    fn default() -> Self {
        Self {
            lost: 0,
            outdated: 0,
            timeouts: 0,
            others: 0,
            completed_reconfigures: 0,
            zero_size_deferrals: 0,
            retry_requests: 0,
            timeout_retry_requests: 0,
            other_retry_requests: 0,
            transient_retry_armed: true,
        }
    }
}

impl NativeSurfaceRecoveryState {
    pub(super) fn observe_acquire_error(&mut self, error: &wgpu::SurfaceError) {
        match error {
            wgpu::SurfaceError::Lost => {
                self.lost = self.lost.saturating_add(1);
            }
            wgpu::SurfaceError::Outdated => {
                self.outdated = self.outdated.saturating_add(1);
            }
            wgpu::SurfaceError::Timeout => {
                self.timeouts = self.timeouts.saturating_add(1);
            }
            wgpu::SurfaceError::Other => {
                self.others = self.others.saturating_add(1);
            }
            _ => {}
        }
    }

    pub(super) fn record_completed_reconfigure(&mut self) {
        self.completed_reconfigures = self.completed_reconfigures.saturating_add(1);
    }

    pub(super) fn record_zero_size_deferral(&mut self) {
        self.zero_size_deferrals = self.zero_size_deferrals.saturating_add(1);
    }

    pub(super) fn record_retry_request(&mut self) {
        self.retry_requests = self.retry_requests.saturating_add(1);
    }

    pub(super) fn record_timeout_retry_request(&mut self, retry_allowed: bool) -> bool {
        if !self.consume_transient_retry_permit(retry_allowed) {
            return false;
        }
        self.timeout_retry_requests = self.timeout_retry_requests.saturating_add(1);
        true
    }

    pub(super) fn record_other_retry_request(&mut self, retry_allowed: bool) -> bool {
        if !self.consume_transient_retry_permit(retry_allowed) {
            return false;
        }
        self.other_retry_requests = self.other_retry_requests.saturating_add(1);
        true
    }

    fn consume_transient_retry_permit(&mut self, retry_allowed: bool) -> bool {
        let should_retry = retry_allowed && self.transient_retry_armed;
        self.transient_retry_armed = false;
        should_retry
    }

    pub(super) fn rearm_transient_retry(&mut self) {
        self.transient_retry_armed = true;
    }

    pub(super) const fn diagnostics(self) -> crate::runtime::NativeSurfaceRecoveryDiagnostics {
        crate::runtime::NativeSurfaceRecoveryDiagnostics {
            lost: self.lost,
            outdated: self.outdated,
            timeouts: self.timeouts,
            others: self.others,
            completed_reconfigures: self.completed_reconfigures,
            zero_size_deferrals: self.zero_size_deferrals,
            retry_requests: self.retry_requests,
            timeout_retry_requests: self.timeout_retry_requests,
            other_retry_requests: self.other_retry_requests,
        }
    }
}

impl From<NativeSurfaceRecoveryState> for crate::runtime::NativeSurfaceRecoveryDiagnostics {
    fn from(state: NativeSurfaceRecoveryState) -> Self {
        state.diagnostics()
    }
}

/// Monotonic evidence for the currently configured native presentation target.
///
/// This is deliberately opaque to fingerprint consumers: an unknown or
/// exhausted target never produces reusable evidence, and the serial is never
/// wrapped back to an earlier value.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::gui_runtime::native_vello) struct NativeTargetGeneration {
    serial: u64,
    status: NativeTargetGenerationStatus,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum NativeTargetGenerationStatus {
    Unknown,
    Known,
    Exhausted,
}

impl NativeTargetGeneration {
    pub(super) const fn unknown() -> Self {
        Self {
            serial: 0,
            status: NativeTargetGenerationStatus::Unknown,
        }
    }

    /// Advance after an authoritative target transition. Returns `false` once
    /// the serial is exhausted; callers must then remain conservative.
    pub(super) fn advance(&mut self) -> bool {
        if matches!(self.status, NativeTargetGenerationStatus::Exhausted) {
            return false;
        }
        let Some(serial) = self.serial.checked_add(1) else {
            self.status = NativeTargetGenerationStatus::Exhausted;
            return false;
        };
        self.serial = serial;
        self.status = NativeTargetGenerationStatus::Known;
        true
    }

    /// Fence an uncertain recovery without claiming that the old target is
    /// still valid. The next configured target advances monotonically.
    pub(super) fn invalidate_unknown(&mut self) {
        if !matches!(self.status, NativeTargetGenerationStatus::Exhausted) {
            self.status = NativeTargetGenerationStatus::Unknown;
        }
    }

    pub(super) const fn is_known(self) -> bool {
        matches!(self.status, NativeTargetGenerationStatus::Known)
    }

    #[cfg(test)]
    pub(super) const fn from_test_serial(serial: u64) -> Self {
        Self {
            serial,
            status: NativeTargetGenerationStatus::Known,
        }
    }
}

impl Default for NativeTargetGeneration {
    fn default() -> Self {
        Self::unknown()
    }
}

/// The complete native surface/renderer binding for one window.
///
/// The bundle is published only after both WGPU surface setup and Vello
/// renderer construction succeed. Its generation is owner-provided evidence,
/// never a device or handle identity substitute.
pub(super) struct NativeWindowResourceBundle {
    pub(super) generation: NativeAdapterGeneration,
    pub(super) render_surface: RenderSurface<'static>,
    pub(super) renderer: Renderer,
}

impl NativeWindowResourceBundle {
    pub(super) fn new(
        generation: NativeAdapterGeneration,
        render_surface: RenderSurface<'static>,
        renderer: Renderer,
    ) -> Option<Self> {
        generation.is_known().then_some(Self {
            generation,
            render_surface,
            renderer,
        })
    }
}

const MAX_QUARANTINED_NATIVE_RESOURCES: usize = 2;

/// Bounded ownership for native resources that have left the admitted path.
///
/// This is intentionally only a quarantine boundary. A later retirement
/// contract may drain entries after a renderer-owned completion witness; this
/// type never authorizes synchronous destruction or GPU work.
pub(super) struct NativeResourceQuarantine<T> {
    entries: VecDeque<T>,
}

impl<T> Default for NativeResourceQuarantine<T> {
    fn default() -> Self {
        Self {
            entries: VecDeque::new(),
        }
    }
}

impl<T> NativeResourceQuarantine<T> {
    #[cfg(test)]
    pub(super) fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub(super) fn is_full(&self) -> bool {
        self.entries.len() >= MAX_QUARANTINED_NATIVE_RESOURCES
    }

    pub(super) fn try_push(&mut self, entry: T) -> Result<(), T> {
        if self.is_full() {
            Err(entry)
        } else {
            self.entries.push_back(entry);
            Ok(())
        }
    }

    #[cfg(test)]
    fn len(&self) -> usize {
        self.entries.len()
    }
}

pub(super) struct NativeResourcePublicationReservation<'a, T> {
    active: &'a mut Option<T>,
    quarantine: &'a mut NativeResourceQuarantine<T>,
}

fn reserve_native_resource_publication<'a, T>(
    active: &'a mut Option<T>,
    quarantine: &'a mut NativeResourceQuarantine<T>,
) -> Option<NativeResourcePublicationReservation<'a, T>> {
    if active.is_some() && quarantine.is_full() {
        return None;
    }
    Some(NativeResourcePublicationReservation { active, quarantine })
}

impl<T> NativeResourcePublicationReservation<'_, T> {
    pub(super) fn publish(self, incoming: T) {
        let Self { active, quarantine } = self;
        if let Some(previous) = active.take() {
            // The reservation exclusively owns both fields until this commit,
            // so no other path can fill the bounded quarantine.
            quarantine.entries.push_back(previous);
        }
        *active = Some(incoming);
    }
}

#[derive(Default)]
pub(super) struct NativeRunnerWindowState {
    pub(super) id: Option<WindowId>,
    pub(super) window: Option<Arc<Window>>,
    pub(super) native_resources: Option<NativeWindowResourceBundle>,
    pub(super) quarantined_native_resources: NativeResourceQuarantine<NativeWindowResourceBundle>,
    pub(super) native_dpi_scale: crate::theme::DpiScale,
    pub(super) dpi_scale: crate::theme::DpiScale,
    pub(super) dpi_scale_override: Option<crate::theme::DpiScale>,
    pub(super) native_focus_lost: bool,
    pub(super) monitor_fingerprint: Option<MonitorFingerprint>,
    pub(super) accessibility_display: AccessibilityDisplaySnapshot,
    pub(super) environment: crate::runtime::WindowEnvironment,
    pub(super) target_generation: NativeTargetGeneration,
    pub(super) native_surface_target_fenced: bool,
    pub(super) surface_recovery: NativeSurfaceRecoveryState,
}

impl NativeRunnerWindowState {
    pub(super) fn can_publish_native_resources(&self) -> bool {
        self.native_resources.is_none() || !self.quarantined_native_resources.is_full()
    }

    pub(super) fn reserve_native_resource_publication(
        &mut self,
    ) -> Option<NativeResourcePublicationReservation<'_, NativeWindowResourceBundle>> {
        reserve_native_resource_publication(
            &mut self.native_resources,
            &mut self.quarantined_native_resources,
        )
    }

    /// Isolate the active bundle after an admission veto without destroying
    /// its WGPU/Vello resources synchronously.
    pub(super) fn isolate_native_resources(&mut self) -> bool {
        if let Some(stale) = self.native_resources.take()
            && let Err(stale) = self.quarantined_native_resources.try_push(stale)
        {
            self.native_resources = Some(stale);
            return false;
        }
        true
    }
}

pub(super) struct NativeRunnerInputState {
    pub(super) last_cursor: Option<Point>,
    pub(super) native_cursor: Option<WidgetCursor>,
    pub(super) native_cursor_visible: bool,
    #[cfg(test)]
    pub(super) native_cursor_apply_count: usize,
    pub(super) clipboard: Option<arboard::Clipboard>,
    pub(super) modifiers: ModifiersState,
    pub(super) effective_pointer_gesture: Option<NativePointerGestureLatch>,
    pub(super) last_navigation_key_repeat: Option<Instant>,
    pub(super) pending_gpu_surface_wheel: Option<PendingGpuSurfaceWheel>,
    pub(super) pending_scroll_container_wheel: Option<PendingGpuSurfaceWheel>,
    pub(super) pending_scrollbar_drag: Option<PendingScrollbarDrag>,
}

impl Default for NativeRunnerInputState {
    fn default() -> Self {
        Self {
            last_cursor: None,
            native_cursor: None,
            native_cursor_visible: true,
            #[cfg(test)]
            native_cursor_apply_count: 0,
            clipboard: arboard::Clipboard::new().ok(),
            modifiers: ModifiersState::default(),
            effective_pointer_gesture: None,
            last_navigation_key_repeat: None,
            pending_gpu_surface_wheel: None,
            pending_scroll_container_wheel: None,
            pending_scrollbar_drag: None,
        }
    }
}

pub(super) struct NativeRunnerTimingState {
    pub(super) redraw_requested: bool,
    pub(super) redraw_requested_at: Option<Instant>,
    pub(super) startup_timing: StartupTimingProfile,
    pub(super) first_frame_presented: bool,
    pub(super) animation_origin: Instant,
    pub(super) last_redraw: Instant,
    pub(super) last_timed_frame_drain: Instant,
    pub(super) deferred_surface_refresh: bool,
    pub(super) deferred_surface_refresh_scope: Option<crate::runtime::RepaintScope>,
    pub(super) deferred_scene_rebuild: bool,
    pub(super) deferred_scene_rebuild_requires_encode: bool,
    pub(super) deferred_auxiliary_window_sync: bool,
    pub(super) last_interactive_scene_rebuild: Instant,
    pub(super) pending_surface_resize: Option<PhysicalSize<u32>>,
    pub(super) pending_surface_resize_reason: Option<FrameWorkReason>,
    pub(super) pending_viewport_resize: Option<Vector2>,
    pub(super) pending_viewport_resize_reason: Option<FrameWorkReason>,
    pub(super) surface_resize_applied_this_frame: bool,
    pub(super) pending_frame_work: FrameWork,
}

impl Default for NativeRunnerTimingState {
    fn default() -> Self {
        let now = Instant::now();
        Self {
            redraw_requested: false,
            redraw_requested_at: None,
            startup_timing: StartupTimingProfile::new(),
            first_frame_presented: false,
            animation_origin: now,
            last_redraw: now,
            last_timed_frame_drain: now,
            deferred_surface_refresh: false,
            deferred_surface_refresh_scope: None,
            deferred_scene_rebuild: false,
            deferred_scene_rebuild_requires_encode: false,
            deferred_auxiliary_window_sync: false,
            last_interactive_scene_rebuild: now - Duration::from_secs(1),
            pending_surface_resize: None,
            pending_surface_resize_reason: None,
            pending_viewport_resize: None,
            pending_viewport_resize_reason: None,
            surface_resize_applied_this_frame: false,
            pending_frame_work: FrameWork::None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        NativeResourceQuarantine, NativeRunnerWindowState, NativeSurfaceRecoveryState,
        NativeTargetGeneration,
    };
    use crate::runtime::NativeSurfaceRecoveryDiagnostics;

    #[test]
    fn native_window_resources_start_unpublished_and_without_stale_gpu_state() {
        let mut state = NativeRunnerWindowState::default();

        assert!(state.native_resources.is_none());
        assert!(state.quarantined_native_resources.is_empty());
        assert!(!state.native_surface_target_fenced);

        assert!(state.isolate_native_resources());
        assert!(state.native_resources.is_none());
        assert!(state.quarantined_native_resources.is_empty());
    }

    #[test]
    fn native_resource_quarantine_refuses_capacity_overflow_without_dropping_input() {
        let mut quarantine = NativeResourceQuarantine::default();

        assert!(quarantine.try_push(1).is_ok());
        assert!(quarantine.try_push(2).is_ok());
        assert!(quarantine.is_full());
        assert_eq!(quarantine.try_push(3), Err(3));
        assert_eq!(quarantine.len(), 2);
    }

    #[test]
    fn full_native_resource_publication_preserves_rejected_input_and_active_state() {
        let mut active = Some(1);
        let mut quarantine = NativeResourceQuarantine::default();
        assert!(quarantine.try_push(2).is_ok());
        assert!(quarantine.try_push(3).is_ok());
        let mut incoming = Some(4);

        // The generic reservation is acquired before a native bundle is built;
        // a full quarantine therefore leaves the caller's input untouched.
        assert!(super::reserve_native_resource_publication(&mut active, &mut quarantine).is_none());
        assert_eq!(incoming.take(), Some(4));
        assert_eq!(active, Some(1));
        assert_eq!(quarantine.len(), 2);
    }

    #[test]
    fn target_generation_fences_initial_resize_dpi_and_unknown_recovery() {
        let mut generation = NativeTargetGeneration::default();
        assert!(!generation.is_known());
        assert!(generation.advance());
        assert!(generation.is_known());
        let previous = generation;
        assert!(generation.advance());
        generation.invalidate_unknown();
        assert!(!generation.is_known());
        assert!(generation.advance());
        assert!(generation.is_known());
        assert_ne!(generation, previous);
    }

    #[test]
    fn target_generation_does_not_wrap_after_exhaustion() {
        let mut generation = NativeTargetGeneration::from_test_serial(u64::MAX);
        assert!(!generation.advance());
        assert!(!generation.is_known());
        assert!(!generation.advance());
    }

    #[test]
    fn surface_recovery_counters_saturate_and_convert() {
        let mut state = NativeSurfaceRecoveryState::default();
        state.observe_acquire_error(&vello::wgpu::SurfaceError::Lost);
        state.observe_acquire_error(&vello::wgpu::SurfaceError::Outdated);
        state.observe_acquire_error(&vello::wgpu::SurfaceError::Timeout);
        state.observe_acquire_error(&vello::wgpu::SurfaceError::Other);
        state.record_completed_reconfigure();
        state.record_zero_size_deferral();
        state.record_retry_request();
        assert!(state.record_timeout_retry_request(true));

        assert_eq!(
            state.diagnostics(),
            NativeSurfaceRecoveryDiagnostics {
                lost: 1,
                outdated: 1,
                timeouts: 1,
                others: 1,
                completed_reconfigures: 1,
                zero_size_deferrals: 1,
                retry_requests: 1,
                timeout_retry_requests: 1,
                other_retry_requests: 0,
            }
        );

        state.lost = u64::MAX;
        state.outdated = u64::MAX;
        state.timeouts = u64::MAX;
        state.others = u64::MAX;
        state.completed_reconfigures = u64::MAX;
        state.zero_size_deferrals = u64::MAX;
        state.retry_requests = u64::MAX;
        state.timeout_retry_requests = u64::MAX;
        state.other_retry_requests = u64::MAX;
        state.observe_acquire_error(&vello::wgpu::SurfaceError::Lost);
        state.observe_acquire_error(&vello::wgpu::SurfaceError::Outdated);
        state.observe_acquire_error(&vello::wgpu::SurfaceError::Timeout);
        state.observe_acquire_error(&vello::wgpu::SurfaceError::Other);
        state.record_completed_reconfigure();
        state.record_zero_size_deferral();
        state.record_retry_request();
        state.rearm_transient_retry();
        assert!(state.record_timeout_retry_request(true));
        state.rearm_transient_retry();
        assert!(state.record_other_retry_request(true));

        let diagnostics = NativeSurfaceRecoveryDiagnostics::from(state);
        assert_eq!(diagnostics.other_retry_requests, u64::MAX);
        assert_eq!(
            diagnostics,
            NativeSurfaceRecoveryDiagnostics {
                lost: u64::MAX,
                outdated: u64::MAX,
                timeouts: u64::MAX,
                others: u64::MAX,
                completed_reconfigures: u64::MAX,
                zero_size_deferrals: u64::MAX,
                retry_requests: u64::MAX,
                timeout_retry_requests: u64::MAX,
                other_retry_requests: u64::MAX,
            }
        );
    }

    #[test]
    fn consecutive_timeout_retry_is_one_shot_until_success_or_target_transition() {
        let mut state = NativeSurfaceRecoveryState::default();

        state.observe_acquire_error(&vello::wgpu::SurfaceError::Timeout);
        assert!(state.record_timeout_retry_request(true));
        state.observe_acquire_error(&vello::wgpu::SurfaceError::Other);
        assert!(!state.record_other_retry_request(true));

        // A successful acquisition rearms the next consecutive sequence.
        state.rearm_transient_retry();
        state.observe_acquire_error(&vello::wgpu::SurfaceError::Other);
        assert!(state.record_other_retry_request(true));
        state.observe_acquire_error(&vello::wgpu::SurfaceError::Timeout);
        assert!(!state.record_timeout_retry_request(true));

        // Rearming is idempotent, so an authoritative transition grants only
        // one later retry even if another transition notification is repeated.
        state.rearm_transient_retry();
        state.rearm_transient_retry();
        state.observe_acquire_error(&vello::wgpu::SurfaceError::Timeout);
        assert!(state.record_timeout_retry_request(true));
        state.observe_acquire_error(&vello::wgpu::SurfaceError::Other);
        assert!(!state.record_other_retry_request(true));

        // A minimized window consumes the sequence permit without scheduling
        // a retry; only a later success or target transition can rearm it.
        state.rearm_transient_retry();
        state.observe_acquire_error(&vello::wgpu::SurfaceError::Other);
        assert!(!state.record_other_retry_request(false));
        state.observe_acquire_error(&vello::wgpu::SurfaceError::Timeout);
        assert!(!state.record_timeout_retry_request(true));

        let diagnostics = state.diagnostics();
        assert_eq!(diagnostics.timeouts, 4);
        assert_eq!(diagnostics.others, 4);
        assert_eq!(diagnostics.timeout_retry_requests, 2);
        assert_eq!(diagnostics.other_retry_requests, 1);
    }
}
