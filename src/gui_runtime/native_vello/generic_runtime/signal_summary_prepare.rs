//! Bounded UI-owned preparation of raw signal summary pyramids.
//!
//! The parent native runner owns this broker behind `Rc<RefCell<_>>`,
//! calls dispatch outside that borrow,
//! and reconciles current paint plans after capacity changes.

use super::{adapter::NativeAdapterGeneration, runner_state::NativeTargetGeneration};
use crate::{
    gui::repaint::RepaintSignal,
    runtime::{GpuSignalSummary, GpuSurfaceContent},
};
use std::hash::{Hash, Hasher};
use std::{
    collections::{HashMap, VecDeque},
    panic::{AssertUnwindSafe, catch_unwind},
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU64, Ordering},
        mpsc::{Receiver, SyncSender, TryRecvError, sync_channel},
    },
};
use winit::window::WindowId;

const MAX_ACTIVE: usize = 2;
const MAX_QUEUED: usize = 8;
const MAX_SOURCES: usize = 64;
const MAX_TARGETS: usize = 128;
const MAX_LOGICAL_BYTES: usize = 256 * 1024 * 1024;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct SummaryTargetId {
    serial: u64,
    window: WindowId,
    adapter_generation: NativeAdapterGeneration,
    target_generation: NativeTargetGeneration,
    surface_key: u64,
}

impl SummaryTargetId {
    pub(super) fn new(
        window: WindowId,
        adapter_generation: NativeAdapterGeneration,
        target_generation: NativeTargetGeneration,
        surface_key: u64,
    ) -> Option<Self> {
        static NEXT_SERIAL: AtomicU64 = AtomicU64::new(1);
        let serial = NEXT_SERIAL
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |serial| {
                serial.checked_add(1)
            })
            .ok()?;
        Some(Self {
            serial,
            window,
            adapter_generation,
            target_generation,
            surface_key,
        })
    }

    pub(super) const fn window(&self) -> WindowId {
        self.window
    }
    pub(super) const fn adapter_generation(&self) -> NativeAdapterGeneration {
        self.adapter_generation
    }
    pub(super) const fn target_generation(&self) -> NativeTargetGeneration {
        self.target_generation
    }
    pub(super) const fn surface_key(&self) -> u64 {
        self.surface_key
    }
}

impl Hash for SummaryTargetId {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.serial.hash(state);
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
struct SourceKey {
    allocation: usize,
    sample_len: usize,
    revision: u64,
    declared_frames: usize,
    declared_bands: usize,
    effective_frames: usize,
    effective_bands: usize,
}

#[derive(Clone)]
pub(super) struct SummaryRequest {
    key: SourceKey,
    samples: Arc<[f32]>,
}

impl SummaryRequest {
    pub(super) fn from_raw_surface(content: &GpuSurfaceContent, revision: u64) -> Option<Self> {
        let GpuSurfaceContent::SignalBands {
            frames,
            band_count,
            samples,
            ..
        } = content
        else {
            return None;
        };
        if *band_count == 0 || !content.is_renderable() {
            return None;
        }
        Some(Self::new(
            Arc::clone(samples),
            *frames,
            *band_count,
            revision,
        ))
    }

