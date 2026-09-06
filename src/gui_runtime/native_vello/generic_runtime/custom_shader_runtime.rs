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
        wgpu_device_id,
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

pub(super) struct NativeCustomShaderPreparation {
    broker: SharedCustomShaderBroker,
    targets: HashMap<u64, CustomShaderTargetId>,
    waiting_cursor: usize,
}

impl NativeCustomShaderPreparation {
    pub(super) fn new(wake: Arc<dyn RepaintSignal>) -> Self {
        Self {
            broker: Rc::new(RefCell::new(CustomShaderPreparationBroker::new(wake))),
            targets: HashMap::new(),
            waiting_cursor: 0,
        }
    }

    pub(super) fn with_shared(broker: SharedCustomShaderBroker) -> Self {
        Self {
            broker,
            targets: HashMap::new(),
            waiting_cursor: 0,
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
        surface_key: u64,
    ) -> Option<(CustomShaderTargetId, bool)> {
        if let Some(current) = self.targets.get(&surface_key).copied()
            && current.window() == window
            && current.adapter_generation() == adapter
            && current.target_generation() == target
        {
            return Some((current, false));
        }
        if let Some(previous) = self.targets.remove(&surface_key) {
            self.broker.borrow_mut().release_target(previous);
        }
        // Match the broker's global interest bound locally.  A denied target
        // is deliberately not retained, so a later capacity transition can
        // admit it without stale runner ownership.
        if self.targets.len() >= 1024 {
            return None;
        }
        Some((
            CustomShaderTargetId::new(window, adapter, target, surface_key)?,
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
        self.targets
            .get(&target.surface_key())
            .is_some_and(|current| *current == target)
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
                    preparation.accepts(target)
                        && self
                            .frame
                            .last_paint_plan
                            .primitives
                            .iter()
                            .rev()
                            .find_map(|primitive| {
                                let PaintPrimitive::GpuSurface(surface) = primitive else {
                                    return None;
                                };
                                (surface.key == target.surface_key()).then_some(surface)
                            })
                            .is_some_and(|surface| {
                                matches!(&surface.content, GpuSurfaceContent::CustomShader { .. })
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
        format: wgpu::TextureFormat,
        cached: &HashSet<CustomShaderPipelineIdentity>,
    ) -> Vec<PendingCustomShaderInstall> {
        self.reconcile_custom_shader_preparations_filtered(
            adapter, device, format, cached, None, true,
        )
    }

    pub(super) fn reconcile_waiting_custom_shader_preparations(
        &mut self,
        adapter: NativeAdapterGeneration,
        device: &wgpu::Device,
        format: wgpu::TextureFormat,
        cached: &HashSet<CustomShaderPipelineIdentity>,
        targets: &HashSet<CustomShaderTargetId>,
    ) -> bool {
        !self
            .reconcile_custom_shader_preparations_filtered(
                adapter,
                device,
                format,
                cached,
                Some(targets),
                false,
            )
            .is_empty()
    }

    pub(super) fn take_waiting_custom_shader_targets(
        &mut self,
        limit: usize,
    ) -> Vec<CustomShaderTargetId> {
        self.custom_shader_preparation
            .as_mut()
            .map_or_else(Vec::new, |preparation| {
                preparation.waiting_targets_rotating(limit)
            })
    }

    fn reconcile_custom_shader_preparations_filtered(
        &mut self,
        adapter: NativeAdapterGeneration,
        device: &wgpu::Device,
        format: wgpu::TextureFormat,
        cached: &HashSet<CustomShaderPipelineIdentity>,
        only_targets: Option<&HashSet<CustomShaderTargetId>>,
        wake_new_admission: bool,
    ) -> Vec<PendingCustomShaderInstall> {
        let Some(window) = self.window.id else {
            return Vec::new();
        };
        let target_generation = self.window.target_generation;
        let Some(preparation) = self.custom_shader_preparation.as_mut() else {
            return Vec::new();
        };
        let device_identity = wgpu_device_id(device);
        let mut live = HashSet::new();
        let mut seen_surface_keys = HashSet::new();
        let mut installs = Vec::new();

        // Reverse iteration makes the retained-plan replacement rule explicit:
        // the last descriptor for a duplicated surface key is authoritative.
        for primitive in self.frame.last_paint_plan.primitives.iter().rev() {
            let PaintPrimitive::GpuSurface(surface) = primitive else {
                continue;
            };
            if !seen_surface_keys.insert(surface.key) {
                continue;
            }
            let GpuSurfaceContent::CustomShader { descriptor } = &surface.content else {
                continue;
            };
            let Some(key) = custom_shader_pipeline_key(descriptor) else {
                continue;
            };
            live.insert(surface.key);
            let request = CustomShaderPreparationRequest::new(
                device.clone(),
                device_identity,
                adapter,
                format,
                key,
            );
            let Some((target, is_new_target)) =
                preparation.target_for(window, adapter, target_generation, surface.key)
            else {
                continue;
            };
            if only_targets.is_some_and(|targets| !targets.contains(&target)) {
                continue;
            }

            // Existing physical cache residency needs no broker interest or
            // worker task.  Any older candidate for this target is released.
            if cached.contains(&request.identity()) {
                preparation.broker.borrow_mut().release_target(target);
                preparation.targets.remove(&surface.key);
                continue;
            }

            let mut broker = preparation.broker.borrow_mut();
            let before = broker.capacity_status();
            let state = broker.request(target, request.clone());
            if matches!(state, CustomShaderPreparationState::Unavailable) {
                preparation.targets.remove(&surface.key);
            } else if is_new_target {
                preparation.targets.insert(surface.key, target);
            }
            if preparation.targets.contains_key(&surface.key) {
                live.insert(surface.key);
            }
            let capacity_changed = before != broker.capacity_status();
            if wake_new_admission && capacity_changed {
                broker.request_pump();
            }
            let prepared = broker
                .prepared(target)
                .filter(|prepared| prepared.matches(&request));
            let failure = prepared.is_none().then(|| broker.failure(target)).flatten();
            if prepared.is_some() || failure.is_some() {
                installs.push((target, request, prepared, failure));
                if installs.len() == 1024 {
                    break;
                }
            }
        }

        if only_targets.is_none() {
            let stale: Vec<_> = preparation
                .targets
                .keys()
                .copied()
                .filter(|key| !live.contains(key))
                .collect();
            let mut broker = preparation.broker.borrow_mut();
            let before = broker.capacity_status();
            for key in stale {
                if let Some(target) = preparation.targets.remove(&key) {
                    broker.release_target(target);
                }
            }
            if wake_new_admission && before != broker.capacity_status() {
                broker.request_pump();
            }
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
                9,
            )
            .expect("first target");
        preparation.targets.insert(9, first.0);

        let stable = preparation
            .target_for(
                window,
                adapter(1),
                NativeTargetGeneration::from_test_serial(1),
                9,
            )
            .expect("stable target");
        assert_eq!(stable, (first.0, false));

        let replacement = preparation
            .target_for(
                window,
                adapter(1),
                NativeTargetGeneration::from_test_serial(2),
                9,
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
                    key,
                )
                .expect("bounded target")
                .0;
            preparation.targets.insert(key, target);
        }
        assert!(
            preparation
                .target_for(
                    window,
                    adapter(1),
                    NativeTargetGeneration::from_test_serial(1),
                    1024,
                )
                .is_none()
        );
    }
}
