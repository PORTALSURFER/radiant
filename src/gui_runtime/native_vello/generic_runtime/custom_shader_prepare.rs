//! Bounded, transient preparation of custom shader pipelines.
//!
//! This cache deliberately owns only candidates waiting to be consumed by a
//! renderer transaction. Installed pipelines remain owned and bounded by the
//! renderer's physical cache, so a saturated renderer can still stage its
//! replacement without this broker retaining old installed objects.

use super::gpu_surface::custom_shader::pipeline::{
    prepare_custom_shader_pipeline, CustomShaderPreparationFailure,
    OwnedCustomShaderPipelineRequest,
};
use super::gpu_surface::gpu_surface_types::{
    CustomShaderPipeline, CustomShaderPipelineIdentity, CustomShaderPipelineKey,
};
use super::{adapter::NativeAdapterGeneration, runner_state::NativeTargetGeneration};
use crate::gui::repaint::RepaintSignal;
use std::{
    collections::{HashMap, VecDeque},
    hash::{Hash, Hasher},
    panic::{catch_unwind, AssertUnwindSafe},
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        mpsc::{sync_channel, Receiver, SyncSender},
        Arc,
    },
};
use vello::wgpu;
use winit::window::WindowId;

const MAX_ACTIVE: usize = 2;
const MAX_QUEUED: usize = 8;
const MAX_ENTRIES: usize = 256;
const MAX_INTERESTS: usize = 1024;
const MAX_KEY_TEXT_BYTES: usize = 1024 * 1024;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct CustomShaderTargetId {
    serial: u64,
    window: WindowId,
    adapter_generation: NativeAdapterGeneration,
    target_generation: NativeTargetGeneration,
    surface_key: u64,
}
impl CustomShaderTargetId {
    pub(super) fn new(
        window: WindowId,
        adapter_generation: NativeAdapterGeneration,
        target_generation: NativeTargetGeneration,
        surface_key: u64,
    ) -> Option<Self> {
        static NEXT: AtomicU64 = AtomicU64::new(1);
        let serial = NEXT
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |value| {
                value.checked_add(1)
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
    pub(super) const fn window(self) -> WindowId {
        self.window
    }
    pub(super) const fn adapter_generation(self) -> NativeAdapterGeneration {
        self.adapter_generation
    }
    pub(super) const fn target_generation(self) -> NativeTargetGeneration {
        self.target_generation
    }
    pub(super) const fn surface_key(self) -> u64 {
        self.surface_key
    }
}
impl Hash for CustomShaderTargetId {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.serial.hash(state);
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct PreparationKey {
    device_identity: usize,
    adapter_generation: NativeAdapterGeneration,
    target_format: wgpu::TextureFormat,
    pipeline: CustomShaderPipelineKey,
}
// Generations deliberately are not generally hashable. Equality still fences
// them; hashing the immutable device/format/key fields only is sufficient.
impl Hash for PreparationKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.device_identity.hash(state);
        self.target_format.hash(state);
        self.pipeline.hash(state);
    }
}

#[derive(Clone)]
pub(super) struct CustomShaderPreparationRequest {
    key: PreparationKey,
    request: OwnedCustomShaderPipelineRequest,
}
impl CustomShaderPreparationRequest {
    /// The caller supplies the UI-captured identity. It is intentionally not
    /// recalculated from the worker's cloned `wgpu::Device`.
    pub(super) fn new(
        device: wgpu::Device,
        device_identity: usize,
        adapter_generation: NativeAdapterGeneration,
        target_format: wgpu::TextureFormat,
        pipeline: CustomShaderPipelineKey,
    ) -> Self {
        Self {
            key: PreparationKey {
                device_identity,
                adapter_generation,
                target_format,
                pipeline: pipeline.clone(),
            },
            request: OwnedCustomShaderPipelineRequest {
                device,
                device_identity,
                target_format,
                key: pipeline,
            },
        }
    }
    pub(super) fn identity(&self) -> CustomShaderPipelineIdentity {
        CustomShaderPipelineIdentity {
            device: self.key.device_identity,
            format: self.key.target_format,
            key: self.key.pipeline.clone(),
        }
    }
    pub(super) const fn adapter_generation(&self) -> NativeAdapterGeneration {
        self.key.adapter_generation
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum CustomShaderPreparationState {
    Ready,
    Pending,
    WaitingAdmission,
    Unavailable,
}

#[derive(Clone)]
pub(super) struct PreparedCustomShaderPipeline {
    key: PreparationKey,
    /// Keeps the device owner alive while this transient candidate lease exists.
    _device: wgpu::Device,
    pipeline: CustomShaderPipeline,
    _lease: CandidateLease,
}
impl PreparedCustomShaderPipeline {
    pub(super) fn pipeline(&self) -> &CustomShaderPipeline {
        &self.pipeline
    }
    pub(super) fn matches(&self, request: &CustomShaderPreparationRequest) -> bool {
        self.key == request.key
    }
    pub(super) fn identity(&self) -> CustomShaderPipelineIdentity {
        CustomShaderPipelineIdentity {
            device: self.key.device_identity,
            format: self.key.target_format,
            key: self.key.pipeline.clone(),
        }
    }
    pub(super) const fn adapter_generation(&self) -> NativeAdapterGeneration {
        self.key.adapter_generation
    }
}

#[derive(Clone)]
struct CandidateLease(Option<Arc<RetentionToken>>);
struct RetentionToken {
    wake: Arc<dyn RepaintSignal>,
    retired: Arc<AtomicBool>,
}
impl Drop for CandidateLease {
    fn drop(&mut self) {
        let Some(token) = self.0.take() else {
            return;
        };
        let retired = Arc::clone(&token.retired);
        let wake = Arc::clone(&token.wake);
        drop(token);
        if retired.load(Ordering::Acquire) {
            wake.request_repaint();
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
        device: wgpu::Device,
        pipeline: CustomShaderPipeline,
        token: Arc<RetentionToken>,
    },
    Failed(CustomShaderPreparationFailure),
    Retired {
        device: Option<wgpu::Device>,
        pipeline: Option<CustomShaderPipeline>,
        token: Option<Arc<RetentionToken>>,
    },
}
struct Entry {
    request: OwnedCustomShaderPipelineRequest,
    interests: usize,
    state: EntryState,
}
enum Interest {
    Entry(PreparationKey),
    Waiting,
}
enum Terminal {
    Ready(CustomShaderPipeline),
    Failed(CustomShaderPreparationFailure),
}
struct Completion {
    id: u64,
    key: PreparationKey,
    terminal: Terminal,
}

pub(super) struct CustomShaderPreparationDispatch {
    id: u64,
    key: PreparationKey,
    request: OwnedCustomShaderPipelineRequest,
    cancelled: Arc<AtomicBool>,
    sender: SyncSender<Completion>,
    wake: Arc<dyn RepaintSignal>,
}
impl CustomShaderPreparationDispatch {
    pub(super) const fn id(&self) -> u64 {
        self.id
    }
    pub(super) fn run(self) {
        let cancelled = || self.cancelled.load(Ordering::Acquire);
        let terminal = match catch_unwind(AssertUnwindSafe(|| {
            prepare_custom_shader_pipeline(self.request, cancelled)
        })) {
            Ok(Ok(pipeline)) => Terminal::Ready(pipeline),
            Ok(Err(error)) => Terminal::Failed(error),
            Err(_) => Terminal::Failed(CustomShaderPreparationFailure::Panicked),
        };
        let _ = self.sender.send(Completion {
            id: self.id,
            key: self.key,
            terminal,
        });
        self.wake.request_repaint();
    }
}

pub(super) struct CustomShaderPreparationBroker {
    wake: Arc<dyn RepaintSignal>,
    sender: SyncSender<Completion>,
    receiver: Receiver<Completion>,
    entries: HashMap<PreparationKey, Entry>,
    interests: HashMap<CustomShaderTargetId, Interest>,
    queue: VecDeque<PreparationKey>,
    active: usize,
    active_devices: HashMap<usize, usize>,
    text_bytes: usize,
    next_job: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct CustomShaderPreparationCapacityStatus {
    pub(super) active: usize,
    pub(super) queued: usize,
    pub(super) entries: usize,
    pub(super) interests: usize,
    pub(super) key_text_bytes: usize,
}
impl CustomShaderPreparationBroker {
    pub(super) fn new(wake: Arc<dyn RepaintSignal>) -> Self {
        let (sender, receiver) = sync_channel(MAX_ACTIVE);
        Self {
            wake,
            sender,
            receiver,
            entries: HashMap::new(),
            interests: HashMap::new(),
            queue: VecDeque::new(),
            active: 0,
            active_devices: HashMap::new(),
            text_bytes: 0,
            next_job: 1,
        }
    }
    pub(super) fn request(
        &mut self,
        target: CustomShaderTargetId,
        request: CustomShaderPreparationRequest,
    ) -> CustomShaderPreparationState {
        if matches!(self.interests.get(&target), Some(Interest::Entry(key)) if *key == request.key)
        {
            return self.state_for(&request.key);
        }
        self.release_target(target);
        if self.interests.len() >= MAX_INTERESTS {
            return CustomShaderPreparationState::Unavailable;
        }
        if let Some(entry) = self.entries.get_mut(&request.key) {
            let had_no_interest = entry.interests == 0;
            entry.interests += 1;
            self.interests
                .insert(target, Interest::Entry(request.key.clone()));
            let state = std::mem::replace(
                &mut entry.state,
                EntryState::Retired {
                    device: None,
                    pipeline: None,
                    token: None,
                },
            );
            return match state {
                EntryState::Ready {
                    device,
                    pipeline,
                    token,
                } => {
                    entry.state = EntryState::Ready {
                        device,
                        pipeline,
                        token,
                    };
                    CustomShaderPreparationState::Ready
                }
                EntryState::Failed(failure) if !had_no_interest => {
                    entry.state = EntryState::Failed(failure);
                    CustomShaderPreparationState::Unavailable
                }
                EntryState::Failed(_) => {
                    entry.state = EntryState::Queued;
                    self.queue.push_back(request.key);
                    CustomShaderPreparationState::Pending
                }
                EntryState::Retired {
                    device: Some(device),
                    pipeline: Some(pipeline),
                    token: Some(token),
                } => {
                    token.retired.store(false, Ordering::Release);
                    entry.state = EntryState::Ready {
                        device,
                        pipeline,
                        token,
                    };
                    CustomShaderPreparationState::Ready
                }
                EntryState::Retired { .. } => {
                    entry.state = EntryState::Queued;
                    self.queue.push_back(request.key);
                    CustomShaderPreparationState::Pending
                }
                _ => CustomShaderPreparationState::Pending,
            };
        }
        let bytes = request.key.pipeline.text_bytes();
        if bytes > MAX_KEY_TEXT_BYTES {
            return CustomShaderPreparationState::Unavailable;
        }
        if self.entries.len() >= MAX_ENTRIES
            || self.text_bytes.saturating_add(bytes) > MAX_KEY_TEXT_BYTES
            || self.queue.len() >= MAX_QUEUED
        {
            self.interests.insert(target, Interest::Waiting);
            return CustomShaderPreparationState::WaitingAdmission;
        }
        self.text_bytes += bytes;
        self.entries.insert(
            request.key.clone(),
            Entry {
                request: request.request,
                interests: 1,
                state: EntryState::Queued,
            },
        );
        self.queue.push_back(request.key.clone());
        self.interests.insert(target, Interest::Entry(request.key));
        CustomShaderPreparationState::Pending
    }
    fn state_for(&self, key: &PreparationKey) -> CustomShaderPreparationState {
        match self.entries.get(key).map(|entry| &entry.state) {
            Some(EntryState::Ready { .. }) => CustomShaderPreparationState::Ready,
            Some(EntryState::Failed(_)) => CustomShaderPreparationState::Unavailable,
            None => CustomShaderPreparationState::WaitingAdmission,
            _ => CustomShaderPreparationState::Pending,
        }
    }
    pub(super) fn failure(
        &self,
        target: CustomShaderTargetId,
    ) -> Option<CustomShaderPreparationFailure> {
        let Interest::Entry(key) = self.interests.get(&target)? else {
            return None;
        };
        match &self.entries.get(key)?.state {
            EntryState::Failed(failure) => Some(*failure),
            _ => None,
        }
    }
    pub(super) fn prepared(
        &self,
        target: CustomShaderTargetId,
    ) -> Option<PreparedCustomShaderPipeline> {
        let Interest::Entry(key) = self.interests.get(&target)? else {
            return None;
        };
        let EntryState::Ready {
            device,
            pipeline,
            token,
        } = &self.entries.get(key)?.state
        else {
            return None;
        };
        Some(PreparedCustomShaderPipeline {
            key: key.clone(),
            _device: device.clone(),
            pipeline: pipeline.clone(),
            _lease: CandidateLease(Some(Arc::clone(token))),
        })
    }
    /// Called only after a renderer transaction consumes a candidate. This
    /// releases transient broker capacity; installed cache ownership is separate.
    pub(super) fn consume_target(&mut self, target: CustomShaderTargetId) {
        self.release_target(target);
    }
    pub(super) fn take_dispatch(&mut self) -> Option<CustomShaderPreparationDispatch> {
        if self.active >= MAX_ACTIVE {
            return None;
        }
        let len = self.queue.len();
        for _ in 0..len {
            let key = self.queue.pop_front()?;
            let Some(entry) = self.entries.get_mut(&key) else {
                continue;
            };
            if !matches!(entry.state, EntryState::Queued) || entry.interests == 0 {
                continue;
            }
            if self
                .active_devices
                .get(&key.device_identity)
                .copied()
                .unwrap_or(0)
                != 0
            {
                self.queue.push_back(key);
                continue;
            }
            let id = self.next_job;
            self.next_job = self.next_job.checked_add(1)?;
            let cancelled = Arc::new(AtomicBool::new(false));
            entry.state = EntryState::Active {
                id,
                cancelled: Arc::clone(&cancelled),
                retired: false,
            };
            self.active += 1;
            *self.active_devices.entry(key.device_identity).or_default() += 1;
            return Some(CustomShaderPreparationDispatch {
                id,
                key,
                request: entry.request.clone(),
                cancelled,
                sender: self.sender.clone(),
                wake: Arc::clone(&self.wake),
            });
        }
        None
    }
    pub(super) fn waiting_targets(&self) -> impl Iterator<Item = CustomShaderTargetId> + '_ {
        self.interests.iter().filter_map(|(target, interest)| {
            matches!(interest, Interest::Waiting).then_some(*target)
        })
    }
    pub(super) fn request_pump(&self) {
        self.wake.request_repaint();
    }
    pub(super) fn capacity_status(&self) -> CustomShaderPreparationCapacityStatus {
        CustomShaderPreparationCapacityStatus {
            active: self.active,
            queued: self.queue.len(),
            entries: self.entries.len(),
            interests: self.interests.len(),
            key_text_bytes: self.text_bytes,
        }
    }
    pub(super) fn reject_dispatch(&mut self, id: u64) {
        self.finish_rejected(id);
    }
    fn finish_rejected(&mut self, id: u64) {
        let key = self.entries.iter().find_map(|(key, entry)| {
            matches!(entry.state, EntryState::Active { id: active, .. } if active == id)
                .then_some(key.clone())
        });
        if let Some(key) = key {
            self.finish_active(&key);
            if let Some(entry) = self.entries.get_mut(&key) {
                entry.state = if entry.interests == 0 {
                    EntryState::Retired {
                        device: None,
                        pipeline: None,
                        token: None,
                    }
                } else {
                    EntryState::Failed(CustomShaderPreparationFailure::Panicked)
                };
            }
        }
    }
    fn finish_active(&mut self, key: &PreparationKey) {
        self.active = self.active.saturating_sub(1);
        if let Some(count) = self.active_devices.get_mut(&key.device_identity) {
            *count = count.saturating_sub(1);
            if *count == 0 {
                self.active_devices.remove(&key.device_identity);
            }
        }
    }
    pub(super) fn drain_completions(&mut self) -> Vec<CustomShaderTargetId> {
        let mut notify = Vec::new();
        while let Ok(completion) = self.receiver.try_recv() {
            let current = matches!(self.entries.get(&completion.key).map(|entry| &entry.state), Some(EntryState::Active { id, .. }) if *id == completion.id);
            if !current {
                continue;
            }
            self.finish_active(&completion.key);
            let can_requeue = self.queue.len() < MAX_QUEUED;
            let cancelled_without_queue = matches!(
                &completion.terminal,
                Terminal::Failed(CustomShaderPreparationFailure::Cancelled)
            ) && !can_requeue;
            let Some(entry) = self.entries.get_mut(&completion.key) else {
                continue;
            };
            entry.state = match completion.terminal {
                Terminal::Ready(pipeline) if entry.interests != 0 => EntryState::Ready {
                    device: entry.request.device.clone(),
                    pipeline,
                    token: Arc::new(RetentionToken {
                        wake: Arc::clone(&self.wake),
                        retired: Arc::new(AtomicBool::new(false)),
                    }),
                },
                Terminal::Ready(pipeline) => EntryState::Retired {
                    device: Some(entry.request.device.clone()),
                    pipeline: Some(pipeline),
                    token: Some(Arc::new(RetentionToken {
                        wake: Arc::clone(&self.wake),
                        retired: Arc::new(AtomicBool::new(true)),
                    })),
                },
                Terminal::Failed(CustomShaderPreparationFailure::Cancelled)
                    if entry.interests != 0 && can_requeue =>
                {
                    EntryState::Queued
                }
                Terminal::Failed(CustomShaderPreparationFailure::Cancelled)
                    if entry.interests != 0 =>
                {
                    EntryState::Retired {
                        device: None,
                        pipeline: None,
                        token: None,
                    }
                }
                Terminal::Failed(failure) if entry.interests != 0 => EntryState::Failed(failure),
                Terminal::Failed(_) => EntryState::Retired {
                    device: None,
                    pipeline: None,
                    token: None,
                },
            };
            if matches!(entry.state, EntryState::Ready { .. }) {
                notify.extend(self.interests.iter().filter_map(|(target, interest)| {
                    matches!(interest, Interest::Entry(key) if *key == completion.key)
                        .then_some(*target)
                }));
            }
            if matches!(entry.state, EntryState::Queued) {
                self.queue.push_back(completion.key);
            }
            if cancelled_without_queue {
                for interest in self.interests.values_mut() {
                    if matches!(interest, Interest::Entry(key) if *key == completion.key) {
                        *interest = Interest::Waiting;
                    }
                }
                if let Some(entry) = self.entries.get_mut(&completion.key) {
                    entry.interests = 0;
                }
            }
        }
        notify
    }
    pub(super) fn release_target(&mut self, target: CustomShaderTargetId) {
        let Some(Interest::Entry(key)) = self.interests.remove(&target) else {
            return;
        };
        let Some(entry) = self.entries.get_mut(&key) else {
            return;
        };
        entry.interests = entry.interests.saturating_sub(1);
        if entry.interests != 0 {
            return;
        }
        self.wake.request_repaint();
        entry.state = match std::mem::replace(
            &mut entry.state,
            EntryState::Retired {
                device: None,
                pipeline: None,
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
            EntryState::Ready {
                device,
                pipeline,
                token,
            } => {
                token.retired.store(true, Ordering::Release);
                EntryState::Retired {
                    device: Some(device),
                    pipeline: Some(pipeline),
                    token: Some(token),
                }
            }
            EntryState::Queued => {
                self.queue.retain(|queued| *queued != key);
                EntryState::Retired {
                    device: None,
                    pipeline: None,
                    token: None,
                }
            }
            // A typed failure is sticky only while a target is interested in
            // this exact identity. Once the final target leaves it must be
            // removable, otherwise a later identity retry leaks an entry.
            EntryState::Failed(_) => EntryState::Retired {
                device: None,
                pipeline: None,
                token: None,
            },
            state => state,
        };
    }
    pub(super) fn maintain_retired(&mut self) {
        let retired: Vec<_> = self
            .entries
            .iter()
            .filter_map(|(key, entry)| match &entry.state {
                EntryState::Retired {
                    token: Some(token), ..
                } if entry.interests == 0 && Arc::strong_count(token) == 1 => Some(key.clone()),
                EntryState::Retired { token: None, .. } if entry.interests == 0 => {
                    Some(key.clone())
                }
                _ => None,
            })
            .collect();
        for key in retired {
            if self.entries.remove(&key).is_some() {
                self.text_bytes = self.text_bytes.saturating_sub(key.pipeline.text_bytes());
            }
        }
    }
}
impl Drop for CustomShaderPreparationBroker {
    fn drop(&mut self) {
        for entry in self.entries.values() {
            if let EntryState::Active { cancelled, .. } = &entry.state {
                cancelled.store(true, Ordering::Release);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicUsize;

    struct CountingWake(AtomicUsize);
    impl RepaintSignal for CountingWake {
        fn request_repaint(&self) {
            self.0.fetch_add(1, Ordering::Relaxed);
        }
    }

    #[test]
    fn target_serial_fences_equal_surface_epochs() {
        let first = CustomShaderTargetId::new(
            WindowId::dummy(),
            NativeAdapterGeneration::from_test_serial(7),
            NativeTargetGeneration::from_test_serial(9),
            11,
        )
        .expect("test serial");
        let replacement = CustomShaderTargetId::new(
            WindowId::dummy(),
            NativeAdapterGeneration::from_test_serial(7),
            NativeTargetGeneration::from_test_serial(9),
            11,
        )
        .expect("test serial");
        assert_ne!(first, replacement);
        assert_eq!(first.window(), WindowId::dummy());
        assert_eq!(first.surface_key(), 11);
    }

    #[test]
    fn preparation_key_keeps_adapter_generation_in_equality() {
        let pipeline = CustomShaderPipelineKey {
            shader_key: Arc::from("shader"),
            wgsl_source: Arc::from("@vertex fn v() {}"),
            vertex_entry_point: Arc::from("v"),
            fragment_entry_point: Arc::from("f"),
            has_uniform_payload: false,
            has_storage_payload: false,
            has_presentation_uniform_payload: false,
        };
        let first = PreparationKey {
            device_identity: 1,
            adapter_generation: NativeAdapterGeneration::from_test_serial(1),
            target_format: wgpu::TextureFormat::Rgba8Unorm,
            pipeline: pipeline.clone(),
        };
        let replacement = PreparationKey {
            adapter_generation: NativeAdapterGeneration::from_test_serial(2),
            ..first.clone()
        };
        assert_ne!(first, replacement);
        let mut entries = HashMap::new();
        entries.insert(first, 1usize);
        assert!(!entries.contains_key(&replacement));
    }

    #[test]
    fn last_retired_candidate_lease_wakes_after_its_token_is_released() {
        let wake = Arc::new(CountingWake(AtomicUsize::new(0)));
        let token = Arc::new(RetentionToken {
            wake: wake.clone(),
            retired: Arc::new(AtomicBool::new(true)),
        });
        let lease = CandidateLease(Some(token));
        drop(lease);
        assert_eq!(wake.0.load(Ordering::Relaxed), 1);
    }

    fn native_device() -> wgpu::Device {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::PRIMARY,
            ..wgpu::InstanceDescriptor::new_without_display_handle()
        });
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            compatible_surface: None,
            ..Default::default()
        }))
        .expect("shader preparation broker requires a native adapter");
        pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            label: Some("radiant_shader_preparation_broker_test"),
            required_features: wgpu::Features::empty(),
            required_limits: wgpu::Limits::default(),
            ..Default::default()
        }))
        .expect("shader preparation broker requires a native device")
        .0
    }

