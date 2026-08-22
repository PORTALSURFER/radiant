//! Private, bounded WGPU timestamp acquisition for the primary native frame.
//!
//! The public runtime exposes only the backend-neutral sample.  This module
//! owns all WGPU query, resolve, mapping, and callback state so no WGPU type
//! crosses the generic runtime boundary.

use crate::runtime::{
    FrameGpuTimingOutcome, FrameGpuTimingSample, FrameGpuTimingUnavailableReason,
};
use std::sync::{
    Arc,
    atomic::{AtomicU8, AtomicU64, Ordering},
};
use std::time::Duration;
use vello::wgpu;
use winit::event_loop::EventLoopProxy;

use super::{NativeAdapterGeneration, RuntimeUserEvent};

const TIMING_SLOT_COUNT: usize = 4;
const TIMING_QUERIES_PER_SLOT: u32 = 2;
const TIMING_QUERY_COUNT: u32 = TIMING_SLOT_COUNT as u32 * TIMING_QUERIES_PER_SLOT;
const TIMING_BUFFER_SIZE: wgpu::BufferAddress = 16;

const CALLBACK_PENDING: u8 = 0;
const CALLBACK_MAPPED: u8 = 1;
const CALLBACK_MAPPING_FAILED: u8 = 2;
const CALLBACK_CANCELED: u8 = 3;

static NEXT_TIMING_RESOURCE_ID: AtomicU64 = AtomicU64::new(1);

