//! Native ownership and reconciliation for asynchronous custom-shader preparation.
//!
//! The broker owns only transient, device-bound candidates.  This module turns
//! the authoritative paint plan into exact-generation interests and hands
//! accepted candidates to the renderer transaction boundary.

use super::{
    GenericNativeVelloRunner, NativeAdapterGeneration,
    custom_shader_prepare::{
        CustomShaderPreparationBroker, CustomShaderPreparationRequest,
        CustomShaderPreparationState, CustomShaderTargetId, PreparedCustomShaderPipeline,
    },
    gpu_surface::{
        custom_shader::pipeline::{CustomShaderPreparationFailure, custom_shader_pipeline_key},
        gpu_surface_types::CustomShaderPipelineIdentity,
    },
    runner_state::NativeTargetGeneration,
};
use crate::{
    gui::repaint::RepaintSignal,
    runtime::{GpuSurfaceContent, PaintPrimitive, RuntimeBridge},
};
use std::{
    cell::RefCell,
    collections::{HashMap, HashSet},
    rc::Rc,
    sync::Arc,
};
use vello::wgpu;
use winit::window::WindowId;

pub(super) type SharedCustomShaderBroker = Rc<RefCell<CustomShaderPreparationBroker>>;
type RegistrationKey = (u64, usize);

pub(super) struct NativeCustomShaderPreparation {
    broker: SharedCustomShaderBroker,
    targets: HashMap<RegistrationKey, CustomShaderTargetId>,
    waiting_cursor: usize,
    waiting_retry_targets: HashSet<CustomShaderTargetId>,
}

impl NativeCustomShaderPreparation {
    pub(super) fn new(wake: Arc<dyn RepaintSignal>) -> Self {
        Self {
            broker: Rc::new(RefCell::new(CustomShaderPreparationBroker::new(wake))),
            targets: HashMap::new(),
            waiting_cursor: 0,
            waiting_retry_targets: HashSet::new(),
        }
    }

    pub(super) fn with_shared(broker: SharedCustomShaderBroker) -> Self {
        Self {
            broker,
            targets: HashMap::new(),
            waiting_cursor: 0,
            waiting_retry_targets: HashSet::new(),
        }
    }

    pub(super) fn shared(&self) -> SharedCustomShaderBroker {
        Rc::clone(&self.broker)
    }

    fn target_for(
        &mut self,
        window: WindowId,
        adapter: NativeAdapterGeneration,
        target: NativeTargetGeneration,
        registration: RegistrationKey,
    ) -> Option<(CustomShaderTargetId, bool)> {
        if let Some(current) = self.targets.get(&registration).copied()
            && current.window() == window
            && current.adapter_generation() == adapter
            && current.target_generation() == target
        {
            return Some((current, false));
        }
        if let Some(previous) = self.targets.remove(&registration) {
            self.broker.borrow_mut().release_target(previous);
        }
        // Match the broker's global interest bound locally.  A denied target
        // is deliberately not retained, so a later capacity transition can
        // admit it without stale runner ownership.
        if self.targets.len() >= 1024 {
            return None;
        }
        Some((
            CustomShaderTargetId::new(window, adapter, target, registration.0)?,
            true,
        ))
    }

    pub(super) fn release_all(&mut self) {
        let targets = std::mem::take(&mut self.targets);
        let mut broker = self.broker.borrow_mut();
        for target in targets.into_values() {
            broker.release_target(target);
        }
    }

    pub(super) fn accepts(&self, target: CustomShaderTargetId) -> bool {
        self.targets.values().any(|current| *current == target)
    }

    fn waiting_targets_rotating(&mut self, limit: usize) -> Vec<CustomShaderTargetId> {
        let mut waiting: Vec<_> = self.broker.borrow().waiting_targets().collect();
        if waiting.is_empty() || limit == 0 {
            return Vec::new();
        }
        waiting.sort_by_key(CustomShaderTargetId::surface_key);
        let start = self.waiting_cursor % waiting.len();
        let count = waiting.len().min(limit);
        let selected = (0..count)
            .map(|offset| waiting[(start + offset) % waiting.len()])
            .collect();
        self.waiting_cursor = (start + count) % waiting.len();
        selected
    }

    fn schedule_waiting_retry(&mut self, limit: usize) -> bool {
        self.waiting_targets_rotating(limit)
            .into_iter()
            .any(|target| self.waiting_retry_targets.insert(target))
    }

    fn take_waiting_retry_targets(&mut self) -> HashSet<CustomShaderTargetId> {
        std::mem::take(&mut self.waiting_retry_targets)
    }
}

/// A candidate and its exact UI-side target evidence.  Receipts are consumed
/// only after the whole renderer transaction commits.
pub(super) type PendingCustomShaderInstall = (
    CustomShaderTargetId,
    CustomShaderPreparationRequest,
    Option<PreparedCustomShaderPipeline>,
    Option<CustomShaderPreparationFailure>,
);