    fn request(device: &wgpu::Device, number: usize) -> CustomShaderPreparationRequest {
        let source = format!(
            "@vertex fn vertex_main(@builtin(vertex_index) i: u32) -> @builtin(position) vec4<f32> {{ let x = f32(i); return vec4<f32>(x, 0.0, 0.0, 1.0); }}\n@fragment fn fragment_main() -> @location(0) vec4<f32> {{ return vec4<f32>({number}.0, 0.0, 0.0, 1.0); }}"
        );
        CustomShaderPreparationRequest::new(
            device.clone(),
            71,
            NativeAdapterGeneration::from_test_serial(1),
            wgpu::TextureFormat::Rgba8Unorm,
            CustomShaderPipelineKey {
                shader_key: Arc::from(format!("shader-{number}")),
                wgsl_source: Arc::from(source),
                vertex_entry_point: Arc::from("vertex_main"),
                fragment_entry_point: Arc::from("fragment_main"),
                has_uniform_payload: false,
                has_storage_payload: false,
                has_presentation_uniform_payload: false,
            },
        )
    }

    fn target(number: u64) -> CustomShaderTargetId {
        CustomShaderTargetId::new(
            WindowId::dummy(),
            NativeAdapterGeneration::from_test_serial(1),
            NativeTargetGeneration::from_test_serial(1),
            number,
        )
        .expect("target serial")
    }