fn allocate_resource_identity() -> Option<u64> {
    let mut current = NEXT_TIMING_RESOURCE_ID.load(Ordering::Relaxed);
    loop {
        let next = current.checked_add(1)?;
        match NEXT_TIMING_RESOURCE_ID.compare_exchange(
            current,
            next,
            Ordering::Relaxed,
            Ordering::Relaxed,
        ) {
            Ok(_) => return Some(current),
            Err(observed) => current = observed,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum NativeGpuTimingSupport {
    Disabled,
    Unsupported,
    Supported,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum GpuTimingSlotPhase {
    Available,
    Reserved,
    StartSubmitted,
    EndEncoded,
    ReadbackPending,
    CancelPending,
    Ready,
    Delivering,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct GpuTimingReservation {
    slot: u8,
    token: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum GpuTimingAdmission {
    Disabled,
    Unsupported,
    CapacityRefused,
    Reserved(GpuTimingReservation),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct GpuTimingCorrelation {
    window_identity: u64,
    frame_sequence: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct GpuTimingTerminal {
    correlation: GpuTimingCorrelation,
    outcome: FrameGpuTimingOutcome,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct GpuTimingSlotState {
    start_query: u32,
    end_query: u32,
    phase: GpuTimingSlotPhase,
    token: Option<u64>,
    correlation: Option<GpuTimingCorrelation>,
    terminal: Option<GpuTimingTerminal>,
}

impl GpuTimingSlotState {
    const fn new(slot: usize) -> Self {
        let first_query = slot as u32 * TIMING_QUERIES_PER_SLOT;
        Self {
            start_query: first_query,
            end_query: first_query + 1,
            phase: GpuTimingSlotPhase::Available,
            token: None,
            correlation: None,
            terminal: None,
        }
    }

    const fn clear(&mut self) {
        self.phase = GpuTimingSlotPhase::Available;
        self.token = None;
        self.correlation = None;
        self.terminal = None;
    }

    fn matches(&self, reservation: GpuTimingReservation) -> bool {
        self.token == Some(reservation.token)
            && reservation.slot < TIMING_SLOT_COUNT as u8
            && self.phase != GpuTimingSlotPhase::Available
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum GpuTimingCancelAction {
    Ignored,
    Recycled,
    AwaitCallback,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum GpuTimingMapping {
    Values { start: u64, end: u64 },
    Failed,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum GpuTimingCallbackDisposition {
    Ignored,
    Recycled,
    Ready,
}

/// Pure fixed-capacity state for one exact-generation timing pool.
///
/// Keeping this kernel independent from WGPU makes stale callbacks, wrapping
/// timestamp arithmetic, and failed-frame cancellation directly testable.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct GpuTimingPoolState {
    support: NativeGpuTimingSupport,
    slots: [GpuTimingSlotState; TIMING_SLOT_COUNT],
    next_token: u64,
}

impl GpuTimingPoolState {
    const fn new(support: NativeGpuTimingSupport) -> Self {
        Self {
            support,
            slots: [
                GpuTimingSlotState::new(0),
                GpuTimingSlotState::new(1),
                GpuTimingSlotState::new(2),
                GpuTimingSlotState::new(3),
            ],
            next_token: 1,
        }
    }

    fn reserve(&mut self) -> GpuTimingAdmission {
        match self.support {
            NativeGpuTimingSupport::Disabled => return GpuTimingAdmission::Disabled,
            NativeGpuTimingSupport::Unsupported => return GpuTimingAdmission::Unsupported,
            NativeGpuTimingSupport::Supported => {}
        }
        let Some(index) = self
            .slots
            .iter()
            .position(|slot| slot.phase == GpuTimingSlotPhase::Available)
        else {
            return GpuTimingAdmission::CapacityRefused;
        };
        let Some(token) = self.next_token.checked_add(1) else {
            return GpuTimingAdmission::CapacityRefused;
        };
        let reservation = GpuTimingReservation {
            slot: index as u8,
            token: self.next_token,
        };
        self.next_token = token;
        let slot = &mut self.slots[index];
        slot.phase = GpuTimingSlotPhase::Reserved;
        slot.token = Some(reservation.token);
        slot.correlation = None;
        slot.terminal = None;
        GpuTimingAdmission::Reserved(reservation)
    }

    fn slot(&self, reservation: GpuTimingReservation) -> Option<&GpuTimingSlotState> {
        self.slots
            .get(usize::from(reservation.slot))
            .filter(|slot| slot.matches(reservation))
    }

    fn slot_mut(&mut self, reservation: GpuTimingReservation) -> Option<&mut GpuTimingSlotState> {
        self.slots
            .get_mut(usize::from(reservation.slot))
            .filter(|slot| slot.matches(reservation))
    }

    fn submit_start(&mut self, reservation: GpuTimingReservation) -> bool {
        let Some(slot) = self.slot_mut(reservation) else {
            return false;
        };
        if slot.phase != GpuTimingSlotPhase::Reserved {
            return false;
        }
        slot.phase = GpuTimingSlotPhase::StartSubmitted;
        true
    }

    fn encode_end(&mut self, reservation: GpuTimingReservation) -> bool {
        let Some(slot) = self.slot_mut(reservation) else {
            return false;
        };
        if slot.phase != GpuTimingSlotPhase::StartSubmitted {
            return false;
        }
        slot.phase = GpuTimingSlotPhase::EndEncoded;
        true
    }

    fn submit_readback(&mut self, reservation: GpuTimingReservation) -> bool {
        let Some(slot) = self.slot_mut(reservation) else {
            return false;
        };
        if slot.phase != GpuTimingSlotPhase::EndEncoded {
            return false;
        }
        slot.phase = GpuTimingSlotPhase::ReadbackPending;
        true
    }

    fn bind_success(
        &mut self,
        reservation: GpuTimingReservation,
        window_identity: u64,
        frame_sequence: u64,
    ) -> bool {
        let Some(slot) = self.slot_mut(reservation) else {
            return false;
        };
        if slot.phase != GpuTimingSlotPhase::ReadbackPending || slot.correlation.is_some() {
            return false;
        }
        slot.correlation = Some(GpuTimingCorrelation {
            window_identity,
            frame_sequence,
        });
        true
    }

    fn cancel(&mut self, reservation: GpuTimingReservation) -> GpuTimingCancelAction {
        let Some(slot) = self.slot_mut(reservation) else {
            return GpuTimingCancelAction::Ignored;
        };
        match slot.phase {
            GpuTimingSlotPhase::Reserved => {
                slot.clear();
                GpuTimingCancelAction::Recycled
            }
            GpuTimingSlotPhase::StartSubmitted | GpuTimingSlotPhase::EndEncoded => {
                slot.phase = GpuTimingSlotPhase::CancelPending;
                GpuTimingCancelAction::AwaitCallback
            }
            GpuTimingSlotPhase::ReadbackPending => {
                slot.phase = GpuTimingSlotPhase::CancelPending;
                GpuTimingCancelAction::AwaitCallback
            }
            GpuTimingSlotPhase::Available
            | GpuTimingSlotPhase::CancelPending
            | GpuTimingSlotPhase::Ready
            | GpuTimingSlotPhase::Delivering => GpuTimingCancelAction::Ignored,
        }
    }

    fn complete_callback(
        &mut self,
        reservation: GpuTimingReservation,
        mapping: GpuTimingMapping,
        timestamp_period: f32,
    ) -> GpuTimingCallbackDisposition {
        let Some(slot) = self.slot_mut(reservation) else {
            return GpuTimingCallbackDisposition::Ignored;
        };
        if slot.phase == GpuTimingSlotPhase::CancelPending {
            slot.clear();
            return GpuTimingCallbackDisposition::Recycled;
        }
        if slot.phase != GpuTimingSlotPhase::ReadbackPending {
            return GpuTimingCallbackDisposition::Ignored;
        }
        let Some(correlation) = slot.correlation else {
            slot.clear();
            return GpuTimingCallbackDisposition::Recycled;
        };
        let outcome = match mapping {
            GpuTimingMapping::Values { start, end } => {
                convert_timestamp_difference(start, end, timestamp_period)
            }
            GpuTimingMapping::Failed => {
                FrameGpuTimingOutcome::unavailable(FrameGpuTimingUnavailableReason::MappingFailed)
            }
        };
        slot.terminal = Some(GpuTimingTerminal {
            correlation,
            outcome,
        });
        slot.phase = GpuTimingSlotPhase::Ready;
        GpuTimingCallbackDisposition::Ready
    }

    fn prepare_delivery(&mut self, reservation: GpuTimingReservation) -> Option<GpuTimingTerminal> {
        let slot = self.slot_mut(reservation)?;
        if slot.phase != GpuTimingSlotPhase::Ready {
            return None;
        }
        slot.phase = GpuTimingSlotPhase::Delivering;
        slot.terminal
    }

    fn finish_delivery(&mut self, reservation: GpuTimingReservation) -> bool {
        let Some(slot) = self.slot_mut(reservation) else {
            return false;
        };
        if slot.phase != GpuTimingSlotPhase::Delivering {
            return false;
        }
        slot.clear();
        true
    }

    fn retirement_eligible(self) -> bool {
        self.slots
            .iter()
            .all(|slot| slot.phase == GpuTimingSlotPhase::Available)
    }

    fn maintenance_pending(self) -> bool {
        self.slots
            .iter()
            .any(|slot| slot.phase != GpuTimingSlotPhase::Available)
    }

    fn poll_required(self) -> bool {
        self.slots.iter().any(|slot| {
            matches!(
                slot.phase,
                GpuTimingSlotPhase::StartSubmitted
                    | GpuTimingSlotPhase::EndEncoded
                    | GpuTimingSlotPhase::ReadbackPending
                    | GpuTimingSlotPhase::CancelPending
            )
        })
    }
}

fn convert_timestamp_difference(
    start: u64,
    end: u64,
    timestamp_period: f32,
) -> FrameGpuTimingOutcome {
    if !timestamp_period.is_finite() || timestamp_period <= 0.0 {
        return FrameGpuTimingOutcome::unavailable(
            FrameGpuTimingUnavailableReason::ConversionFailed,
        );
    }
    let ticks = end.wrapping_sub(start);
    let nanos = (ticks as f64 * f64::from(timestamp_period)).round();
    let max_exclusive = u64::MAX as f64 + 1.0;
    if !nanos.is_finite() || nanos < 0.0 || nanos >= max_exclusive {
        return FrameGpuTimingOutcome::unavailable(
            FrameGpuTimingUnavailableReason::ConversionFailed,
        );
    }
    FrameGpuTimingOutcome::available(Duration::from_nanos(nanos as u64))
}

struct TimingCallbackSignal {
    result: AtomicU8,
    generation: NativeAdapterGeneration,
    resource_identity: u64,
    slot: u8,
    token: u64,
    proxy: EventLoopProxy<RuntimeUserEvent>,
}

impl TimingCallbackSignal {
    fn new(
        generation: NativeAdapterGeneration,
        resource_identity: u64,
        slot: u8,
        token: u64,
        proxy: EventLoopProxy<RuntimeUserEvent>,
    ) -> Arc<Self> {
        Arc::new(Self {
            result: AtomicU8::new(CALLBACK_PENDING),
            generation,
            resource_identity,
            slot,
            token,
            proxy,
        })
    }

    fn record(&self, result: u8) {
        if self
            .result
            .compare_exchange(
                CALLBACK_PENDING,
                result,
                Ordering::AcqRel,
                Ordering::Acquire,
            )
            .is_ok()
        {
            let _ = self
                .proxy
                .send_event(RuntimeUserEvent::NativeGpuTimingReady {
                    generation: self.generation,
                    resource_identity: self.resource_identity,
                    slot: self.slot,
                    token: self.token,
                });
        }
    }

    fn result(&self) -> u8 {
        self.result.load(Ordering::Acquire)
    }
}

struct NativeGpuTimingSlot {
    resolve: wgpu::Buffer,
    readback: wgpu::Buffer,
    callback: Option<Arc<TimingCallbackSignal>>,
}

pub(super) struct NativeGpuTimingDelivery {
    pub(super) reservation: GpuTimingReservation,
    pub(super) sample: FrameGpuTimingSample,
    pub(super) resource_identity: u64,
    mapped: bool,
}

struct NativeGpuTimingPool {
    state: GpuTimingPoolState,
    query_set: wgpu::QuerySet,
    slots: [NativeGpuTimingSlot; TIMING_SLOT_COUNT],
    device: wgpu::Device,
    queue: wgpu::Queue,
    timestamp_period: f32,
    generation: NativeAdapterGeneration,
    proxy: EventLoopProxy<RuntimeUserEvent>,
    resource_identity: u64,
}

impl NativeGpuTimingPool {
    fn new(
        generation: NativeAdapterGeneration,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        proxy: EventLoopProxy<RuntimeUserEvent>,
    ) -> Option<Self> {
        let required =
            wgpu::Features::TIMESTAMP_QUERY | wgpu::Features::TIMESTAMP_QUERY_INSIDE_ENCODERS;
        if !device.features().contains(required) {
            return None;
        }
        let resource_identity = allocate_resource_identity()?;
        let query_set = device.create_query_set(&wgpu::QuerySetDescriptor {
            label: Some("generic_native_frame_gpu_timing_queries"),
            ty: wgpu::QueryType::Timestamp,
            count: TIMING_QUERY_COUNT,
        });
        let slots = std::array::from_fn(|_| NativeGpuTimingSlot {
            resolve: device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("generic_native_frame_gpu_timing_resolve"),
                size: TIMING_BUFFER_SIZE,
                usage: wgpu::BufferUsages::QUERY_RESOLVE | wgpu::BufferUsages::COPY_SRC,
                mapped_at_creation: false,
            }),
            readback: device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("generic_native_frame_gpu_timing_readback"),
                size: TIMING_BUFFER_SIZE,
                usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
                mapped_at_creation: false,
            }),
            callback: None,
        });
        let pool = Self {
            state: GpuTimingPoolState::new(NativeGpuTimingSupport::Supported),
            query_set,
            slots,
            device: device.clone(),
            queue: queue.clone(),
            timestamp_period: queue.get_timestamp_period(),
            generation,
            proxy,
            resource_identity,
        };
        Some(pool)
    }

    fn reserve(&mut self) -> GpuTimingAdmission {
        self.state.reserve()
    }

    fn resource_identity(&self) -> u64 {
        self.resource_identity
    }

    fn submit_start(&mut self, reservation: GpuTimingReservation) -> bool {
        let Some(slot) = self.state.slot(reservation) else {
            return false;
        };
        let start_query = slot.start_query;
        if !self.state.submit_start(reservation) {
            return false;
        }
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("generic_native_frame_gpu_timing_start"),
            });
        encoder.write_timestamp(&self.query_set, start_query);
        self.queue.submit(std::iter::once(encoder.finish()));
        true
    }

    fn encode_end(
        &mut self,
        reservation: GpuTimingReservation,
        encoder: &mut wgpu::CommandEncoder,
    ) -> bool {
        let Some(slot) = self.state.slot(reservation) else {
            return false;
        };
        let (start_query, end_query) = (slot.start_query, slot.end_query);
        let index = usize::from(reservation.slot);
        if !self.state.encode_end(reservation) {
            return false;
        }
        encoder.write_timestamp(&self.query_set, end_query);
        encoder.resolve_query_set(
            &self.query_set,
            start_query..start_query + TIMING_QUERIES_PER_SLOT,
            &self.slots[index].resolve,
            0,
        );
        encoder.copy_buffer_to_buffer(
            &self.slots[index].resolve,
            0,
            &self.slots[index].readback,
            0,
            TIMING_BUFFER_SIZE,
        );
        true
    }

    fn submit_readback(&mut self, reservation: GpuTimingReservation) -> bool {
        let index = usize::from(reservation.slot);
        if !self.state.submit_readback(reservation) {
            return false;
        }
        let callback = TimingCallbackSignal::new(
            self.generation,
            self.resource_identity(),
            reservation.slot,
            reservation.token,
            self.proxy.clone(),
        );
        self.slots[index].callback = Some(Arc::clone(&callback));
        let callback_for_map = Arc::clone(&callback);
        self.slots[index]
            .readback
            .slice(..)
            .map_async(wgpu::MapMode::Read, move |result| {
                callback_for_map.record(if result.is_ok() {
                    CALLBACK_MAPPED
                } else {
                    CALLBACK_MAPPING_FAILED
                });
            });
        true
    }

    fn cancel(&mut self, reservation: GpuTimingReservation) -> bool {
        let index = usize::from(reservation.slot);
        match self.state.cancel(reservation) {
            GpuTimingCancelAction::Ignored => false,
            GpuTimingCancelAction::Recycled => true,
            GpuTimingCancelAction::AwaitCallback => {
                let needs_queue_callback = self
                    .state
                    .slot(reservation)
                    .is_some_and(|slot| slot.phase == GpuTimingSlotPhase::CancelPending)
                    && self.slots[index].callback.is_none();
                if needs_queue_callback {
                    let callback = TimingCallbackSignal::new(
                        self.generation,
                        self.resource_identity(),
                        reservation.slot,
                        reservation.token,
                        self.proxy.clone(),
                    );
                    self.slots[index].callback = Some(Arc::clone(&callback));
                    self.queue.on_submitted_work_done(move || {
                        callback.record(CALLBACK_CANCELED);
                    });
                }
                true
            }
        }
    }

    fn bind_success(
        &mut self,
        reservation: GpuTimingReservation,
        window_identity: u64,
        frame_sequence: u64,
    ) -> bool {
        self.state
            .bind_success(reservation, window_identity, frame_sequence)
    }

    fn prepare_delivery(
        &mut self,
        reservation: GpuTimingReservation,
    ) -> Option<NativeGpuTimingDelivery> {
        let index = usize::from(reservation.slot);
        let callback = self.slots.get(index)?.callback.as_ref()?.clone();
        let phase = self.state.slot(reservation)?.phase;
        match (phase, callback.result()) {
            (GpuTimingSlotPhase::CancelPending, CALLBACK_CANCELED)
            | (GpuTimingSlotPhase::CancelPending, CALLBACK_MAPPING_FAILED) => {
                self.state.complete_callback(
                    reservation,
                    GpuTimingMapping::Failed,
                    self.timestamp_period,
                );
                self.slots[index].callback = None;
                None
            }
            (GpuTimingSlotPhase::CancelPending, CALLBACK_MAPPED) => {
                self.slots[index].readback.unmap();
                self.state.complete_callback(
                    reservation,
                    GpuTimingMapping::Values { start: 0, end: 0 },
                    self.timestamp_period,
                );
                self.slots[index].callback = None;
                None
            }
            (GpuTimingSlotPhase::ReadbackPending, CALLBACK_MAPPING_FAILED) => {
                if self.state.complete_callback(
                    reservation,
                    GpuTimingMapping::Failed,
                    self.timestamp_period,
                ) != GpuTimingCallbackDisposition::Ready
                {
                    return None;
                }
                let terminal = self.state.prepare_delivery(reservation)?;
                self.slots[index].callback = None;
                Some(NativeGpuTimingDelivery {
                    reservation,
                    sample: FrameGpuTimingSample::new(
                        terminal.correlation.window_identity,
                        terminal.correlation.frame_sequence,
                        terminal.outcome,
                    ),
                    resource_identity: self.resource_identity(),
                    mapped: false,
                })
            }
            (GpuTimingSlotPhase::ReadbackPending, CALLBACK_MAPPED) => {
                let range = self.slots[index].readback.slice(..).get_mapped_range();
                let mapping = if range.len() >= TIMING_BUFFER_SIZE as usize {
                    let mut start_bytes = [0; 8];
                    let mut end_bytes = [0; 8];
                    start_bytes.copy_from_slice(&range[0..8]);
                    end_bytes.copy_from_slice(&range[8..16]);
                    GpuTimingMapping::Values {
                        start: u64::from_ne_bytes(start_bytes),
                        end: u64::from_ne_bytes(end_bytes),
                    }
                } else {
                    GpuTimingMapping::Failed
                };
                drop(range);
                if self
                    .state
                    .complete_callback(reservation, mapping, self.timestamp_period)
                    != GpuTimingCallbackDisposition::Ready
                {
                    self.slots[index].readback.unmap();
                    return None;
                }
                let terminal = self.state.prepare_delivery(reservation)?;
                self.slots[index].callback = None;
                Some(NativeGpuTimingDelivery {
                    reservation,
                    sample: FrameGpuTimingSample::new(
                        terminal.correlation.window_identity,
                        terminal.correlation.frame_sequence,
                        terminal.outcome,
                    ),
                    resource_identity: self.resource_identity(),
                    mapped: true,
                })
            }
            _ => None,
        }
    }

    fn finish_delivery(&mut self, delivery: NativeGpuTimingDelivery) -> bool {
        if delivery.mapped {
            self.slots[usize::from(delivery.reservation.slot)]
                .readback
                .unmap();
        }
        self.state.finish_delivery(delivery.reservation)
    }

    fn maintain(&mut self) -> bool {
        if self.state.poll_required() {
            let _ = self.device.poll(wgpu::PollType::Poll);
        }
        self.state.maintenance_pending()
    }

    fn retirement_eligible(&self) -> bool {
        self.state.retirement_eligible()
    }
}

pub(super) struct NativeGpuTimingResources {
    support: NativeGpuTimingSupport,
    pool: Option<NativeGpuTimingPool>,
}

impl NativeGpuTimingResources {
    pub(super) const fn disabled() -> Self {
        Self {
            support: NativeGpuTimingSupport::Disabled,
            pool: None,
        }
    }

    pub(super) fn new(
        enabled: bool,
        generation: NativeAdapterGeneration,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        proxy: EventLoopProxy<RuntimeUserEvent>,
    ) -> Self {
        if !enabled {
            return Self::disabled();
        }
        let Some(pool) = NativeGpuTimingPool::new(generation, device, queue, proxy) else {
            return Self {
                support: NativeGpuTimingSupport::Unsupported,
                pool: None,
            };
        };
        Self {
            support: NativeGpuTimingSupport::Supported,
            pool: Some(pool),
        }
    }

    pub(super) fn reserve(&mut self) -> GpuTimingAdmission {
        self.pool.as_mut().map_or_else(
            || match self.support {
                NativeGpuTimingSupport::Disabled => GpuTimingAdmission::Disabled,
                NativeGpuTimingSupport::Unsupported => GpuTimingAdmission::Unsupported,
                NativeGpuTimingSupport::Supported => GpuTimingAdmission::CapacityRefused,
            },
            NativeGpuTimingPool::reserve,
        )
    }

    pub(super) fn submit_start(&mut self, reservation: GpuTimingReservation) -> bool {
        self.pool
            .as_mut()
            .is_some_and(|pool| pool.submit_start(reservation))
    }

    pub(super) fn resource_identity_matches(&self, resource_identity: u64) -> bool {
        self.pool
            .as_ref()
            .is_some_and(|pool| pool.resource_identity() == resource_identity)
    }

    pub(super) fn encode_end(
        &mut self,
        reservation: GpuTimingReservation,
        encoder: &mut wgpu::CommandEncoder,
    ) -> bool {
        self.pool
            .as_mut()
            .is_some_and(|pool| pool.encode_end(reservation, encoder))
    }

    pub(super) fn submit_readback(&mut self, reservation: GpuTimingReservation) -> bool {
        self.pool
            .as_mut()
            .is_some_and(|pool| pool.submit_readback(reservation))
    }

    pub(super) fn cancel(&mut self, reservation: GpuTimingReservation) -> bool {
        self.pool
            .as_mut()
            .is_some_and(|pool| pool.cancel(reservation))
    }

    pub(super) fn bind_success(
        &mut self,
        reservation: GpuTimingReservation,
        window_identity: u64,
        frame_sequence: u64,
    ) -> bool {
        self.pool
            .as_mut()
            .is_some_and(|pool| pool.bind_success(reservation, window_identity, frame_sequence))
    }

    pub(super) fn prepare_delivery(
        &mut self,
        slot: u8,
        token: u64,
    ) -> Option<NativeGpuTimingDelivery> {
        self.pool
            .as_mut()?
            .prepare_delivery(GpuTimingReservation { slot, token })
    }

    pub(super) fn finish_delivery(&mut self, delivery: NativeGpuTimingDelivery) -> bool {
        self.pool
            .as_mut()
            .is_some_and(|pool| pool.finish_delivery(delivery))
    }

    pub(super) fn maintain(&mut self) -> bool {
        self.pool
            .as_mut()
            .is_some_and(NativeGpuTimingPool::maintain)
    }

    pub(super) fn maintenance_pending(&self) -> bool {
        self.pool
            .as_ref()
            .is_some_and(|pool| pool.state.maintenance_pending())
    }

    pub(super) fn retirement_eligible(&self) -> bool {
        self.pool
            .as_ref()
            .is_none_or(NativeGpuTimingPool::retirement_eligible)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn supported_state() -> GpuTimingPoolState {
        GpuTimingPoolState::new(NativeGpuTimingSupport::Supported)
    }

    fn reserved(state: &mut GpuTimingPoolState) -> GpuTimingReservation {
        let GpuTimingAdmission::Reserved(reservation) = state.reserve() else {
            panic!("supported state should reserve a slot")
        };
        reservation
    }

    fn make_readback_pending(state: &mut GpuTimingPoolState) -> GpuTimingReservation {
        let reservation = reserved(state);
        assert!(state.submit_start(reservation));
        assert!(state.encode_end(reservation));
        assert!(state.submit_readback(reservation));
        assert!(state.bind_success(reservation, 7, 11));
        reservation
    }

    #[test]
    fn disabled_and_unsupported_never_reserve_or_do_work() {
        assert_eq!(
            GpuTimingPoolState::new(NativeGpuTimingSupport::Disabled).reserve(),
            GpuTimingAdmission::Disabled
        );
        assert_eq!(
            GpuTimingPoolState::new(NativeGpuTimingSupport::Unsupported).reserve(),
            GpuTimingAdmission::Unsupported
        );
    }

    #[test]
    fn supported_pool_has_four_slots_and_refuses_the_fifth() {
        let mut state = supported_state();
        let reservations = [
            reserved(&mut state),
            reserved(&mut state),
            reserved(&mut state),
            reserved(&mut state),
        ];

        assert_eq!(state.reserve(), GpuTimingAdmission::CapacityRefused);
        assert!(
            reservations
                .iter()
                .all(|reservation| { state.submit_start(*reservation) })
        );
    }

    #[test]
    fn stale_and_duplicate_completion_are_ignored() {
        let mut state = supported_state();
        let reservation = make_readback_pending(&mut state);
        let stale = GpuTimingReservation {
            slot: reservation.slot,
            token: reservation.token.wrapping_sub(1),
        };

        assert_eq!(
            state.complete_callback(stale, GpuTimingMapping::Values { start: 1, end: 2 }, 1.0),
            GpuTimingCallbackDisposition::Ignored
        );
        assert_eq!(
            state.complete_callback(
                reservation,
                GpuTimingMapping::Values { start: 1, end: 2 },
                1.0
            ),
            GpuTimingCallbackDisposition::Ready
        );
        assert_eq!(
            state.complete_callback(
                reservation,
                GpuTimingMapping::Values { start: 2, end: 3 },
                1.0
            ),
            GpuTimingCallbackDisposition::Ignored
        );
    }

    #[test]
    fn mapping_and_conversion_outcomes_are_terminal() {
        let mut state = supported_state();
        let reservation = make_readback_pending(&mut state);
        assert_eq!(
            state.complete_callback(reservation, GpuTimingMapping::Failed, 1.0),
            GpuTimingCallbackDisposition::Ready
        );
        let terminal = state
            .prepare_delivery(reservation)
            .expect("terminal result");
        assert_eq!(
            terminal.outcome,
            FrameGpuTimingOutcome::unavailable(FrameGpuTimingUnavailableReason::MappingFailed)
        );
        assert!(state.finish_delivery(reservation));

        let reservation = make_readback_pending(&mut state);
        assert_eq!(
            state.complete_callback(
                reservation,
                GpuTimingMapping::Values { start: 1, end: 2 },
                0.0,
            ),
            GpuTimingCallbackDisposition::Ready
        );
        let terminal = state
            .prepare_delivery(reservation)
            .expect("terminal result");
        assert_eq!(
            terminal.outcome,
            FrameGpuTimingOutcome::unavailable(FrameGpuTimingUnavailableReason::ConversionFailed)
        );
    }

    #[test]
    fn timestamp_conversion_uses_wrapping_subtraction() {
        assert_eq!(
            convert_timestamp_difference(u64::MAX - 1, 3, 2.0),
            FrameGpuTimingOutcome::available(Duration::from_nanos(10))
        );
    }

    #[test]
    fn timestamp_conversion_rejects_invalid_periods_and_overflow() {
        for period in [f32::NAN, f32::INFINITY, -1.0, 0.0] {
            assert_eq!(
                convert_timestamp_difference(0, 1, period),
                FrameGpuTimingOutcome::unavailable(
                    FrameGpuTimingUnavailableReason::ConversionFailed
                )
            );
        }
        assert_eq!(
            convert_timestamp_difference(0, u64::MAX, f32::MAX),
            FrameGpuTimingOutcome::unavailable(FrameGpuTimingUnavailableReason::ConversionFailed)
        );
    }

    #[test]
    fn delivery_is_exactly_once_and_the_slot_is_reusable() {
        let mut state = supported_state();
        let reservation = make_readback_pending(&mut state);
        assert_eq!(
            state.complete_callback(
                reservation,
                GpuTimingMapping::Values { start: 1, end: 4 },
                1.0,
            ),
            GpuTimingCallbackDisposition::Ready
        );
        assert!(state.prepare_delivery(reservation).is_some());
        assert!(state.prepare_delivery(reservation).is_none());
        assert!(state.finish_delivery(reservation));
        let reused = reserved(&mut state);
        assert_eq!(reused.slot, reservation.slot);
        assert_ne!(reused.token, reservation.token);
    }

    #[test]
    fn failed_or_vetoed_frame_cancels_without_a_terminal_sample() {
        let mut state = supported_state();
        let reserved_frame = reserved(&mut state);
        assert_eq!(
            state.cancel(reserved_frame),
            GpuTimingCancelAction::Recycled
        );

        let submitted_frame = reserved(&mut state);
        assert!(state.submit_start(submitted_frame));
        assert_eq!(
            state.cancel(submitted_frame),
            GpuTimingCancelAction::AwaitCallback
        );
        assert_eq!(
            state.complete_callback(
                submitted_frame,
                GpuTimingMapping::Values { start: 1, end: 2 },
                1.0,
            ),
            GpuTimingCallbackDisposition::Recycled
        );
        assert!(state.prepare_delivery(submitted_frame).is_none());
    }
}
