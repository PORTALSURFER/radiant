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
    ) -> bool {
        !self
            .reconcile_signal_summary_interests_filtered(adapter, Some(targets), false)
            .is_empty()
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
        // Keep at most one prepared install per retained surface key.  Later
        // paint-plan entries replace earlier ones, matching retained-resource
        // update order without allowing duplicate keys to grow frame-local
        // work beyond the broker's target bound.
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
            let capacity_before = broker.capacity_status();
            let state = broker.request(target, request);
            if matches!(
                state,
                super::signal_summary_prepare::SummaryRequestState::Unavailable
            ) {
                tracing::debug!(target: "radiant::signal_summary_prepare", surface_key = surface.key,
                    "raw signal preparation unavailable");
            }
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
            let capacity_changed = capacity_before != broker.capacity_status();
            if wake_new_admission && capacity_changed {
                broker.request_pump();
            }
            if let Some(prepared) = broker.prepared(target)
                && prepared.matches_raw_surface(&surface.content, surface.revision)
            {
                let install = PendingSummaryInstall {
                    surface_key: surface.key,
                    revision: surface.revision,
                    content: surface.content.clone(),
                    prepared,
                };
                if let Some(index) = installs
                    .iter()
                    .position(|existing: &PendingSummaryInstall| {
                        existing.surface_key == surface.key
                    })
                {
                    installs[index] = install;
                } else if installs.len() < 128 {
                    installs.push(install);
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
            let capacity_before = broker.capacity_status();
            for key in stale {
                if let Some(target) = preparation.targets.remove(&key) {
                    broker.release_target(target);
                }
            }
            if wake_new_admission && capacity_before != broker.capacity_status() {
                broker.request_pump();
            }
        }
        installs
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        gui::types::{Point, Rect, Rgba8, Vector2},
        runtime::{GpuSurfaceCapabilities, PaintGpuSurface, SurfacePaintPlan, UiSurface},
    };
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct TestWake(AtomicUsize);

    impl RepaintSignal for TestWake {
        fn request_repaint(&self) {
            self.0.fetch_add(1, Ordering::Relaxed);
        }
    }

    struct EmptyBridge;

    impl RuntimeBridge<()> for EmptyBridge {
        fn project_surface(&mut self) -> Arc<UiSurface<()>> {
            crate::runtime::test_arc_surface(UiSurface::new(crate::runtime::SurfaceNode::widget(
                crate::widgets::TextWidget::new(
                    1,
                    "",
                    crate::widgets::WidgetSizing::fixed(Vector2::new(1.0, 1.0)),
                ),
                crate::runtime::WidgetMessageMapper::none(),
            )))
        }
    }

    fn signal(key: u64, revision: u64, samples: Arc<[f32]>) -> PaintPrimitive {
        PaintPrimitive::GpuSurface(PaintGpuSurface {
            widget_id: key,
            key,
            revision,
            rect: Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(32.0, 16.0)),
            content: GpuSurfaceContent::SignalBands {
                frames: 4,
                band_count: 1,
                frame_range: [0.0, 4.0],
                samples,
            },
            capabilities: GpuSurfaceCapabilities::default(),
            overlays: Vec::new(),
        })
    }

    fn runner(wake: Arc<TestWake>) -> GenericNativeVelloRunner<EmptyBridge, ()> {
        let mut runner = GenericNativeVelloRunner::new(
            crate::runtime::NativeRunOptions::default(),
            EmptyBridge,
            Vector2::new(64.0, 64.0),
        );
        runner.window.id = Some(WindowId::dummy());
        runner.window.target_generation = NativeTargetGeneration::from_test_serial(1);
        runner.signal_summary_preparation = Some(NativeSignalSummaryPreparation::new(wake));
        runner
    }

    fn set_plan(
        runner: &mut GenericNativeVelloRunner<EmptyBridge, ()>,
        primitives: Vec<PaintPrimitive>,
    ) {
        runner.frame.last_paint_plan = SurfacePaintPlan {
            clear_color: Rgba8::default(),
            primitives,
        };
    }

    fn adapter() -> NativeAdapterGeneration {
        NativeAdapterGeneration::from_test_serial(1)
    }

    #[test]
    fn stable_target_source_replacement_wakes_without_pending_retry_loop() {
        let wake = Arc::new(TestWake(AtomicUsize::new(0)));
        let mut runner = runner(Arc::clone(&wake));
        set_plan(
            &mut runner,
            vec![signal(7, 1, Arc::from([0.0, 0.5, -0.5, 0.25]))],
        );

        let _ = runner.reconcile_signal_summary_interests(adapter());
        assert_eq!(wake.0.load(Ordering::Relaxed), 1);
        let _ = runner.reconcile_signal_summary_interests(adapter());
        assert_eq!(wake.0.load(Ordering::Relaxed), 1);

        set_plan(
            &mut runner,
            vec![signal(7, 2, Arc::from([0.1, 0.6, -0.4, 0.3]))],
        );
        let _ = runner.reconcile_signal_summary_interests(adapter());
        // This test sink counts requests; the native signal coalesces release
        // and admission into one event. A stable pending request issues neither.
        let after_replacement = wake.0.load(Ordering::Relaxed);
        assert!(after_replacement > 1);
        let _ = runner.reconcile_signal_summary_interests(adapter());
        assert_eq!(wake.0.load(Ordering::Relaxed), after_replacement);
    }

    #[test]
    fn target_epoch_replacement_releases_the_old_registration() {
        let wake = Arc::new(TestWake(AtomicUsize::new(0)));
        let mut runner = runner(wake);
        set_plan(
            &mut runner,
            vec![signal(9, 1, Arc::from([0.0, 0.5, -0.5, 0.25]))],
        );
        let _ = runner.reconcile_signal_summary_interests(adapter());
        let old = runner.signal_summary_preparation.as_ref().unwrap().targets[&9];

        runner.window.target_generation = NativeTargetGeneration::from_test_serial(2);
        let _ = runner.reconcile_signal_summary_interests(adapter());
        let preparation = runner.signal_summary_preparation.as_ref().unwrap();
        assert_ne!(preparation.targets[&9], old);
        assert_eq!(preparation.shared().borrow().capacity_status().interests, 1);
    }

    #[test]
    fn ready_waiting_retry_and_duplicate_install_work_stay_bounded() {
        let wake = Arc::new(TestWake(AtomicUsize::new(0)));
        let mut runner = runner(wake);
        let content: Arc<[f32]> = Arc::from([0.0, 0.5, -0.5, 0.25]);
        set_plan(
            &mut runner,
            (0..200)
                .map(|_| signal(11, 1, Arc::clone(&content)))
                .collect(),
        );
        let _ = runner.reconcile_signal_summary_interests(adapter());
        let broker = runner.shared_signal_summary_broker().unwrap();
        broker
            .borrow_mut()
            .take_dispatch()
            .expect("initial summary dispatch")
            .run();
        broker.borrow_mut().drain_completions();
        let target = runner.signal_summary_preparation.as_ref().unwrap().targets[&11];
        let waiting = HashSet::from([target]);
        assert!(runner.reconcile_waiting_signal_summary_interests(adapter(), &waiting));
        assert_eq!(
            runner.reconcile_signal_summary_interests(adapter()).len(),
            1
        );

        set_plan(
            &mut runner,
            (0..160)
                .map(|key| signal(key, 1, Arc::clone(&content)))
                .collect(),
        );
        let _ = runner.reconcile_signal_summary_interests(adapter());
        assert_eq!(
            runner
                .signal_summary_preparation
                .as_ref()
                .unwrap()
                .targets
                .len(),
            128
        );
    }
}