    pub(super) fn new(
        samples: Arc<[f32]>,
        frames: usize,
        band_count: usize,
        revision: u64,
    ) -> Self {
        let effective_bands = band_count.max(1);
        let effective_frames = frames.min(samples.len() / effective_bands);
        Self {
            key: SourceKey {
                allocation: Arc::as_ptr(&samples) as *const f32 as usize,
                sample_len: samples.len(),
                revision,
                declared_frames: frames,
                declared_bands: band_count,
                effective_frames,
                effective_bands,
            },
            samples,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum SummaryRequestState {
    Ready,
    Pending,
    WaitingAdmission,
    Unavailable,
}

/// Kept private so a renderer can only retain the source, summary, and lease together.
#[derive(Clone)]
pub(super) struct PreparedSummary {
    key: SourceKey,
    _source: Arc<[f32]>,
    summary: Arc<GpuSignalSummary>,
    _lease: SummaryRetentionLease,
}

impl PreparedSummary {
    #[cfg(test)]
    pub(super) fn source(&self) -> &Arc<[f32]> {
        &self._source
    }
    pub(super) fn summary(&self) -> &Arc<GpuSignalSummary> {
        &self.summary
    }
    pub(super) fn matches_raw_surface(&self, content: &GpuSurfaceContent, revision: u64) -> bool {
        SummaryRequest::from_raw_surface(content, revision)
            .is_some_and(|request| request.key == self.key)
    }
}

#[derive(Clone)]
struct SummaryRetentionLease(Arc<RetentionToken>);

struct RetentionToken {
    wake: Arc<dyn RepaintSignal>,
    retired: AtomicBool,
}

impl Drop for SummaryRetentionLease {
    fn drop(&mut self) {
        if self.0.retired.load(Ordering::Acquire) {
            self.0.wake.request_repaint();
        }
    }
}

enum EntryState {
    Queued,
    Active {
        id: u64,
        cancelled: Arc<AtomicBool>,
        retired: bool,
    },
    Ready {
        summary: Arc<GpuSignalSummary>,
        token: Arc<RetentionToken>,
    },
    Failed,
    Retired {
        summary: Option<Arc<GpuSignalSummary>>,
        token: Option<Arc<RetentionToken>>,
    },
}

struct SourceEntry {
    samples: Arc<[f32]>,
    bytes: usize,
    interests: usize,
    state: EntryState,
}

enum Interest {
    Source(SourceKey),
    Waiting,
}

enum CompletionState {
    Ready(Arc<GpuSignalSummary>),
    Cancelled,
    Failed,
}

struct Completion {
    id: u64,
    key: SourceKey,
    state: CompletionState,
}

pub(super) struct SummaryDispatch {
    id: u64,
    key: SourceKey,
    samples: Arc<[f32]>,
    frames: usize,
    bands: usize,
    cancelled: Arc<AtomicBool>,
    sender: SyncSender<Completion>,
    wake: Arc<dyn RepaintSignal>,
}

impl SummaryDispatch {
    pub(super) const fn id(&self) -> u64 {
        self.id
    }

    /// This consumes the reserved completion slot and emits exactly one terminal record.
    pub(super) fn run(self) {
        let started = std::time::Instant::now();
        let state = match catch_unwind(AssertUnwindSafe(|| {
            GpuSignalSummary::from_interleaved_samples_cancellable(
                &self.samples,
                self.frames,
                self.bands,
                || self.cancelled.load(Ordering::Acquire),
            )
        })) {
            Ok(Some(summary)) => CompletionState::Ready(Arc::new(summary)),
            Ok(None) => CompletionState::Cancelled,
            Err(_) => CompletionState::Failed,
        };
        if tracing::enabled!(target: "radiant::signal_summary_prepare", tracing::Level::DEBUG) {
            let (result, ready_summary_bytes) = match &state {
                CompletionState::Ready(summary) => ("ready", logical_ready_summary_bytes(summary)),
                CompletionState::Cancelled => ("cancelled", 0),
                CompletionState::Failed => ("failed", 0),
            };
            tracing::debug!(target: "radiant::signal_summary_prepare",
                job = self.id, result, preparation_elapsed_us = started.elapsed().as_micros() as u64,
                source_bytes = self.samples.len().saturating_mul(std::mem::size_of::<f32>()),
                ready_summary_bytes,
                "raw signal worker terminal");
        }
        let _ = self.sender.send(Completion {
            id: self.id,
            key: self.key,
            state,
        });
        self.wake.request_repaint();
    }
}

pub(super) struct SummaryBroker {
    wake: Arc<dyn RepaintSignal>,
    sender: SyncSender<Completion>,
    receiver: Receiver<Completion>,
    sources: HashMap<SourceKey, SourceEntry>,
    interests: HashMap<SummaryTargetId, Interest>,
    queue: VecDeque<SourceKey>,
    next_job: u64,
    active: usize,
    bytes: usize,
    limits: Limits,
}

#[derive(Clone, Copy)]
struct Limits {
    active: usize,
    queued: usize,
    sources: usize,
    targets: usize,
    bytes: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct SummaryCapacityStatus {
    pub(super) active: usize,
    pub(super) queued: usize,
    pub(super) sources: usize,
    pub(super) interests: usize,
    pub(super) logical_bytes: usize,
}

impl Limits {
    const fn production() -> Self {
        Self {
            active: MAX_ACTIVE,
            queued: MAX_QUEUED,
            sources: MAX_SOURCES,
            targets: MAX_TARGETS,
            bytes: MAX_LOGICAL_BYTES,
        }
    }
}

impl SummaryBroker {
    pub(super) fn new(wake: Arc<dyn RepaintSignal>) -> Self {
        Self::with_limits(wake, Limits::production())
    }

    #[cfg(test)]
    pub(super) fn with_byte_limit_for_test(byte_limit: usize) -> Self {
        struct NoopWake;
        impl RepaintSignal for NoopWake {
            fn request_repaint(&self) {}
        }
        Self::with_limits(
            Arc::new(NoopWake),
            Limits {
                bytes: byte_limit,
                ..Limits::production()
            },
        )
    }

    fn with_limits(wake: Arc<dyn RepaintSignal>, limits: Limits) -> Self {
        let (sender, receiver) = sync_channel(limits.active);
        Self {
            wake,
            sender,
            receiver,
            sources: HashMap::new(),
            interests: HashMap::new(),
            queue: VecDeque::new(),
            next_job: 1,
            active: 0,
            bytes: 0,
            limits,
        }
    }

    pub(super) fn request(
        &mut self,
        target: SummaryTargetId,
        request: SummaryRequest,
    ) -> SummaryRequestState {
        if matches!(self.interests.get(&target), Some(Interest::Source(key)) if *key == request.key)
        {
            return match self.sources.get(&request.key).map(|entry| &entry.state) {
                Some(EntryState::Ready { .. }) => SummaryRequestState::Ready,
                Some(EntryState::Failed) => SummaryRequestState::Unavailable,
                Some(_) => SummaryRequestState::Pending,
                None => SummaryRequestState::WaitingAdmission,
            };
        }
        self.release_target(target);
        if !self.interests.contains_key(&target) && self.interests.len() >= self.limits.targets {
            return SummaryRequestState::Unavailable;
        }
        let key = request.key;
        if let Some(entry) = self.sources.get_mut(&key) {
            entry.interests += 1;
            self.interests.insert(target, Interest::Source(key));
            let state = std::mem::replace(
                &mut entry.state,
                EntryState::Retired {
                    summary: None,
                    token: None,
                },
            );
            return match state {
                EntryState::Ready { summary, token } => {
                    entry.state = EntryState::Ready { summary, token };
                    SummaryRequestState::Ready
                }
                EntryState::Failed => {
                    entry.state = EntryState::Failed;
                    SummaryRequestState::Unavailable
                }
                EntryState::Retired {
                    summary: Some(summary),
                    token: Some(token),
                } => {
                    token.retired.store(false, Ordering::Release);
                    entry.state = EntryState::Ready { summary, token };
                    SummaryRequestState::Ready
                }
                EntryState::Retired { .. } if self.queue.len() < self.limits.queued => {
                    entry.state = EntryState::Queued;
                    self.queue.push_back(key);
                    SummaryRequestState::Pending
                }
                EntryState::Retired { .. } => {
                    entry.interests = entry.interests.saturating_sub(1);
                    self.interests.insert(target, Interest::Waiting);
                    // Keep the broker owner until off-redraw maintenance releases it.
                    entry.state = EntryState::Retired {
                        summary: None,
                        token: None,
                    };
                    SummaryRequestState::WaitingAdmission
                }
                state => {
                    entry.state = state;
                    SummaryRequestState::Pending
                }
            };
        }
        let Some(entry_bytes) = summary_bytes(&request) else {
            return SummaryRequestState::Unavailable;
        };
        if entry_bytes > self.limits.bytes {
            return SummaryRequestState::Unavailable;
        }
        if self.sources.len() >= self.limits.sources
            || self
                .bytes
                .checked_add(entry_bytes)
                .is_none_or(|bytes| bytes > self.limits.bytes)
            || self.queue.len() >= self.limits.queued
        {
            self.interests.insert(target, Interest::Waiting);
            return SummaryRequestState::WaitingAdmission;
        }
        self.bytes += entry_bytes;
        self.sources.insert(
            key,
            SourceEntry {
                samples: request.samples,
                bytes: entry_bytes,
                interests: 1,
                state: EntryState::Queued,
            },
        );
        self.queue.push_back(key);
        self.interests.insert(target, Interest::Source(key));
        SummaryRequestState::Pending
    }

    pub(super) fn prepared(&self, target: SummaryTargetId) -> Option<PreparedSummary> {
        let Interest::Source(key) = self.interests.get(&target)? else {
            return None;
        };
        let entry = self.sources.get(key)?;
        let EntryState::Ready { summary, token } = &entry.state else {
            return None;
        };
        Some(PreparedSummary {
            key: *key,
            _source: Arc::clone(&entry.samples),
            summary: Arc::clone(summary),
            _lease: SummaryRetentionLease(Arc::clone(token)),
        })
    }

    pub(super) fn take_dispatch(&mut self) -> Option<SummaryDispatch> {
        if self.active >= self.limits.active {
            return None;
        }
        while let Some(key) = self.queue.pop_front() {
            let Some(entry) = self.sources.get_mut(&key) else {
                continue;
            };
            if entry.interests == 0 || !matches!(entry.state, EntryState::Queued) {
                continue;
            }
            let Some(next_job) = self.next_job.checked_add(1) else {
                entry.state = EntryState::Failed;
                continue;
            };
            let id = self.next_job;
            self.next_job = next_job;
            let cancelled = Arc::new(AtomicBool::new(false));
            entry.state = EntryState::Active {
                id,
                cancelled: Arc::clone(&cancelled),
                retired: false,
            };
            self.active += 1;
            return Some(SummaryDispatch {
                id,
                key,
                samples: Arc::clone(&entry.samples),
                frames: key.effective_frames,
                bands: key.effective_bands,
                cancelled,
                sender: self.sender.clone(),
                wake: Arc::clone(&self.wake),
            });
        }
        None
    }

    pub(super) fn waiting_targets(&self) -> impl Iterator<Item = SummaryTargetId> + '_ {
        self.interests.iter().filter_map(|(target, interest)| {
            matches!(interest, Interest::Waiting).then_some(*target)
        })
    }

    pub(super) fn request_pump(&self) {
        self.wake.request_repaint();
    }

    fn mark_waiting_for(&mut self, key: SourceKey) {
        for interest in self.interests.values_mut() {
            if matches!(interest, Interest::Source(source) if *source == key) {
                *interest = Interest::Waiting;
            }
        }
        if let Some(entry) = self.sources.get_mut(&key) {
            entry.interests = 0;
        }
    }

    pub(super) fn capacity_status(&self) -> SummaryCapacityStatus {
        SummaryCapacityStatus {
            active: self.active,
            queued: self.queue.len(),
            sources: self.sources.len(),
            interests: self.interests.len(),
            logical_bytes: self.bytes,
        }
    }

    #[cfg(test)]
    pub(super) fn prepare_for_test(
        content: &GpuSurfaceContent,
        revision: u64,
    ) -> (Self, PreparedSummary) {
        struct NoopWake;
        impl RepaintSignal for NoopWake {
            fn request_repaint(&self) {}
        }

        let mut broker = Self::new(Arc::new(NoopWake));
        let target = SummaryTargetId::new(
            WindowId::dummy(),
            NativeAdapterGeneration::from_test_serial(1),
            NativeTargetGeneration::from_test_serial(1),
            1,
        )
        .expect("test target serial");
        let request = SummaryRequest::from_raw_surface(content, revision)
            .expect("renderable raw signal content");
        assert_eq!(
            broker.request(target, request),
            SummaryRequestState::Pending
        );
        broker
            .take_dispatch()
            .expect("admitted test dispatch")
            .run();
        broker.drain_completions();
        let prepared = broker.prepared(target).expect("prepared test summary");
        (broker, prepared)
    }

    /// Call only when the host rejected the returned dispatch closure.
    pub(super) fn reject_dispatch(&mut self, id: u64) {
        for entry in self.sources.values_mut() {
            let retired = match &entry.state {
                EntryState::Active {
                    id: active_id,
                    retired,
                    ..
                } if *active_id == id => *retired,
                _ => continue,
            };
            if retired {
                entry.state = EntryState::Retired {
                    summary: None,
                    token: None,
                };
            } else {
                entry.state = EntryState::Failed;
            }
            self.active = self.active.saturating_sub(1);
            break;
        }
    }

    /// Drain after the parent clears its pending wake flag. Returns current targets to re-prime.
    pub(super) fn drain_completions(&mut self) -> Vec<SummaryTargetId> {
        let mut notify = Vec::new();
        loop {
            let completion = match self.receiver.try_recv() {
                Ok(completion) => completion,
                Err(TryRecvError::Empty | TryRecvError::Disconnected) => break,
            };
            let Some(entry) = self.sources.get_mut(&completion.key) else {
                continue;
            };
            let is_current =
                matches!(entry.state, EntryState::Active { id, .. } if id == completion.id);
            if !is_current {
                continue;
            }
            self.active = self.active.saturating_sub(1);
            let ready = matches!(&completion.state, CompletionState::Ready(_));
            let cancelled_with_interest =
                matches!(&completion.state, CompletionState::Cancelled) && entry.interests != 0;
            let requeue = cancelled_with_interest && self.queue.len() < self.limits.queued;
            entry.state = match completion.state {
                CompletionState::Ready(summary) if entry.interests != 0 => EntryState::Ready {
                    summary,
                    token: Arc::new(RetentionToken {
                        wake: Arc::clone(&self.wake),
                        retired: AtomicBool::new(false),
                    }),
                },
                CompletionState::Ready(summary) => EntryState::Retired {
                    summary: Some(summary),
                    token: Some(Arc::new(RetentionToken {
                        wake: Arc::clone(&self.wake),
                        retired: AtomicBool::new(true),
                    })),
                },
                CompletionState::Cancelled if requeue => EntryState::Queued,
                CompletionState::Cancelled => EntryState::Retired {
                    summary: None,
                    token: None,
                },
                CompletionState::Failed if entry.interests != 0 => EntryState::Failed,
                CompletionState::Failed => EntryState::Retired {
                    summary: None,
                    token: None,
                },
            };
            if ready {
                notify.extend(self.interests.iter().filter_map(|(target, interest)| {
                    matches!(interest, Interest::Source(key) if *key == completion.key)
                        .then_some(*target)
                }));
            }
            if requeue {
                self.queue.push_back(completion.key);
            } else if cancelled_with_interest {
                self.mark_waiting_for(completion.key);
            }
        }
        notify
    }

    pub(super) fn release_target(&mut self, target: SummaryTargetId) {
        let Some(interest) = self.interests.remove(&target) else {
            return;
        };
        let Interest::Source(key) = interest else {
            return;
        };
        let Some(entry) = self.sources.get_mut(&key) else {
            return;
        };
        entry.interests = entry.interests.saturating_sub(1);
        if entry.interests != 0 {
            return;
        }
        // A queued/failed source can release capacity without a worker terminal.
        // The numeric capacity snapshot may not change until maintenance drops it.
        self.wake.request_repaint();
        entry.state = match std::mem::replace(
            &mut entry.state,
            EntryState::Retired {
                summary: None,
                token: None,
            },
        ) {
            EntryState::Active { id, cancelled, .. } => {
                cancelled.store(true, Ordering::Release);
                EntryState::Active {
                    id,
                    cancelled,
                    retired: true,
                }
            }
            EntryState::Ready { summary, token } => {
                token.retired.store(true, Ordering::Release);
                EntryState::Retired {
                    summary: Some(summary),
                    token: Some(token),
                }
            }
            EntryState::Queued => {
                self.queue.retain(|queued| *queued != key);
                EntryState::Retired {
                    summary: None,
                    token: None,
                }
            }
            EntryState::Failed | EntryState::Retired { .. } => EntryState::Retired {
                summary: None,
                token: None,
            },
        };
    }

    /// Explicit non-redraw maintenance drops only retired payloads with no external lease.
    pub(super) fn maintain_retired(&mut self) {
        let retired: Vec<_> = self
            .sources
            .iter()
            .filter_map(|(key, entry)| match &entry.state {
                EntryState::Retired {
                    token: Some(token), ..
                } if entry.interests == 0 && Arc::strong_count(token) == 1 => Some(*key),
                EntryState::Retired { token: None, .. } if entry.interests == 0 => Some(*key),
                _ => None,
            })
            .collect();
        for key in retired {
            if let Some(entry) = self.sources.remove(&key) {
                self.bytes = self.bytes.saturating_sub(entry.bytes);
            }
        }
        if tracing::enabled!(target: "radiant::signal_summary_prepare", tracing::Level::DEBUG) {
            let mut retained_source_bytes = 0usize;
            let mut retained_ready_summary_bytes = 0usize;
            for entry in self.sources.values() {
                retained_source_bytes = retained_source_bytes.saturating_add(
                    entry
                        .samples
                        .len()
                        .saturating_mul(std::mem::size_of::<f32>()),
                );
                let summary = match &entry.state {
                    EntryState::Ready { summary, .. }
                    | EntryState::Retired {
                        summary: Some(summary),
                        ..
                    } => Some(summary),
                    _ => None,
                };
                if let Some(summary) = summary {
                    retained_ready_summary_bytes = retained_ready_summary_bytes
                        .saturating_add(logical_ready_summary_bytes(summary));
                }
            }
            tracing::debug!(target: "radiant::signal_summary_prepare",
                active_jobs = self.active, queued_jobs = self.queue.len(), retained_sources = self.sources.len(),
                reserved_logical_bytes = self.bytes, retained_source_bytes, retained_ready_summary_bytes,
                "raw signal preparation ownership");
        }
    }
}

impl Drop for SummaryBroker {
    fn drop(&mut self) {
        for entry in self.sources.values() {
            if let EntryState::Active { cancelled, .. } = &entry.state {
                cancelled.store(true, Ordering::Release);
            }
        }
    }
}

fn logical_ready_summary_bytes(summary: &GpuSignalSummary) -> usize {
    summary.levels.iter().fold(0usize, |bytes, level| {
        bytes.saturating_add(
            level
                .buckets
                .len()
                .saturating_mul(std::mem::size_of::<crate::runtime::GpuSignalSummaryBucket>()),
        )
    })
}

fn summary_bytes(request: &SummaryRequest) -> Option<usize> {
    let source = request
        .samples
        .len()
        .checked_mul(std::mem::size_of::<f32>())?;
    let mut bucket_frames = 1usize;
    let mut summary = 0usize;
    while bucket_frames <= request.key.effective_frames.max(1) {
        let bucket_count = request.key.effective_frames.div_ceil(bucket_frames).max(1);
        summary = summary.checked_add(
            bucket_count
                .checked_mul(request.key.effective_bands)?
                .checked_mul(std::mem::size_of::<crate::runtime::GpuSignalSummaryBucket>())?,
        )?;
        if bucket_frames >= request.key.effective_frames.max(1) {
            break;
        }
        bucket_frames = bucket_frames.saturating_mul(2).max(bucket_frames + 1);
    }
    source.checked_add(summary)
}

#[cfg(test)]
#[path = "signal_summary_prepare/tests.rs"]
mod tests;
