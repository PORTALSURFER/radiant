//! Native ownership and reconciliation for asynchronous raw signal summaries.
//!
//! This stays above the renderer boundary: it only turns the authoritative
//! paint plan into broker interests.  The caller installs an accepted prepared
//! summary into the current renderer immediately before GPU preflight.

use super::{
    GenericNativeVelloRunner, NativeAdapterGeneration,
    runner_state::NativeTargetGeneration,
    signal_summary_prepare::{PreparedSummary, SummaryBroker, SummaryRequest, SummaryTargetId},
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
use winit::window::WindowId;

pub(super) type SharedSummaryBroker = Rc<RefCell<SummaryBroker>>;

pub(super) struct NativeSignalSummaryPreparation {
    broker: SharedSummaryBroker,
    targets: HashMap<u64, SummaryTargetId>,
    waiting_cursor: usize,
}

impl NativeSignalSummaryPreparation {
    pub(super) fn new(wake: Arc<dyn RepaintSignal>) -> Self {
        Self {
            broker: Rc::new(RefCell::new(SummaryBroker::new(wake))),
            targets: HashMap::new(),
            waiting_cursor: 0,
        }
    }

    pub(super) fn with_shared(broker: SharedSummaryBroker) -> Self {
        Self {
            broker,
            targets: HashMap::new(),
            waiting_cursor: 0,
        }
    }

    pub(super) fn shared(&self) -> SharedSummaryBroker {
        Rc::clone(&self.broker)
    }

    fn target_for(
        &mut self,
        window: WindowId,
        adapter: NativeAdapterGeneration,
        target: NativeTargetGeneration,
        surface_key: u64,
    ) -> Option<(SummaryTargetId, bool)> {
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
        // The broker has the application-wide 128-interest authority.  Keep
        // this runner's registration map bounded too, and never retain a
        // denied target that the broker did not admit.
        if self.targets.len() >= 128 {
            return None;
        }
        Some((
            SummaryTargetId::new(window, adapter, target, surface_key)?,
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

    pub(super) fn accepts(&self, target: SummaryTargetId) -> bool {
        self.targets
            .get(&target.surface_key())
            .is_some_and(|current| *current == target)
    }

    fn waiting_targets_rotating(&mut self, limit: usize) -> Vec<SummaryTargetId> {
        let mut waiting: Vec<_> = self.broker.borrow().waiting_targets().collect();
        if waiting.is_empty() || limit == 0 {
            return Vec::new();
        }
        waiting.sort_by_key(SummaryTargetId::surface_key);
        let start = self.waiting_cursor % waiting.len();
        let count = waiting.len().min(limit);
        let selected = (0..count)
            .map(|offset| waiting[(start + offset) % waiting.len()])
            .collect();
        self.waiting_cursor = (start + count) % waiting.len();
        selected
    }
}

/// A prepared summary paired with the exact surface evidence which caused the
/// current runner to retain it.  It is intentionally consumed at the renderer
/// boundary before that frame's upload preflight.
pub(super) struct PendingSummaryInstall {
    pub(super) surface_key: u64,
    pub(super) revision: u64,
    pub(super) content: GpuSurfaceContent,
    pub(super) prepared: PreparedSummary,
}

impl<Bridge, Message> GenericNativeVelloRunner<Bridge, Message>
where
    Bridge: RuntimeBridge<Message>,
{
    pub(super) fn install_signal_summary_broker(&mut self, broker: SharedSummaryBroker) {
        self.signal_summary_preparation = Some(NativeSignalSummaryPreparation::with_shared(broker));
    }

    pub(super) fn shared_signal_summary_broker(&self) -> Option<SharedSummaryBroker> {
        self.signal_summary_preparation
            .as_ref()
            .map(NativeSignalSummaryPreparation::shared)
    }

    pub(super) fn release_signal_summary_interests(&mut self) {
        if let Some(preparation) = self.signal_summary_preparation.as_mut() {
            preparation.release_all();
        }
    }

    pub(super) fn accepts_signal_summary_target(
        &self,
        target: SummaryTargetId,
        adapter: NativeAdapterGeneration,
    ) -> bool {
        self.is_running()
            && self.window.id == Some(target.window())
            && self.window.target_generation == target.target_generation()
            && adapter == target.adapter_generation()
            && self
                .signal_summary_preparation
                .as_ref()
                .is_some_and(|preparation| {
                    preparation.accepts(target)
                        && self
                            .frame
                            .last_paint_plan
                            .primitives
                            .iter()
                            .any(|primitive| {
                                let PaintPrimitive::GpuSurface(surface) = primitive else {
                                    return false;
                                };
                                surface.key == target.surface_key()
                                    && preparation.broker.borrow().prepared(target).is_some_and(
                                        |prepared| {
                                            prepared.matches_raw_surface(
                                                &surface.content,
                                                surface.revision,
                                            )
                                        },
                                    )
                            })
                })
    }

    /// Reconcile raw surfaces from the authoritative plan.  This is called
    /// after rebuilding the plan and before GPU preflight; it never performs
    /// synchronous summary work and it never requests a frame while pending.
    pub(super) fn reconcile_signal_summary_interests(
        &mut self,
        adapter: NativeAdapterGeneration,
    ) -> Vec<PendingSummaryInstall> {
        self.reconcile_signal_summary_interests_filtered(adapter, None, true)
    }

    pub(super) fn reconcile_waiting_signal_summary_interests(
        &mut self,
        adapter: NativeAdapterGeneration,
        targets: &HashSet<SummaryTargetId>,
    ) {
        let _ = self.reconcile_signal_summary_interests_filtered(adapter, Some(targets), false);
    }

    pub(super) fn take_waiting_signal_summary_targets(
        &mut self,
        limit: usize,
    ) -> Vec<SummaryTargetId> {
        self.signal_summary_preparation
            .as_mut()
            .map_or_else(Vec::new, |preparation| {
                preparation.waiting_targets_rotating(limit)
            })
    }

    fn reconcile_signal_summary_interests_filtered(
        &mut self,
        adapter: NativeAdapterGeneration,
        only_targets: Option<&HashSet<SummaryTargetId>>,
        wake_new_admission: bool,
    ) -> Vec<PendingSummaryInstall> {
        let Some(window) = self.window.id else {
            return Vec::new();
        };
        let target_generation = self.window.target_generation;
        let Some(preparation) = self.signal_summary_preparation.as_mut() else {
            return Vec::new();
        };
        let mut live = HashSet::new();
        let mut installs = Vec::new();
        for primitive in &self.frame.last_paint_plan.primitives {
            let PaintPrimitive::GpuSurface(surface) = primitive else {
                continue;
            };
            let Some(request) =
                SummaryRequest::from_raw_surface(&surface.content, surface.revision)
            else {
                continue;
            };
            let Some((target, is_new_target)) =
                preparation.target_for(window, adapter, target_generation, surface.key)
            else {
                continue;
            };
            if only_targets.is_some_and(|targets| !targets.contains(&target)) {
                continue;
            }
            let mut broker = preparation.broker.borrow_mut();
            let state = broker.request(target, request);
            if is_new_target
                && !matches!(
                    state,
                    super::signal_summary_prepare::SummaryRequestState::Unavailable
                )
            {
                preparation.targets.insert(surface.key, target);
            }
            if preparation.targets.contains_key(&surface.key) {
                live.insert(surface.key);
            }
            if wake_new_admission
                && is_new_target
                && matches!(
                    state,
                    super::signal_summary_prepare::SummaryRequestState::Pending
                )
            {
                broker.request_pump();
            }
            if let Some(prepared) = broker.prepared(target)
                && prepared.matches_raw_surface(&surface.content, surface.revision)
            {
                installs.push(PendingSummaryInstall {
                    surface_key: surface.key,
                    revision: surface.revision,
                    content: surface.content.clone(),
                    prepared,
                });
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
            for key in stale {
                if let Some(target) = preparation.targets.remove(&key) {
                    broker.release_target(target);
                }
            }
        }
        installs
    }
}