impl<Bridge, Message> GenericNativeVelloRunner<Bridge, Message>
where
    Bridge: RuntimeBridge<Message>,
{
    pub(super) fn install_custom_shader_broker(&mut self, broker: SharedCustomShaderBroker) {
        self.custom_shader_preparation = Some(NativeCustomShaderPreparation::with_shared(broker));
    }

    pub(super) fn shared_custom_shader_broker(&self) -> Option<SharedCustomShaderBroker> {
        self.custom_shader_preparation
            .as_ref()
            .map(NativeCustomShaderPreparation::shared)
    }

    pub(super) fn release_custom_shader_interests(&mut self) {
        if let Some(preparation) = self.custom_shader_preparation.as_mut() {
            preparation.release_all();
        }
    }

    pub(super) fn accepts_custom_shader_target(
        &self,
        target: CustomShaderTargetId,
        adapter: NativeAdapterGeneration,
    ) -> bool {
        self.is_running()
            && self.window.id == Some(target.window())
            && self.window.target_generation == target.target_generation()
            && adapter == target.adapter_generation()
            && self
                .custom_shader_preparation
                .as_ref()
                .is_some_and(|preparation| {
                    let registration = preparation
                        .targets
                        .iter()
                        .find_map(|(registration, current)| (*current == target).then_some(*registration));
                    registration.is_some_and(|(surface_key, primitive_index)| {
                        preparation.accepts(target)
                            && self.frame.last_paint_plan.primitives.get(primitive_index).is_some_and(|primitive| {
                                matches!(primitive, PaintPrimitive::GpuSurface(surface)
                                    if surface.key == surface_key
                                    && matches!(&surface.content, GpuSurfaceContent::CustomShader { .. }))
                            })
                    })
                })
    }

    /// Reconcile only after the current surface's UI-side device handle is
    /// available.  The captured address is passed through to the request; the
    /// worker's cloned device is never used as identity evidence.
    pub(super) fn reconcile_custom_shader_preparations(
        &mut self,
        adapter: NativeAdapterGeneration,
        device: &wgpu::Device,
        device_identity: usize,
        format: wgpu::TextureFormat,
        cached: &HashSet<CustomShaderPipelineIdentity>,
    ) -> Vec<PendingCustomShaderInstall> {
        let retry_targets = self
            .custom_shader_preparation
            .as_mut()
            .map(NativeCustomShaderPreparation::take_waiting_retry_targets)
            .unwrap_or_default();
        self.reconcile_custom_shader_preparations_filtered(
            adapter,
            device,
            device_identity,
            format,
            cached,
            Some(&retry_targets),
            true,
        )
    }

    pub(super) fn schedule_waiting_custom_shader_retry(&mut self, limit: usize) -> bool {
        self.custom_shader_preparation
            .as_mut()
            .is_some_and(|preparation| preparation.schedule_waiting_retry(limit))
    }

    fn reconcile_custom_shader_preparations_filtered(
        &mut self,
        adapter: NativeAdapterGeneration,
        device: &wgpu::Device,
        device_identity: usize,
        format: wgpu::TextureFormat,
        cached: &HashSet<CustomShaderPipelineIdentity>,
        retry_targets: Option<&HashSet<CustomShaderTargetId>>,
        wake_new_admission: bool,
    ) -> Vec<PendingCustomShaderInstall> {
        let Some(window) = self.window.id else {
            return Vec::new();
        };
        let target_generation = self.window.target_generation;
        let Some(preparation) = self.custom_shader_preparation.as_mut() else {
            return Vec::new();
        };
        let waiting: HashSet<_> = preparation.broker.borrow().waiting_targets().collect();
        let mut live = HashSet::new();
        let mut installs = Vec::new();

        // Every ordered occurrence remains an interest: the renderer's
        // transition preflight preserves duplicate surface keys with distinct
        // physical identities even though retained binding ownership is last-wins.
        for (primitive_index, primitive) in self.frame.last_paint_plan.primitives.iter().enumerate()
        {
            let PaintPrimitive::GpuSurface(surface) = primitive else {
                continue;
            };
            let GpuSurfaceContent::CustomShader { descriptor } = &surface.content else {
                continue;
            };
            let Some(key) = custom_shader_pipeline_key(descriptor) else {
                continue;
            };
            let registration = (surface.key, primitive_index);
            live.insert(registration);
            let request = CustomShaderPreparationRequest::new(
                device.clone(),
                device_identity,
                adapter,
                format,
                key,
            );
            let Some((target, is_new_target)) =
                preparation.target_for(window, adapter, target_generation, registration)
            else {
                continue;
            };
            if waiting.contains(&target)
                && retry_targets.is_some_and(|targets| !targets.contains(&target))
            {
                continue;
            }

            // Existing physical cache residency needs no broker interest or
            // worker task.  Any older candidate for this target is released.
            if cached.contains(&request.identity()) {
                preparation.broker.borrow_mut().release_target(target);
                preparation.targets.remove(&registration);
                continue;
            }

            let mut broker = preparation.broker.borrow_mut();
            let before = broker.capacity_status();
            let state = broker.request(target, request.clone());
            let failure = broker.failure(target);
            if matches!(state, CustomShaderPreparationState::Unavailable) && failure.is_none() {
                preparation.targets.remove(&registration);
            } else if is_new_target {
                preparation.targets.insert(registration, target);
            }
            if preparation.targets.contains_key(&registration) {
                live.insert(registration);
            }
            let capacity_changed = before != broker.capacity_status();
            if wake_new_admission && capacity_changed {
                broker.request_pump();
            }
            let prepared = broker
                .prepared(target)
                .filter(|prepared| prepared.matches(&request));
            let failure = prepared.is_none().then_some(failure).flatten();
            if prepared.is_some() || failure.is_some() {
                installs.push((target, request, prepared, failure));
                if installs.len() == 1024 {
                    break;
                }
            }
        }

        let stale: Vec<_> = preparation
            .targets
            .keys()
            .copied()
            .filter(|key| !live.contains(key))
            .collect();
        let capacity_changed = {
            let mut broker = preparation.broker.borrow_mut();
            let before = broker.capacity_status();
            for key in stale {
                if let Some(target) = preparation.targets.remove(&key) {
                    broker.release_target(target);
                }
            }
            before != broker.capacity_status()
        };
        if wake_new_admission && capacity_changed {
            preparation.schedule_waiting_retry(8);
            let broker = preparation.broker.borrow();
            broker.request_pump();
        }
        installs.reverse();
        installs
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct TestWake(AtomicUsize);
    impl RepaintSignal for TestWake {
        fn request_repaint(&self) {
            self.0.fetch_add(1, Ordering::Relaxed);
        }
    }

    fn adapter(serial: u64) -> NativeAdapterGeneration {
        NativeAdapterGeneration::from_test_serial(serial)
    }

    #[test]
    fn exact_window_adapter_and_target_epoch_replace_registration() {
        let wake = Arc::new(TestWake(AtomicUsize::new(0)));
        let mut preparation = NativeCustomShaderPreparation::new(wake);
        let window = WindowId::dummy();
        let first = preparation
            .target_for(
                window,
                adapter(1),
                NativeTargetGeneration::from_test_serial(1),
                (9, 0),
            )
            .expect("first target");
        preparation.targets.insert((9, 0), first.0);

        let stable = preparation
            .target_for(
                window,
                adapter(1),
                NativeTargetGeneration::from_test_serial(1),
                (9, 0),
            )
            .expect("stable target");
        assert_eq!(stable, (first.0, false));

        let replacement = preparation
            .target_for(
                window,
                adapter(1),
                NativeTargetGeneration::from_test_serial(2),
                (9, 0),
            )
            .expect("new epoch target");
        assert_ne!(replacement.0, first.0);
        assert!(replacement.1);
        assert_eq!(preparation.broker.borrow().capacity_status().interests, 0);
    }

    #[test]
    fn runner_target_map_stays_bounded_before_broker_admission() {
        let wake = Arc::new(TestWake(AtomicUsize::new(0)));
        let mut preparation = NativeCustomShaderPreparation::new(wake);
        let window = WindowId::dummy();
        for key in 0..1024 {
            let target = preparation
                .target_for(
                    window,
                    adapter(1),
                    NativeTargetGeneration::from_test_serial(1),
                    (key, key as usize),
                )
                .expect("bounded target")
                .0;
            preparation.targets.insert((key, key as usize), target);
        }
        assert!(
            preparation
                .target_for(
                    window,
                    adapter(1),
                    NativeTargetGeneration::from_test_serial(1),
                    (1024, 1024),
                )
                .is_none()
        );
    }

    #[test]
    fn duplicate_surface_occurrences_keep_distinct_target_registrations() {
        let wake = Arc::new(TestWake(AtomicUsize::new(0)));
        let mut preparation = NativeCustomShaderPreparation::new(wake);
        let window = WindowId::dummy();
        let first = preparation
            .target_for(
                window,
                adapter(1),
                NativeTargetGeneration::from_test_serial(1),
                (41, 3),
            )
            .expect("first occurrence")
            .0;
        preparation.targets.insert((41, 3), first);
        let second = preparation
            .target_for(
                window,
                adapter(1),
                NativeTargetGeneration::from_test_serial(1),
                (41, 8),
            )
            .expect("second occurrence")
            .0;
        preparation.targets.insert((41, 8), second);

        assert_ne!(first, second);
        assert_eq!(preparation.targets.len(), 2);
    }
}