    #[test]
    #[ignore = "requires a native WGPU device"]
    fn native_broker_coalesces_bounds_rejects_cancels_and_retires() {
        let device = native_device();
        let wake = Arc::new(CountingWake(AtomicUsize::new(0)));
        let mut bounded = CustomShaderPreparationBroker::new(Arc::clone(&wake));
        let first_target = target(1);
        let first_request = request(&device, 1);
        assert_eq!(
            bounded.request(first_target, first_request.clone()),
            CustomShaderPreparationState::Pending
        );
        assert_eq!(
            bounded.request(first_target, first_request.clone()),
            CustomShaderPreparationState::Pending
        );
        assert_eq!(bounded.capacity_status().interests, 1);

        for number in 2..=8 {
            assert_eq!(
                bounded.request(target(number), request(&device, number as usize)),
                CustomShaderPreparationState::Pending
            );
        }
        assert_eq!(
            bounded.request(target(9), request(&device, 9)),
            CustomShaderPreparationState::WaitingAdmission
        );
        assert_eq!(bounded.capacity_status().queued, MAX_QUEUED);

        let mut broker = CustomShaderPreparationBroker::new(wake);
        assert_eq!(
            broker.request(first_target, first_request.clone()),
            CustomShaderPreparationState::Pending
        );
        let first = broker.take_dispatch().expect("first dispatch");
        let first_id = first.id();
        let first_key = first.key.clone();
        broker.release_target(first_target);
        assert_eq!(
            broker.request(first_target, first_request.clone()),
            CustomShaderPreparationState::Pending
        );
        first
            .sender
            .send(Completion {
                id: first_id,
                key: first_key.clone(),
                terminal: Terminal::Failed(CustomShaderPreparationFailure::Cancelled),
            })
            .expect("completion slot");
        drop(first);
        broker.drain_completions();
        assert_eq!(broker.capacity_status().active, 0);

        let second = broker.take_dispatch().expect("reactivated dispatch");
        let second_id = second.id();
        let second_key = second.key.clone();
        // A delayed terminal from the cancelled job must not consume the new
        // active reservation or overwrite its state.
        second
            .sender
            .send(Completion {
                id: first_id,
                key: first_key,
                terminal: Terminal::Failed(CustomShaderPreparationFailure::Cancelled),
            })
            .expect("completion slot");
        broker.drain_completions();
        assert_eq!(broker.capacity_status().active, 1);
        broker.reject_dispatch(second_id);
        assert_eq!(
            broker.failure(first_target),
            Some(CustomShaderPreparationFailure::Panicked)
        );
        broker.release_target(first_target);
        broker.maintain_retired();
        assert_eq!(
            broker.request(first_target, first_request),
            CustomShaderPreparationState::Pending
        );
        // The re-admitted target carries the exact original key after the
        // failed entry was released and retired.
        assert_eq!(
            second_key,
            broker
                .interests
                .get(&first_target)
                .and_then(|interest| match interest {
                    Interest::Entry(key) => Some(key),
                    Interest::Waiting => None,
                })
                .expect("replacement interest")
        );
    }

    #[test]
    #[ignore = "requires a native WGPU device"]
    fn native_ready_candidate_retires_after_final_consumed_lease() {
        let device = native_device();
        let target = target(41);
        let mut broker =
            CustomShaderPreparationBroker::new(Arc::new(CountingWake(AtomicUsize::new(0))));
        let request = request(&device, 41);
        assert_eq!(
            broker.request(target, request.clone()),
            CustomShaderPreparationState::Pending
        );
        broker.take_dispatch().expect("worker dispatch").run();
        assert_eq!(broker.drain_completions(), vec![target]);
        let prepared = broker.prepared(target).expect("ready candidate");
        assert!(prepared.matches(&request));
        assert_eq!(prepared.identity(), request.identity());
        broker.consume_target(target);
        // The renderer would retain only the cloned pipeline; this temporary
        // broker lease must be dropped before off-redraw retirement can free it.
        drop(prepared);
        broker.maintain_retired();
        assert_eq!(broker.capacity_status().entries, 0);
    }
}
