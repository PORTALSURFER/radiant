//! Focused state groups owned by the generic native Vello runner.

use super::NativeAdapterGeneration;
use super::PendingGpuSurfaceWheel;
use super::PendingScrollbarDrag;
use super::input::NativePointerGestureLatch;
use super::native_resource_maintenance::{
    NativeResourceMaintenanceBinding, NativeResourceMaintenanceKernel,
    NativeResourceMaintenanceSlot,
};
use super::native_visual_packet::NativeVisualRequestMailbox;
use super::submission_completion::NativeSubmissionCompletionWitness;
use super::window_environment::{AccessibilityDisplaySnapshot, MonitorFingerprint};
use super::{
    CompositedBaseFrame, FrameWork, FrameWorkReason, GpuSurfaceRenderer, PostGpuOverlayRenderer,
    RuntimeUserEvent,
};
use crate::gui::input::{InputSequence, InputSequenceRange};
use crate::gui::types::Vector2;
use crate::gui::types::{Point, Rect as UiRect};
use crate::gui_runtime::native_vello::startup::StartupTimingProfile;
use crate::runtime::NativeWindowDiagnosticIdentity;
use crate::widgets::WidgetCursor;
use std::{
    collections::VecDeque,
    sync::Arc,
    time::{Duration, Instant},
};
use vello::{Renderer, util::RenderSurface, wgpu};
use winit::{
    dpi::PhysicalSize,
    event_loop::EventLoopProxy,
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

/// Retained GPU state that belongs to one native window resource generation.
///
/// This is constructed afresh with each published resource bundle. It must not
/// be retained in [`NativeVelloFrameState`](super::NativeVelloFrameState),
/// because quarantining a bundle is the fence that isolates every retained
/// WGPU resource for its exact adapter generation.
pub(super) struct NativeWindowGpuResources {
    pub(super) gpu_surface_renderer: GpuSurfaceRenderer,
    pub(super) post_gpu_overlay_renderer: PostGpuOverlayRenderer,
    pub(super) composited_base_frame: Option<CompositedBaseFrame>,
}

impl NativeWindowGpuResources {
    pub(super) fn new() -> Self {
        Self {
            gpu_surface_renderer: GpuSurfaceRenderer::default(),
            post_gpu_overlay_renderer: PostGpuOverlayRenderer::default(),
            composited_base_frame: None,
        }
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
    pub(super) gpu_resources: NativeWindowGpuResources,
    pub(super) completion_witness: NativeSubmissionCompletionWitness,
}

impl NativeWindowResourceBundle {
    pub(super) fn new(
        generation: NativeAdapterGeneration,
        render_surface: RenderSurface<'static>,
        renderer: Renderer,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        event_proxy: EventLoopProxy<RuntimeUserEvent>,
    ) -> Option<Self> {
        if !generation.is_known() {
            return None;
        }
        let completion_witness =
            NativeSubmissionCompletionWitness::new(generation, device, queue, event_proxy);
        if completion_witness.generation() != generation {
            return None;
        }
        Some(Self {
            generation,
            render_surface,
            renderer,
            gpu_resources: NativeWindowGpuResources::new(),
            completion_witness,
        })
    }

    pub(super) fn record_successful_native_submission(&mut self) {
        self.completion_witness.record_successful_submission();
    }

    pub(super) fn maintain_completion(&mut self) -> bool {
        self.completion_witness.maintain()
    }

    pub(super) fn maintenance_binding(
        &self,
        slot: NativeResourceMaintenanceSlot,
    ) -> NativeResourceMaintenanceBinding {
        NativeResourceMaintenanceBinding::new(
            slot,
            self.generation,
            self.completion_witness.maintenance_identity(),
        )
    }

    pub(super) fn maintenance_pending(&self) -> bool {
        self.completion_witness.maintenance_pending()
    }

    pub(super) fn maintain_completion_once(&mut self) -> bool {
        self.completion_witness.maintain_once()
    }

    pub(super) const fn retirement_eligible(&self) -> bool {
        self.completion_witness.retirement_eligible()
    }
}

/// One event-loop maintenance turn may physically drop at most one quarantined
/// native bundle across the primary and all auxiliary runners.
pub(super) struct NativeResourceMaintenanceTurn {
    drop_available: bool,
    pending: bool,
}

impl Default for NativeResourceMaintenanceTurn {
    fn default() -> Self {
        Self::new()
    }
}

impl NativeResourceMaintenanceTurn {
    pub(super) const fn new() -> Self {
        Self {
            drop_available: true,
            pending: false,
        }
    }

    #[cfg(test)]
    pub(super) const fn has_pending(&self) -> bool {
        self.pending
    }

    fn record_pending(&mut self) {
        self.pending = true;
    }

    fn record_drop(&mut self) {
        self.drop_available = false;
    }

    fn drop_one_ready<T>(
        &mut self,
        quarantine: &mut NativeResourceQuarantine<T>,
        ready: impl FnMut(&T) -> bool,
    ) -> bool {
        if !self.drop_available || !quarantine.drop_one_ready(ready) {
            return false;
        }
        self.record_drop();
        true
    }

    fn record_pending_if_ready<T>(
        &mut self,
        quarantine: &NativeResourceQuarantine<T>,
        ready: impl FnMut(&T) -> bool,
    ) {
        if quarantine.has_ready(ready) {
            self.record_pending();
        }
    }
}

const MAX_QUARANTINED_NATIVE_RESOURCES: usize = 2;

/// Bounded ownership for native resources that have left the admitted path.
///
/// The maintenance boundary removes at most one entry after its renderer-owned
/// completion witness is ready; it never authorizes synchronous waiting or GPU
/// work.
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

    fn drop_one_ready(&mut self, mut ready: impl FnMut(&T) -> bool) -> bool {
        let Some(index) = self.entries.iter().position(&mut ready) else {
            return false;
        };
        let _ = self.entries.remove(index);
        true
    }

    fn has_ready(&self, mut ready: impl FnMut(&T) -> bool) -> bool {
        self.entries.iter().any(&mut ready)
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

fn maintain_native_resource_entries<T>(
    active: &mut Option<T>,
    quarantine: &mut NativeResourceQuarantine<T>,
    turn: &mut NativeResourceMaintenanceTurn,
    mut maintain: impl FnMut(&mut T) -> bool,
    mut ready: impl FnMut(&T) -> bool,
) {
    if let Some(entry) = active.as_mut()
        && maintain(entry)
    {
        turn.record_pending();
    }
    for entry in &mut quarantine.entries {
        if maintain(entry) {
            turn.record_pending();
        }
    }
    let _ = turn.drop_one_ready(quarantine, &mut ready);
    turn.record_pending_if_ready(quarantine, &mut ready);
}

fn retire_native_resource_entries<T>(
    active: &mut Option<T>,
    quarantine: &mut NativeResourceQuarantine<T>,
    turn: &mut NativeResourceMaintenanceTurn,
    mut maintain: impl FnMut(&mut T) -> bool,
    mut ready: impl FnMut(&T) -> bool,
) -> bool {
    if active.is_some() {
        if quarantine.is_full() {
            // Keep the active entry owned until a later bounded turn frees
            // quarantine capacity; never drop it just to make room.
            turn.record_pending();
        } else if let Some(entry) = active.take() {
            match quarantine.try_push(entry) {
                Ok(()) => {}
                Err(entry) => {
                    // Preserve the active entry if the capacity reservation
                    // ever changes between the check and the push.
                    *active = Some(entry);
                    turn.record_pending();
                }
            }
        }
    }
    maintain_native_resource_entries(active, quarantine, turn, &mut maintain, &mut ready);
    active.is_none() && quarantine.is_empty()
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct NativeImeCursorAreaPublication {
    window_id: WindowId,
    native_scale_generation: NativeTargetGeneration,
    native_dpi_scale: crate::theme::DpiScale,
    area: UiRect,
}

#[derive(Default)]
pub(super) struct NativeImeCursorAreaCache {
    publication: Option<NativeImeCursorAreaPublication>,
}

impl NativeImeCursorAreaCache {
    pub(super) fn candidate_to_publish(
        &mut self,
        window_id: WindowId,
        native_scale_generation: NativeTargetGeneration,
        native_dpi_scale: crate::theme::DpiScale,
        candidate: Option<UiRect>,
    ) -> Option<UiRect> {
        let Some(area) = candidate.filter(|area| area.has_finite_positive_area()) else {
            self.invalidate();
            return None;
        };
        if !native_scale_generation.is_known() {
            self.invalidate();
            return None;
        }
        let publication = NativeImeCursorAreaPublication {
            window_id,
            native_scale_generation,
            native_dpi_scale,
            area,
        };
        (self.publication != Some(publication)).then_some(area)
    }

    pub(super) fn record(
        &mut self,
        window_id: WindowId,
        native_scale_generation: NativeTargetGeneration,
        native_dpi_scale: crate::theme::DpiScale,
        area: UiRect,
    ) {
        self.publication = Some(NativeImeCursorAreaPublication {
            window_id,
            native_scale_generation,
            native_dpi_scale,
            area,
        });
    }

    pub(super) fn invalidate(&mut self) {
        self.publication = None;
    }
}

#[derive(Default)]
pub(super) struct NativeRunnerWindowState {
    pub(super) id: Option<WindowId>,
    pub(super) window: Option<Arc<Window>>,
    pub(super) native_resources: Option<NativeWindowResourceBundle>,
    pub(super) quarantined_native_resources: NativeResourceQuarantine<NativeWindowResourceBundle>,
    /// Round-robin position for normal Running maintenance in Q0, Q1, Active
    /// order. It advances only after an exact ticket executes successfully.
    pub(super) native_resource_maintenance_cursor: u8,
    pub(super) native_dpi_scale: crate::theme::DpiScale,
    pub(super) dpi_scale: crate::theme::DpiScale,
    pub(super) dpi_scale_override: Option<crate::theme::DpiScale>,
    pub(super) native_window_focused: bool,
    /// Last visibility state explicitly selected by Radiant.  This is a
    /// display/lifecycle restoration hint only; it is never an eligibility
    /// or presentation authority.
    pub(super) logical_window_visible: bool,
    /// A requested packet may cross a fenced target only when an explicit,
    /// bounded recovery path armed it.  Unsolicited redraws never consume this
    /// exception.
    pub(super) requested_recovery_redraw: bool,
    pub(super) native_focus_lost: bool,
    pub(super) monitor_fingerprint: Option<MonitorFingerprint>,
    pub(super) accessibility_display: AccessibilityDisplaySnapshot,
    pub(super) environment: crate::runtime::WindowEnvironment,
    pub(super) target_generation: NativeTargetGeneration,
    pub(super) native_surface_target_fenced: bool,
    pub(super) surface_recovery: NativeSurfaceRecoveryState,
    pub(super) ime_cursor_area_cache: NativeImeCursorAreaCache,
    pub(super) native_visual_requests: NativeVisualRequestMailbox,
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

    pub(super) fn quarantine_active_native_resources(&mut self) -> bool {
        let Some(active) = self.native_resources.take() else {
            return true;
        };
        match self.quarantined_native_resources.try_push(active) {
            Ok(()) => true,
            Err(active) => {
                self.native_resources = Some(active);
                false
            }
        }
    }

    pub(super) fn maintain_native_resources(&mut self, turn: &mut NativeResourceMaintenanceTurn) {
        maintain_native_resource_entries(
            &mut self.native_resources,
            &mut self.quarantined_native_resources,
            turn,
            NativeWindowResourceBundle::maintain_completion,
            NativeWindowResourceBundle::retirement_eligible,
        );
    }

    /// Select one exact active/quarantine slot for normal Running maintenance.
    /// The snapshot is fixed-capacity and never falls back to a broad scan at
    /// execution time.
    pub(super) fn native_resource_maintenance_candidate(
        &self,
    ) -> Option<NativeResourceMaintenanceBinding> {
        NativeResourceMaintenanceKernel::select(
            self.native_resource_maintenance_bindings(),
            self.native_resource_maintenance_cursor,
        )
    }

    /// Return the current binding for one exact positional slot.  This is
    /// deliberately separate from candidate selection: revalidation must not
    /// scan to another slot after the scheduler has issued a ticket.
    pub(super) fn native_resource_maintenance_binding(
        &self,
        slot: NativeResourceMaintenanceSlot,
    ) -> Option<NativeResourceMaintenanceBinding> {
        let index = match slot {
            NativeResourceMaintenanceSlot::Quarantine(0) => 0,
            NativeResourceMaintenanceSlot::Quarantine(1) => 1,
            NativeResourceMaintenanceSlot::Active => 2,
            NativeResourceMaintenanceSlot::Quarantine(_) => return None,
        };
        self.native_resource_maintenance_bindings()[index]
    }

    fn native_resource_maintenance_bindings(
        &self,
    ) -> [Option<NativeResourceMaintenanceBinding>; 3] {
        let mut quarantine = [None; 2];
        for (index, resources) in self.quarantined_native_resources.entries.iter().enumerate() {
            if index >= quarantine.len() {
                break;
            }
            if resources.maintenance_pending() || resources.retirement_eligible() {
                quarantine[index] = Some(
                    resources.maintenance_binding(NativeResourceMaintenanceSlot::Quarantine(
                        index as u8,
                    )),
                );
            }
        }
        let active = self.native_resources.as_ref().and_then(|resources| {
            resources
                .maintenance_pending()
                .then(|| resources.maintenance_binding(NativeResourceMaintenanceSlot::Active))
        });
        [quarantine[0], quarantine[1], active]
    }

    pub(super) fn advance_native_resource_maintenance_cursor(
        &mut self,
        slot: NativeResourceMaintenanceSlot,
        quarantine_removed: bool,
    ) {
        self.native_resource_maintenance_cursor =
            if quarantine_removed && slot == NativeResourceMaintenanceSlot::Quarantine(0) {
                0
            } else {
                NativeResourceMaintenanceKernel::next_cursor(slot)
            };
    }

    /// Execute one already-admitted exact slot.  A false result is a current
    /// evidence veto and performs no poll, rearm, removal, or fallback scan.
    pub(super) fn maintain_native_resource_slot(
        &mut self,
        binding: NativeResourceMaintenanceBinding,
    ) -> Option<bool> {
        match binding.slot() {
            NativeResourceMaintenanceSlot::Active => {
                let resources = self.native_resources.as_mut()?;
                let current = resources.maintenance_binding(NativeResourceMaintenanceSlot::Active);
                if !NativeResourceMaintenanceKernel::is_current(binding, current) {
                    return None;
                }
                let _ = resources.maintain_completion_once();
                Some(false)
            }
            NativeResourceMaintenanceSlot::Quarantine(index) => {
                let index = usize::from(index);
                let resources = self.quarantined_native_resources.entries.get_mut(index)?;
                let current = resources
                    .maintenance_binding(NativeResourceMaintenanceSlot::Quarantine(index as u8));
                if !NativeResourceMaintenanceKernel::is_current(binding, current) {
                    return None;
                }
                let _ = resources.maintain_completion_once();
                let retirement_eligible = resources.retirement_eligible();
                if retirement_eligible {
                    let _ = self.quarantined_native_resources.entries.remove(index);
                }
                Some(retirement_eligible)
            }
        }
    }

    /// Move the complete active bundle into bounded retirement ownership,
    /// advance every exact-generation completion witness without waiting, and
    /// report completion only after both active and quarantined ownership is
    /// empty.
    pub(super) fn retire_native_resources(
        &mut self,
        turn: &mut NativeResourceMaintenanceTurn,
    ) -> bool {
        retire_native_resource_entries(
            &mut self.native_resources,
            &mut self.quarantined_native_resources,
            turn,
            NativeWindowResourceBundle::maintain_completion,
            NativeWindowResourceBundle::retirement_eligible,
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
    pub(super) input_sequence_allocator: NativeInputSequenceAllocator,
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
            input_sequence_allocator: NativeInputSequenceAllocator::default(),
            pending_gpu_surface_wheel: None,
            pending_scroll_container_wheel: None,
            pending_scrollbar_drag: None,
        }
    }
}

/// Checked, non-wrapping native input sequence allocator owned by one runner.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct NativeInputSequenceAllocator {
    next_sequence: Option<u64>,
}

impl Default for NativeInputSequenceAllocator {
    fn default() -> Self {
        Self {
            next_sequence: Some(1),
        }
    }
}

impl NativeInputSequenceAllocator {
    pub(super) fn allocate(&mut self) -> Option<InputSequenceRange> {
        let value = self.next_sequence?;
        self.next_sequence = value.checked_add(1);
        Some(InputSequenceRange::singleton(
            InputSequence::from_runtime_value(value),
        ))
    }
}

/// Parent-owned allocator for native-window diagnostic identities.
///
/// The checked next value is consumed even when a prospective auxiliary
/// runner later fails initialization, so an identity is never reused.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct NativeWindowDiagnosticIdentityAllocator {
    next_identity: Option<u64>,
}

impl Default for NativeWindowDiagnosticIdentityAllocator {
    fn default() -> Self {
        Self {
            next_identity: Some(1),
        }
    }
}

impl NativeWindowDiagnosticIdentityAllocator {
    pub(super) const fn for_primary() -> (Self, Option<NativeWindowDiagnosticIdentity>) {
        (
            Self {
                next_identity: Some(2),
            },
            Some(NativeWindowDiagnosticIdentity::from_runtime_value(1)),
        )
    }

    pub(super) const fn exhausted() -> Self {
        Self {
            next_identity: None,
        }
    }

    pub(super) fn allocate(&mut self) -> Option<NativeWindowDiagnosticIdentity> {
        let value = self.next_identity?;
        self.next_identity = value.checked_add(1);
        Some(NativeWindowDiagnosticIdentity::from_runtime_value(value))
    }
}

pub(super) struct NativeRunnerTimingState {
    pub(super) redraw_requested: bool,
    pub(super) redraw_requested_at: Option<Instant>,
    pub(super) latest_native_interactive_arrival: Option<Instant>,
    pub(super) startup_timing: StartupTimingProfile,
    pub(super) first_frame_presented: bool,
    pub(super) native_window_diagnostic_identity: Option<NativeWindowDiagnosticIdentity>,
    pub(super) next_frame_sequence: Option<u64>,
    pub(super) native_frame_snapshot_revision:
        super::native_encode_present::NativeFrameSnapshotRevisionAllocator,
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
    /// Next normal Running maintenance opportunity for this window.  Lifecycle
    /// maintenance uses its separate turn and does not consume this deadline.
    pub(super) native_resource_maintenance_deadline: Option<Instant>,
}

impl Default for NativeRunnerTimingState {
    fn default() -> Self {
        Self::new(Some(NativeWindowDiagnosticIdentity::from_runtime_value(1)))
    }
}

impl NativeRunnerTimingState {
    pub(super) fn new(
        native_window_diagnostic_identity: Option<NativeWindowDiagnosticIdentity>,
    ) -> Self {
        let now = Instant::now();
        Self {
            redraw_requested: false,
            redraw_requested_at: None,
            latest_native_interactive_arrival: None,
            startup_timing: StartupTimingProfile::new(),
            first_frame_presented: false,
            native_window_diagnostic_identity,
            next_frame_sequence: Some(1),
            native_frame_snapshot_revision: Default::default(),
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
            native_resource_maintenance_deadline: None,
        }
    }

    pub(super) fn allocate_frame_sequence(&mut self) -> Option<u64> {
        let frame_sequence = self.next_frame_sequence?;
        self.next_frame_sequence = frame_sequence.checked_add(1);
        Some(frame_sequence)
    }

    pub(super) fn record_native_interactive_arrival_if_enabled(
        &mut self,
        diagnostics_enabled: bool,
        arrived_at: Instant,
    ) {
        if diagnostics_enabled {
            self.latest_native_interactive_arrival = Some(arrived_at);
        }
    }

    pub(super) fn take_input_to_present_latency_us(
        &mut self,
        presented_at: Instant,
    ) -> Option<u64> {
        self.latest_native_interactive_arrival
            .take()
            .map(|arrived_at| {
                let micros = presented_at
                    .saturating_duration_since(arrived_at)
                    .as_micros();
                micros.min(u64::MAX as u128) as u64
            })
    }
}

#[cfg(test)]
mod tests {
    use super::{
        NativeImeCursorAreaCache, NativeInputSequenceAllocator, NativeResourceMaintenanceTurn,
        NativeResourceQuarantine, NativeRunnerInputState, NativeRunnerTimingState,
        NativeRunnerWindowState, NativeSurfaceRecoveryState, NativeTargetGeneration,
        NativeWindowDiagnosticIdentityAllocator, NativeWindowGpuResources,
    };
    use crate::gui::types::{Point, Rect};
    use crate::runtime::NativeSurfaceRecoveryDiagnostics;
    use crate::theme::DpiScale;
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };
    use std::time::{Duration, Instant};
    use winit::dpi::PhysicalSize;

    struct DropTracked {
        ready: bool,
        drops: Arc<AtomicUsize>,
    }

    impl DropTracked {
        fn new(ready: bool, drops: &Arc<AtomicUsize>) -> Self {
            Self {
                ready,
                drops: Arc::clone(drops),
            }
        }
    }

    impl Drop for DropTracked {
        fn drop(&mut self) {
            self.drops.fetch_add(1, Ordering::Relaxed);
        }
    }

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
    fn ime_cursor_area_cache_suppresses_repeats_only_after_recording_a_call() {
        let mut cache = NativeImeCursorAreaCache::default();
        let window_id = winit::window::WindowId::from(1);
        let generation = NativeTargetGeneration::from_test_serial(1);
        let first_area = Rect::from_min_max(Point::new(8.0, 10.0), Point::new(16.0, 24.0));

        assert_eq!(
            cache.candidate_to_publish(window_id, generation, DpiScale::ONE, Some(first_area)),
            Some(first_area)
        );
        // A candidate observation cannot suppress itself before the actual
        // Winit call has happened and been recorded.
        assert_eq!(
            cache.candidate_to_publish(window_id, generation, DpiScale::ONE, Some(first_area)),
            Some(first_area)
        );
        cache.record(window_id, generation, DpiScale::ONE, first_area);
        assert_eq!(
            cache.candidate_to_publish(window_id, generation, DpiScale::ONE, Some(first_area)),
            None
        );
    }

    #[test]
    fn ime_cursor_area_cache_republishes_when_native_dpi_changes_under_fixed_target_generation() {
        let mut cache = NativeImeCursorAreaCache::default();
        let window_id = winit::window::WindowId::from(1);
        let target_generation = NativeTargetGeneration::from_test_serial(1);
        let first_native_dpi_scale = DpiScale::new(1.0);
        let next_native_dpi_scale = DpiScale::new(2.0);
        let area = Rect::from_min_max(Point::new(8.0, 10.0), Point::new(16.0, 24.0));

        assert_eq!(
            cache.candidate_to_publish(
                window_id,
                target_generation,
                first_native_dpi_scale,
                Some(area),
            ),
            Some(area)
        );
        cache.record(window_id, target_generation, first_native_dpi_scale, area);
        assert_eq!(
            cache.candidate_to_publish(
                window_id,
                target_generation,
                first_native_dpi_scale,
                Some(area),
            ),
            None
        );

        // A fixed application override can keep the target generation and
        // logical caret area unchanged while the native monitor scale changes.
        assert_eq!(
            cache.candidate_to_publish(
                window_id,
                target_generation,
                next_native_dpi_scale,
                Some(area),
            ),
            Some(area)
        );
    }

    #[test]
    fn ime_cursor_area_cache_republishes_after_movement_invalidity_and_identity_changes() {
        let mut cache = NativeImeCursorAreaCache::default();
        let first_window = winit::window::WindowId::from(1);
        let replacement_window = winit::window::WindowId::from(2);
        let first_generation = NativeTargetGeneration::from_test_serial(1);
        let next_generation = NativeTargetGeneration::from_test_serial(2);
        let first_area = Rect::from_min_max(Point::new(8.0, 10.0), Point::new(16.0, 24.0));
        let moved_area = Rect::from_min_max(Point::new(20.0, 10.0), Point::new(28.0, 24.0));

        assert_eq!(
            cache.candidate_to_publish(
                first_window,
                first_generation,
                DpiScale::ONE,
                Some(first_area)
            ),
            Some(first_area)
        );
        cache.record(first_window, first_generation, DpiScale::ONE, first_area);

        assert_eq!(
            cache.candidate_to_publish(
                first_window,
                first_generation,
                DpiScale::ONE,
                Some(moved_area)
            ),
            Some(moved_area)
        );
        cache.record(first_window, first_generation, DpiScale::ONE, moved_area);
        assert_eq!(
            cache.candidate_to_publish(
                first_window,
                first_generation,
                DpiScale::ONE,
                Some(moved_area)
            ),
            None
        );

        // None/invalid evidence clears the suppression authority, even when
        // the next valid candidate is unchanged.
        assert_eq!(
            cache.candidate_to_publish(first_window, first_generation, DpiScale::ONE, None),
            None
        );
        let invalid_area = Rect::from_min_max(Point::new(20.0, 10.0), Point::new(20.0, 24.0));
        assert_eq!(
            cache.candidate_to_publish(
                first_window,
                first_generation,
                DpiScale::ONE,
                Some(invalid_area),
            ),
            None
        );
        assert_eq!(
            cache.candidate_to_publish(
                first_window,
                first_generation,
                DpiScale::ONE,
                Some(moved_area)
            ),
            Some(moved_area)
        );
        cache.record(first_window, first_generation, DpiScale::ONE, moved_area);

        // A changed target generation republishes the same logical area.
        assert_eq!(
            cache.candidate_to_publish(
                first_window,
                next_generation,
                DpiScale::ONE,
                Some(moved_area)
            ),
            Some(moved_area)
        );
        cache.record(first_window, next_generation, DpiScale::ONE, moved_area);

        // A different native WindowId starts a fresh publication generation,
        // even when its first valid area happens to be identical.
        assert_eq!(
            cache.candidate_to_publish(
                replacement_window,
                next_generation,
                DpiScale::ONE,
                Some(moved_area),
            ),
            Some(moved_area)
        );
    }

    #[test]
    fn fresh_window_gpu_state_is_quarantined_as_one_owned_entry() {
        let mut quarantine = NativeResourceQuarantine::default();
        let gpu_resources = NativeWindowGpuResources::new();

        assert!(gpu_resources.composited_base_frame.is_none());
        assert!(quarantine.try_push(gpu_resources).is_ok());
        assert_eq!(quarantine.len(), 1);

        let mut turn = NativeResourceMaintenanceTurn::new();
        assert!(turn.drop_one_ready(&mut quarantine, |_| true));
        assert!(quarantine.is_empty());
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
    fn pending_native_resource_is_retained_without_drop() {
        let drops = Arc::new(AtomicUsize::new(0));
        let mut quarantine = NativeResourceQuarantine::default();
        assert!(quarantine.try_push(DropTracked::new(false, &drops)).is_ok());
        let mut turn = NativeResourceMaintenanceTurn::new();

        assert!(!turn.drop_one_ready(&mut quarantine, |entry| entry.ready));
        assert_eq!(drops.load(Ordering::Relaxed), 0);
        assert_eq!(quarantine.len(), 1);
    }

    #[test]
    fn retiring_native_resource_moves_active_into_quarantine_without_dropping_pending_work() {
        let drops = Arc::new(AtomicUsize::new(0));
        let mut active = Some(DropTracked::new(false, &drops));
        let mut quarantine = NativeResourceQuarantine::default();
        let mut turn = NativeResourceMaintenanceTurn::new();

        assert!(!super::retire_native_resource_entries(
            &mut active,
            &mut quarantine,
            &mut turn,
            |entry| !entry.ready,
            |entry| entry.ready,
        ));
        assert!(active.is_none());
        assert_eq!(quarantine.len(), 1);
        assert_eq!(drops.load(Ordering::Relaxed), 0);
        assert!(turn.has_pending());
    }

    #[test]
    fn retiring_native_resource_retains_active_when_quarantine_is_full() {
        let drops = Arc::new(AtomicUsize::new(0));
        let mut active = Some(DropTracked::new(true, &drops));
        let mut quarantine = NativeResourceQuarantine::default();
        assert!(quarantine.try_push(DropTracked::new(false, &drops)).is_ok());
        assert!(quarantine.try_push(DropTracked::new(false, &drops)).is_ok());
        let mut turn = NativeResourceMaintenanceTurn::new();

        assert!(!super::retire_native_resource_entries(
            &mut active,
            &mut quarantine,
            &mut turn,
            |entry| !entry.ready,
            |entry| entry.ready,
        ));
        assert!(active.is_some());
        assert_eq!(quarantine.len(), 2);
        assert_eq!(drops.load(Ordering::Relaxed), 0);
        assert!(turn.has_pending());
    }

    #[test]
    fn retiring_native_resource_reports_empty_only_after_one_bounded_drop() {
        let drops = Arc::new(AtomicUsize::new(0));
        let mut active = Some(DropTracked::new(true, &drops));
        let mut quarantine = NativeResourceQuarantine::default();
        let mut turn = NativeResourceMaintenanceTurn::new();

        assert!(super::retire_native_resource_entries(
            &mut active,
            &mut quarantine,
            &mut turn,
            |entry| !entry.ready,
            |entry| entry.ready,
        ));
        assert!(active.is_none());
        assert!(quarantine.is_empty());
        assert_eq!(drops.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn completed_native_resource_reclaims_quarantine_capacity() {
        let drops = Arc::new(AtomicUsize::new(0));
        let mut quarantine = NativeResourceQuarantine::default();
        assert!(quarantine.try_push(DropTracked::new(true, &drops)).is_ok());
        assert!(quarantine.try_push(DropTracked::new(false, &drops)).is_ok());
        assert!(quarantine.is_full());
        let mut turn = NativeResourceMaintenanceTurn::new();

        assert!(turn.drop_one_ready(&mut quarantine, |entry| entry.ready));
        assert_eq!(drops.load(Ordering::Relaxed), 1);
        assert!(!quarantine.is_full());
        assert_eq!(quarantine.len(), 1);
    }

    #[test]
    fn native_resource_maintenance_drops_at_most_one_bundle_globally_per_turn() {
        let drops = Arc::new(AtomicUsize::new(0));
        let mut primary = NativeResourceQuarantine::default();
        let mut auxiliary = NativeResourceQuarantine::default();
        assert!(primary.try_push(DropTracked::new(true, &drops)).is_ok());
        assert!(auxiliary.try_push(DropTracked::new(true, &drops)).is_ok());
        let mut turn = NativeResourceMaintenanceTurn::new();

        assert!(turn.drop_one_ready(&mut primary, |entry| entry.ready));
        assert!(!turn.drop_one_ready(&mut auxiliary, |entry| entry.ready));
        turn.record_pending_if_ready(&auxiliary, |entry| entry.ready);
        assert_eq!(drops.load(Ordering::Relaxed), 1);
        assert!(primary.is_empty());
        assert_eq!(auxiliary.len(), 1);
        assert!(turn.has_pending());
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
    fn native_resource_publication_quarantines_the_old_complete_bundle_as_one_entry() {
        let mut active = Some(1);
        let mut quarantine = NativeResourceQuarantine::default();
        assert!(quarantine.try_push(2).is_ok());

        let publication = super::reserve_native_resource_publication(&mut active, &mut quarantine)
            .expect("one quarantine slot should admit a complete replacement");
        publication.publish(3);

        assert_eq!(active, Some(3));
        assert_eq!(quarantine.len(), 2);
        assert!(quarantine.entries.contains(&2));
        assert!(quarantine.entries.contains(&1));
    }

    #[test]
    fn abandoned_native_resource_publication_reservation_preserves_active_state() {
        let mut active = Some(1);
        let mut quarantine = NativeResourceQuarantine::default();
        assert!(quarantine.try_push(2).is_ok());

        {
            let reservation =
                super::reserve_native_resource_publication(&mut active, &mut quarantine);
            assert!(reservation.is_some());
            // Scope exit models an initialization error after reservation and
            // before native-resource publication.
        }

        assert_eq!(active, Some(1));
        assert_eq!(quarantine.len(), 1);
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
    fn native_frame_sequence_starts_at_one_and_increments_without_reuse() {
        let mut timing = NativeRunnerTimingState::default();

        assert_eq!(timing.allocate_frame_sequence(), Some(1));
        assert_eq!(timing.allocate_frame_sequence(), Some(2));
        assert_eq!(timing.allocate_frame_sequence(), Some(3));
    }

    #[test]
    fn native_frame_sequence_exhaustion_does_not_wrap_or_reuse() {
        let mut timing = NativeRunnerTimingState {
            next_frame_sequence: Some(u64::MAX - 1),
            ..NativeRunnerTimingState::default()
        };

        assert_eq!(timing.allocate_frame_sequence(), Some(u64::MAX - 1));
        assert_eq!(timing.allocate_frame_sequence(), Some(u64::MAX));
        assert_eq!(timing.allocate_frame_sequence(), None);
        assert_eq!(timing.next_frame_sequence, None);
    }

    #[test]
    fn native_input_arrival_is_latest_wins() {
        let mut timing = NativeRunnerTimingState::default();
        let first = Instant::now();
        let second = first + Duration::from_micros(10);

        timing.record_native_interactive_arrival_if_enabled(true, first);
        timing.record_native_interactive_arrival_if_enabled(true, second);

        assert_eq!(
            timing.take_input_to_present_latency_us(second + Duration::from_micros(7)),
            Some(7)
        );
    }

    #[test]
    fn native_input_arrival_is_retained_until_successful_presentation() {
        let mut timing = NativeRunnerTimingState::default();
        let arrived_at = Instant::now();

        timing.record_native_interactive_arrival_if_enabled(true, arrived_at);

        // A failed or skipped presentation does not call the consuming helper.
        assert_eq!(timing.latest_native_interactive_arrival, Some(arrived_at));
        assert_eq!(
            timing.take_input_to_present_latency_us(arrived_at + Duration::from_micros(13)),
            Some(13)
        );
    }

    #[test]
    fn native_input_arrival_is_consumed_once_after_successful_presentation() {
        let mut timing = NativeRunnerTimingState::default();
        let arrived_at = Instant::now();

        timing.record_native_interactive_arrival_if_enabled(true, arrived_at);

        assert_eq!(
            timing.take_input_to_present_latency_us(arrived_at + Duration::from_micros(17)),
            Some(17)
        );
        assert_eq!(
            timing.take_input_to_present_latency_us(arrived_at + Duration::from_micros(23)),
            None
        );
    }

    #[test]
    fn native_input_latency_saturates_when_presentation_clock_is_earlier() {
        let mut timing = NativeRunnerTimingState::default();
        let arrived_at = Instant::now();

        timing.record_native_interactive_arrival_if_enabled(true, arrived_at);

        assert_eq!(
            timing.take_input_to_present_latency_us(arrived_at - Duration::from_micros(1)),
            Some(0)
        );
    }

    #[test]
    fn disabled_native_input_diagnostics_do_not_create_state() {
        let mut timing = NativeRunnerTimingState::default();
        let arrived_at = Instant::now();

        timing.record_native_interactive_arrival_if_enabled(false, arrived_at);

        assert_eq!(timing.latest_native_interactive_arrival, None);
        assert_eq!(
            timing.take_input_to_present_latency_us(arrived_at + Duration::from_micros(1)),
            None
        );
    }

    #[test]
    fn native_window_diagnostic_identity_allocator_starts_at_one_and_increments() {
        let (mut allocator, primary) = NativeWindowDiagnosticIdentityAllocator::for_primary();

        assert_eq!(primary.map(|identity| identity.get()), Some(1));
        assert_eq!(allocator.allocate().map(|identity| identity.get()), Some(2));
        assert_eq!(allocator.allocate().map(|identity| identity.get()), Some(3));
        assert_eq!(allocator.allocate().map(|identity| identity.get()), Some(4));
    }

    #[test]
    fn native_window_diagnostic_identity_allocator_exhaustion_does_not_wrap_or_reuse() {
        let mut allocator = NativeWindowDiagnosticIdentityAllocator {
            next_identity: Some(u64::MAX),
        };

        assert_eq!(
            allocator.allocate().map(|identity| identity.get()),
            Some(u64::MAX)
        );
        assert_eq!(allocator.allocate(), None);
        assert_eq!(allocator.next_identity, None);
    }

    #[test]
    fn native_input_sequence_allocators_are_independent_per_runner_domain() {
        let mut primary = NativeRunnerInputState::default();
        let mut auxiliary = NativeRunnerInputState::default();

        let primary_first = primary
            .input_sequence_allocator
            .allocate()
            .expect("primary runner should allocate its first input sequence");
        let primary_second = primary
            .input_sequence_allocator
            .allocate()
            .expect("primary runner should allocate its second input sequence");
        let auxiliary_first = auxiliary
            .input_sequence_allocator
            .allocate()
            .expect("auxiliary runner should have its own first input sequence");

        assert_eq!(primary_first.start().runtime_value(), 1);
        assert_eq!(primary_first.end().runtime_value(), 1);
        assert_eq!(primary_second.start().runtime_value(), 2);
        assert_eq!(primary_second.end().runtime_value(), 2);
        assert_eq!(auxiliary_first.start().runtime_value(), 1);
        assert_eq!(auxiliary_first.end().runtime_value(), 1);
    }

    #[test]
    fn native_input_sequence_allocator_exhaustion_is_checked_and_permanent() {
        let mut allocator = NativeInputSequenceAllocator {
            next_sequence: Some(u64::MAX),
        };

        let last = allocator
            .allocate()
            .expect("the maximum representable sequence remains allocatable");
        assert_eq!(last.start().runtime_value(), u64::MAX);
        assert_eq!(last.end().runtime_value(), u64::MAX);
        assert_eq!(allocator.allocate(), None);
        assert_eq!(allocator.allocate(), None);
        assert_eq!(allocator.next_sequence, None);
    }

    #[test]
    fn native_window_diagnostic_identity_stays_fixed_in_timing_state() {
        let (_, identity) = NativeWindowDiagnosticIdentityAllocator::for_primary();
        let mut timing = NativeRunnerTimingState::new(identity);

        assert_eq!(timing.native_window_diagnostic_identity, identity);
        assert_eq!(timing.allocate_frame_sequence(), Some(1));
        timing.first_frame_presented = true;
        timing.next_frame_sequence = None;
        assert_eq!(timing.native_window_diagnostic_identity, identity);
    }

    #[test]
    fn zero_sized_lost_and_outdated_targets_defer_without_retry_permission() {
        let zero = PhysicalSize::new(0, 480);

        assert_eq!(
            super::surface_acquire_policy(vello::wgpu::SurfaceError::Lost, zero),
            super::SurfaceAcquirePolicy::Defer
        );
        assert_eq!(
            super::surface_acquire_policy(vello::wgpu::SurfaceError::Outdated, zero),
            super::SurfaceAcquirePolicy::Defer
        );
        assert_eq!(
            super::surface_acquire_policy(
                vello::wgpu::SurfaceError::Lost,
                PhysicalSize::new(640, 480)
            ),
            super::SurfaceAcquirePolicy::ReconfigureAndRetry
        );
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

    #[test]
    fn other_recovery_retry_is_one_fresh_requested_packet_only() {
        let mut state = NativeSurfaceRecoveryState::default();

        assert!(state.record_other_retry_request(true));
        assert!(!state.record_other_retry_request(true));

        state.rearm_transient_retry();
        assert!(!state.record_other_retry_request(false));
        assert!(!state.record_other_retry_request(true));
    }
}
