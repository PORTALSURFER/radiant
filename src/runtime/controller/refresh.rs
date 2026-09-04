//! Revision-backed surface refresh stages and diagnostics.

use super::{
    SurfaceRuntime,
    interaction_state::{
        RuntimeFocusOwner, RuntimeManagedPointerCaptureState, RuntimeManagedWheelSequenceState,
    },
    layout_state::SurfaceLayoutStateDiagnostics,
    virtual_layout::RuntimeVirtualLayoutProjectionProbe,
};
use crate::gui::types::{Point, Rect, Vector2};
use crate::runtime::{
    RepaintScope, RuntimeBridge, SurfaceInvalidation,
    surface::{
        ReconciliationAttemptOutcome, RefreshExecutionDecision, SurfaceDamage,
        ViewDeltaDiagnostics, WidgetReplacementCommitResult, WidgetReplacementPlan,
        WidgetReplacementPlanVeto, classify_view_delta,
    },
};
use crate::widgets::WidgetId;
use std::fmt::Write as _;
use std::time::{Duration, Instant};

const MAX_IDENTITY_REPLACEMENTS_PER_REFRESH: usize = 4;
const MAX_IDENTITY_PATH_COMPONENTS: usize = 8;
const INVALID_COMPATIBILITY_KIND: &str = "<invalid-cached-widget-evidence>";

/// Runtime policy for incompatible same-ID widget replacements.
///
/// The default observational policy completes the safe replacement cleanup and
/// records bounded diagnostics without interrupting the host. [`Self::strict`]
/// is intended for deterministic tests and fails after that cleanup and
/// diagnostics commit whenever a refresh observes one or more replacements.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum IdentityAudit {
    /// Recover safely and leave the replacement available through diagnostics.
    #[default]
    Observational,
    /// Recover safely, commit diagnostics, then fail the completed refresh.
    Strict,
}

impl IdentityAudit {
    /// Return the strict identity-audit policy.
    pub const fn strict() -> Self {
        Self::Strict
    }

    /// Return the observational identity-audit policy.
    pub const fn observational() -> Self {
        Self::Observational
    }

    const fn is_strict(self) -> bool {
        matches!(self, Self::Strict)
    }
}

/// A bounded, resolved widget path retained in an identity diagnostic.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SurfaceIdentityPath {
    /// Path components from the projected surface root.
    pub components: [usize; MAX_IDENTITY_PATH_COMPONENTS],
    /// Number of valid components in [`Self::components`].
    pub len: u8,
    /// Whether the resolved path exceeded the diagnostic bound.
    pub truncated: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        gui::types::{Point, Rect, Vector2},
        layout::{
            ContainerKind, ContainerPolicy, LAYOUT_CAPABILITIES_CONTRACT_VERSION,
            LAYOUT_CAPABILITIES_PROJECTION_CONTRACT_VERSION, LayoutCapabilities, LayoutHitRegion,
            LayoutHitRegionId, LayoutInteraction, LayoutInteractionRevision, OverflowPolicy,
            SlotParams,
        },
        runtime::{
            Command, Event, RuntimeBridge, RuntimeHostCapabilities, RuntimeTaskHost, SurfaceChild,
            SurfaceNode, TaskPriority, UiSurface, WidgetMessageMapper,
            surface::{ViewDeltaCause, ViewDeltaEffect, WidgetReplacementPlanVeto},
        },
        widgets::{
            ButtonWidget, EditPhase, InteractionProvenance, KnobDomainCancellationReason,
            KnobDomainMessage, KnobPointerMetadata, KnobWidget, NumericAdjustment, NumericCodec,
            NumericInputEditBatch, NumericInputInteraction, NumericInputInteractionBatch,
            NumericInputWidget, NumericParseResult, NumericStep, NumericStepDirection,
            RetainedKnobDomainWidget, ScrollbarAxis, ScrollbarWidget, TextEditCommand, TextWidget,
            Widget, WidgetCommon, WidgetInput, WidgetKey, WidgetOutput, WidgetSizing,
        },
    };
    use std::{
        cell::{Cell, RefCell},
        rc::Rc,
        sync::Arc,
    };

    #[derive(Clone)]
    struct FenceSemanticWidget {
        common: crate::widgets::WidgetCommon,
        revision: crate::widgets::WidgetSemanticsRevision,
    }

    impl FenceSemanticWidget {
        fn new(id: u64, revision: &'static str) -> Self {
            Self {
                common: crate::widgets::WidgetCommon::fixed(id, 80.0, 28.0),
                revision: crate::widgets::WidgetSemanticsRevision::exact(revision),
            }
        }
    }

    impl crate::widgets::WidgetSemantics for FenceSemanticWidget {
        fn revision(&self) -> crate::widgets::WidgetSemanticsRevision {
            self.revision.clone()
        }
    }

    impl crate::widgets::Widget for FenceSemanticWidget {
        fn revision(&self) -> crate::widgets::WidgetRevision {
            crate::widgets::WidgetRevision::exact((), (), (), ())
        }

        fn capabilities(&self) -> crate::widgets::WidgetCapabilities<'_> {
            crate::widgets::WidgetCapabilities::new().semantics(self)
        }

        fn common(&self) -> &crate::widgets::WidgetCommon {
            &self.common
        }

        fn common_mut(&mut self) -> &mut crate::widgets::WidgetCommon {
            &mut self.common
        }

        fn handle_input(
            &mut self,
            _bounds: crate::gui::types::Rect,
            _input: crate::widgets::WidgetInput,
        ) -> Option<crate::widgets::WidgetOutput> {
            None
        }

        fn append_paint(
            &self,
            _primitives: &mut Vec<crate::runtime::PaintPrimitive>,
            _bounds: crate::gui::types::Rect,
            _layout: &crate::layout::LayoutOutput,
            _theme: &crate::theme::ThemeTokens,
        ) {
        }
    }

    #[derive(Default)]
    struct ReplacementBridge {
        replace: bool,
        replacement_count: usize,
        deep: bool,
        geometry: bool,
        mapper_changed: bool,
        geometry_mode: bool,
        exact: bool,
        semantic_mode: bool,
        semantic_changed: bool,
    }

    impl RuntimeBridge<()> for ReplacementBridge {
        fn project_surface(&mut self) -> Arc<UiSurface<()>> {
            if self.replacement_count != 0 {
                return crate::runtime::test_arc_surface(UiSurface::new(SurfaceNode::row(
                    1,
                    0.0,
                    (0..self.replacement_count)
                        .map(|index| {
                            SurfaceChild::fill(replacement_widget(index as u64 + 20, self.replace))
                        })
                        .collect(),
                )));
            }
            if self.geometry_mode {
                let mut slot = crate::layout::SlotParams::fill();
                slot.margin.left = if self.geometry { 4.0 } else { 0.0 };
                return crate::runtime::test_arc_surface(UiSurface::new(SurfaceNode::row(
                    1,
                    0.0,
                    vec![SurfaceChild::new(
                        slot,
                        SurfaceNode::widget(
                            crate::widgets::TextWidget::new(
                                20,
                                "Stable",
                                WidgetSizing::fixed(Vector2::new(80.0, 28.0)),
                            ),
                            WidgetMessageMapper::none(),
                        ),
                    )],
                )));
            }
            if self.exact {
                return crate::runtime::test_arc_surface(UiSurface::new(SurfaceNode::widget(
                    crate::widgets::TextWidget::new(
                        20,
                        "Stable",
                        WidgetSizing::fixed(Vector2::new(80.0, 28.0)),
                    ),
                    WidgetMessageMapper::none(),
                )));
            }
            if self.semantic_mode {
                let revision = if self.semantic_changed {
                    "after"
                } else {
                    "before"
                };
                return crate::runtime::test_arc_surface(UiSurface::new(SurfaceNode::widget(
                    FenceSemanticWidget::new(20, revision),
                    WidgetMessageMapper::none(),
                )));
            }
            if self.deep {
                let mut node = replacement_widget(20, self.replace);
                for id in 0..(MAX_IDENTITY_PATH_COMPONENTS + 2) {
                    node =
                        SurfaceNode::column(id as u64 + 100, 0.0, vec![SurfaceChild::fill(node)]);
                }
                return crate::runtime::test_arc_surface(UiSurface::new(node));
            }
            let mapper = if self.mapper_changed {
                WidgetMessageMapper::dynamic(|_| None)
            } else {
                WidgetMessageMapper::none()
            };
            let node = if self.replace {
                SurfaceNode::widget(
                    ScrollbarWidget::new(
                        20,
                        ScrollbarAxis::Vertical,
                        WidgetSizing::fixed(Vector2::new(16.0, 80.0)),
                    ),
                    mapper,
                )
            } else {
                SurfaceNode::widget(
                    ButtonWidget::new(
                        20,
                        "Previous",
                        WidgetSizing::fixed(Vector2::new(80.0, 28.0)),
                    ),
                    mapper,
                )
            };
            crate::runtime::test_arc_surface(UiSurface::new(node))
        }
    }

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    enum TeardownEvent {
        Prepare {
            id: u64,
            successor_authority: Option<bool>,
            projection_depth: usize,
        },
        Map {
            id: u64,
            marker: u8,
        },
        Synchronize {
            id: u64,
        },
        Reduce {
            id: u64,
            marker: u8,
        },
        Project,
        ExternalWork,
    }

    #[derive(Default)]
    struct TeardownLog {
        events: Vec<TeardownEvent>,
    }

    #[derive(Clone, Copy)]
    struct TeardownProbeConfig {
        active: bool,
        authority: bool,
        disabled: bool,
        read_only: bool,
        marker: u8,
        maps_output: bool,
    }

    #[derive(Clone, Copy)]
    struct TeardownPayload {
        id: u64,
    }

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    struct TeardownMessage {
        id: u64,
        marker: u8,
    }

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    enum TeardownDispatchMode {
        None,
        Exit,
        Focus,
        Nested,
        External,
    }

    #[derive(Clone)]
    struct TeardownProbeWidget {
        common: crate::widgets::WidgetCommon,
        active: bool,
        authority: bool,
        log: Rc<RefCell<TeardownLog>>,
        projection_depth: Rc<Cell<usize>>,
    }

    impl TeardownProbeWidget {
        fn new(
            id: u64,
            config: TeardownProbeConfig,
            log: Rc<RefCell<TeardownLog>>,
            projection_depth: Rc<Cell<usize>>,
        ) -> Self {
            let mut common = crate::widgets::WidgetCommon::fixed(id, 80.0, 28.0);
            common.focus = crate::widgets::FocusBehavior::Keyboard;
            common.state.disabled = config.disabled;
            common.state.read_only = config.read_only;
            Self {
                common,
                active: config.active,
                authority: config.authority,
                log,
                projection_depth,
            }
        }

        fn has_authority(&self) -> bool {
            self.authority && !self.common.state.disabled && !self.common.state.read_only
        }
    }

    impl crate::widgets::Widget for TeardownProbeWidget {
        fn revision(&self) -> crate::widgets::WidgetRevision {
            crate::widgets::WidgetRevision::exact((), (), (), ())
        }

        fn common(&self) -> &crate::widgets::WidgetCommon {
            &self.common
        }

        fn common_mut(&mut self) -> &mut crate::widgets::WidgetCommon {
            &mut self.common
        }

        fn handle_input(
            &mut self,
            _bounds: crate::gui::types::Rect,
            _input: crate::widgets::WidgetInput,
        ) -> Option<crate::widgets::WidgetOutput> {
            None
        }

        fn synchronize_from_previous(&mut self, previous: &dyn crate::widgets::Widget) {
            let Some(previous) = previous.as_any().downcast_ref::<Self>() else {
                return;
            };
            self.active = previous.active;
            self.log
                .borrow_mut()
                .events
                .push(TeardownEvent::Synchronize { id: self.common.id });
        }

        fn prepare_replacement(
            &mut self,
            successor: Option<&dyn crate::widgets::Widget>,
        ) -> Option<crate::widgets::WidgetOutput> {
            let successor_authority = successor.and_then(|successor| {
                successor
                    .as_any()
                    .downcast_ref::<Self>()
                    .map(Self::has_authority)
            });
            self.log.borrow_mut().events.push(TeardownEvent::Prepare {
                id: self.common.id,
                successor_authority,
                projection_depth: self.projection_depth.get(),
            });
            if self.active && successor_authority != Some(true) {
                self.active = false;
                return Some(crate::widgets::WidgetOutput::typed(TeardownPayload {
                    id: self.common.id,
                }));
            }
            None
        }

        fn append_paint(
            &self,
            _primitives: &mut Vec<crate::runtime::PaintPrimitive>,
            _bounds: crate::gui::types::Rect,
            _layout: &crate::layout::LayoutOutput,
            _theme: &crate::theme::ThemeTokens,
        ) {
        }
    }

    #[derive(Clone, Copy)]
    enum TeardownMode {
        Single(TeardownProbeConfig),
        Removed,
        Incompatible,
        Multiple([TeardownProbeConfig; 2]),
    }

    struct TeardownBridge {
        mode: TeardownMode,
        dispatch_mode: TeardownDispatchMode,
        record_projections: bool,
        log: Rc<RefCell<TeardownLog>>,
        projection_depth: Rc<Cell<usize>>,
        reduced: Vec<TeardownMessage>,
    }

    impl TeardownBridge {
        fn new(mode: TeardownMode) -> (Self, Rc<RefCell<TeardownLog>>) {
            let log = Rc::new(RefCell::new(TeardownLog::default()));
            let projection_depth = Rc::new(Cell::new(0));
            (
                Self {
                    mode,
                    dispatch_mode: TeardownDispatchMode::None,
                    record_projections: false,
                    log: Rc::clone(&log),
                    projection_depth,
                    reduced: Vec::new(),
                },
                log,
            )
        }

        fn probe_widget(
            &self,
            id: u64,
            config: TeardownProbeConfig,
        ) -> SurfaceNode<TeardownMessage> {
            let widget = TeardownProbeWidget::new(
                id,
                config,
                Rc::clone(&self.log),
                Rc::clone(&self.projection_depth),
            );
            let marker = config.marker;
            let log = Rc::clone(&self.log);
            let mapper = if config.maps_output {
                WidgetMessageMapper::dynamic(move |output| {
                    output.typed_cloned::<TeardownPayload>().map(|payload| {
                        log.borrow_mut().events.push(TeardownEvent::Map {
                            id: payload.id,
                            marker,
                        });
                        TeardownMessage {
                            id: payload.id,
                            marker,
                        }
                    })
                })
            } else {
                WidgetMessageMapper::none()
            };
            SurfaceNode::widget(widget, mapper)
        }

        fn surface(&self) -> UiSurface<TeardownMessage> {
            match self.mode {
                TeardownMode::Single(config) => UiSurface::new(self.probe_widget(20, config)),
                TeardownMode::Removed => UiSurface::new(SurfaceNode::container(
                    1,
                    ContainerPolicy::default(),
                    Vec::new(),
                )),
                TeardownMode::Incompatible => UiSurface::new(SurfaceNode::widget(
                    ButtonWidget::new(
                        20,
                        "replacement",
                        WidgetSizing::fixed(Vector2::new(80.0, 28.0)),
                    ),
                    WidgetMessageMapper::none(),
                )),
                TeardownMode::Multiple(configs) => UiSurface::new(SurfaceNode::column(
                    1,
                    0.0,
                    configs
                        .into_iter()
                        .enumerate()
                        .map(|(index, config)| {
                            SurfaceChild::fill(self.probe_widget(index as u64 + 20, config))
                        })
                        .collect(),
                )),
            }
        }
    }

    impl RuntimeBridge<TeardownMessage> for TeardownBridge {
        fn project_surface(&mut self) -> Arc<UiSurface<TeardownMessage>> {
            assert_eq!(self.projection_depth.get(), 0);
            if self.record_projections {
                self.log.borrow_mut().events.push(TeardownEvent::Project);
            }
            self.projection_depth.set(1);
            let surface = self.surface();
            self.projection_depth.set(0);
            crate::runtime::test_arc_surface(surface)
        }

        fn update(&mut self, message: TeardownMessage) -> Command<TeardownMessage> {
            self.reduce_message(message);
            if message.id != 20 {
                return Command::none();
            }
            match self.dispatch_mode {
                TeardownDispatchMode::None => Command::none(),
                TeardownDispatchMode::Exit => Command::exit(),
                TeardownDispatchMode::Focus => Command::focus(20),
                TeardownDispatchMode::Nested => Command::message(TeardownMessage {
                    id: 99,
                    marker: message.marker,
                }),
                TeardownDispatchMode::External => Command::perform_worker_effect_with_priority(
                    "teardown-external-work",
                    TaskPriority::Background,
                    None,
                    0,
                    || (),
                    |_| TeardownMessage { id: 98, marker: 0 },
                ),
            }
        }

        fn reduce_message(&mut self, message: TeardownMessage) {
            self.log.borrow_mut().events.push(TeardownEvent::Reduce {
                id: message.id,
                marker: message.marker,
            });
            self.reduced.push(message);
        }

        fn host_capabilities(&self) -> RuntimeHostCapabilities<Self, TeardownMessage> {
            RuntimeHostCapabilities::new().with_tasks()
        }
    }

    impl RuntimeTaskHost<TeardownMessage> for TeardownBridge {
        fn spawn_worker_task(
            &mut self,
            _name: &'static str,
            _priority: TaskPriority,
            _is_cancelled: Option<Box<dyn Fn() -> bool + Send + Sync + 'static>>,
            work: Box<dyn FnOnce() + Send + 'static>,
        ) -> bool {
            self.log
                .borrow_mut()
                .events
                .push(TeardownEvent::ExternalWork);
            drop(work);
            true
        }
    }

    fn active_probe(marker: u8) -> TeardownProbeConfig {
        TeardownProbeConfig {
            active: true,
            authority: true,
            disabled: false,
            read_only: false,
            marker,
            maps_output: true,
        }
    }

    fn terminal_batch_fixture(
        dispatch_mode: TeardownDispatchMode,
        record_projections: bool,
    ) -> (Vec<TeardownMessage>, Vec<TeardownEvent>) {
        let (bridge, log) =
            TeardownBridge::new(TeardownMode::Multiple([active_probe(1), active_probe(2)]));
        let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(200.0, 80.0));
        log.borrow_mut().events.clear();
        let bridge = runtime.bridge_mut();
        bridge.mode = TeardownMode::Multiple([
            TeardownProbeConfig {
                active: false,
                authority: false,
                marker: 9,
                ..active_probe(9)
            },
            TeardownProbeConfig {
                active: false,
                authority: false,
                marker: 9,
                ..active_probe(9)
            },
        ]);
        bridge.dispatch_mode = dispatch_mode;
        bridge.record_projections = record_projections;

        runtime.refresh();

        let reduced = runtime.bridge().reduced.clone();
        let events = log.borrow().events.clone();
        (reduced, events)
    }

    fn reduced_ids(events: &[TeardownEvent]) -> Vec<u64> {
        events
            .iter()
            .filter_map(|event| match event {
                TeardownEvent::Reduce { id, .. } => Some(*id),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn terminal_messages_reduce_before_exit_focus_nested_or_external_work() {
        let (reduced, events) = terminal_batch_fixture(TeardownDispatchMode::Exit, false);
        assert_eq!(
            reduced,
            [
                TeardownMessage { id: 20, marker: 1 },
                TeardownMessage { id: 21, marker: 2 },
            ]
        );
        assert_eq!(reduced_ids(&events), [20, 21]);

        let (reduced, events) = terminal_batch_fixture(TeardownDispatchMode::Focus, true);
        assert_eq!(
            reduced,
            [
                TeardownMessage { id: 20, marker: 1 },
                TeardownMessage { id: 21, marker: 2 },
            ]
        );
        assert_eq!(reduced_ids(&events), [20, 21]);
        let project_positions = events
            .iter()
            .enumerate()
            .filter_map(|(index, event)| matches!(event, TeardownEvent::Project).then_some(index))
            .collect::<Vec<_>>();
        let second_project = *project_positions
            .get(1)
            .expect("focus should open a fresh surface after reduction");
        let second_terminal_reduce = events
            .iter()
            .position(|event| matches!(event, TeardownEvent::Reduce { id: 21, .. }))
            .expect("second terminal should reduce");
        assert!(
            second_terminal_reduce < second_project,
            "fresh-surface focus must not project between terminal reducers"
        );

        let (reduced, events) = terminal_batch_fixture(TeardownDispatchMode::Nested, false);
        assert_eq!(reduced_ids(&events), [20, 21, 99]);
        assert_eq!(
            reduced,
            [
                TeardownMessage { id: 20, marker: 1 },
                TeardownMessage { id: 21, marker: 2 },
                TeardownMessage { id: 99, marker: 1 },
            ]
        );

        let (reduced, events) = terminal_batch_fixture(TeardownDispatchMode::External, false);
        assert_eq!(reduced_ids(&events), [20, 21]);
        assert_eq!(
            reduced,
            [
                TeardownMessage { id: 20, marker: 1 },
                TeardownMessage { id: 21, marker: 2 },
            ]
        );
        let second_terminal_reduce = events
            .iter()
            .position(|event| matches!(event, TeardownEvent::Reduce { id: 21, .. }))
            .expect("second terminal should reduce");
        let external_work = events
            .iter()
            .position(|event| matches!(event, TeardownEvent::ExternalWork))
            .expect("external-work command should be admitted");
        assert!(
            second_terminal_reduce < external_work,
            "external work must not interleave terminal reduction"
        );
    }

    #[test]
    fn replacement_teardown_cancels_removed_and_incompatible_widgets_once() {
        for mode in [TeardownMode::Removed, TeardownMode::Incompatible] {
            let (bridge, log) = TeardownBridge::new(TeardownMode::Single(active_probe(7)));
            let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(120.0, 80.0));
            runtime.interaction.focus.owner = Some(RuntimeFocusOwner::Widget(20));
            runtime.interaction.pointer.capture = Some(20);
            runtime.interaction.pointer.capture_state = Some((20, Default::default()));
            runtime.interaction.hover.widget = Some(20);
            log.borrow_mut().events.clear();
            runtime.bridge_mut().mode = mode;

            runtime.refresh();

            assert_eq!(
                runtime.bridge().reduced,
                [TeardownMessage { id: 20, marker: 7 }]
            );
            assert_eq!(runtime.focused_widget(), None);
            assert_eq!(runtime.pointer_capture(), None);
            assert_eq!(runtime.hovered_widget(), None);
            assert_eq!(
                log.borrow().events,
                [
                    TeardownEvent::Prepare {
                        id: 20,
                        successor_authority: None,
                        projection_depth: 0,
                    },
                    TeardownEvent::Map { id: 20, marker: 7 },
                    TeardownEvent::Reduce { id: 20, marker: 7 },
                ]
            );

            runtime.refresh();
            assert_eq!(
                runtime.bridge().reduced,
                [TeardownMessage { id: 20, marker: 7 }]
            );
        }
    }

    #[test]
    fn compatible_reprojection_preserves_state_without_terminal_output() {
        let (bridge, log) = TeardownBridge::new(TeardownMode::Single(active_probe(1)));
        let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(120.0, 80.0));
        log.borrow_mut().events.clear();
        runtime.bridge_mut().mode = TeardownMode::Single(TeardownProbeConfig {
            active: false,
            marker: 9,
            ..active_probe(9)
        });

        runtime.refresh();

        assert!(runtime.bridge().reduced.is_empty());
        let active = runtime
            .surface
            .find_widget(20)
            .and_then(|widget| {
                widget
                    .widget()
                    .as_any()
                    .downcast_ref::<TeardownProbeWidget>()
            })
            .map(|widget| widget.active);
        assert_eq!(active, Some(true));
        assert!(matches!(
            log.borrow().events.as_slice(),
            [
                TeardownEvent::Prepare {
                    successor_authority: Some(true),
                    projection_depth: 0,
                    ..
                },
                TeardownEvent::Synchronize { id: 20 }
            ]
        ));
    }

    #[test]
    fn successor_authority_loss_cancels_for_authority_disabled_and_read_only_changes() {
        for (authority, disabled, read_only) in [
            (false, false, false),
            (true, true, false),
            (true, false, true),
        ] {
            let (bridge, log) = TeardownBridge::new(TeardownMode::Single(active_probe(3)));
            let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(120.0, 80.0));
            log.borrow_mut().events.clear();
            runtime.bridge_mut().mode = TeardownMode::Single(TeardownProbeConfig {
                active: false,
                authority,
                disabled,
                read_only,
                marker: 8,
                maps_output: true,
            });

            runtime.refresh();

            assert_eq!(
                runtime.bridge().reduced,
                [TeardownMessage { id: 20, marker: 3 }]
            );
            assert!(log.borrow().events.iter().any(|event| {
                matches!(
                    event,
                    TeardownEvent::Prepare {
                        successor_authority: Some(false),
                        ..
                    }
                )
            }));
            let events = log.borrow().events.clone();
            let first_reduce = events
                .iter()
                .position(|event| matches!(event, TeardownEvent::Reduce { .. }))
                .expect("terminal output should be reduced");
            assert!(
                events[..first_reduce]
                    .iter()
                    .all(|event| !matches!(event, TeardownEvent::Synchronize { .. }))
            );
        }
    }

    #[test]
    fn teardown_messages_use_old_mapper_in_previous_order_and_unmapped_teardown_still_cleans() {
        let (bridge, log) =
            TeardownBridge::new(TeardownMode::Multiple([active_probe(1), active_probe(2)]));
        let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(200.0, 80.0));
        log.borrow_mut().events.clear();
        runtime.bridge_mut().mode = TeardownMode::Multiple([
            TeardownProbeConfig {
                active: false,
                authority: false,
                marker: 9,
                ..active_probe(9)
            },
            TeardownProbeConfig {
                active: false,
                authority: false,
                marker: 9,
                ..active_probe(9)
            },
        ]);

        runtime.refresh();

        assert_eq!(
            runtime.bridge().reduced,
            [
                TeardownMessage { id: 20, marker: 1 },
                TeardownMessage { id: 21, marker: 2 },
            ]
        );
        let events = log.borrow().events.clone();
        let first_reduce = events
            .iter()
            .position(|event| matches!(event, TeardownEvent::Reduce { .. }))
            .expect("terminal output should be reduced");
        assert!(events[..first_reduce].iter().all(|event| {
            matches!(
                event,
                TeardownEvent::Prepare { .. }
                    | TeardownEvent::Map { .. }
                    | TeardownEvent::Synchronize { .. }
            )
        }));
        assert_eq!(
            &events[first_reduce..first_reduce + 2],
            &[
                TeardownEvent::Reduce { id: 20, marker: 1 },
                TeardownEvent::Reduce { id: 21, marker: 2 },
            ]
        );
        assert!(
            !events[first_reduce + 2..]
                .iter()
                .any(|event| matches!(event, TeardownEvent::Reduce { .. }))
        );

        let (bridge, log) = TeardownBridge::new(TeardownMode::Single(TeardownProbeConfig {
            maps_output: false,
            ..active_probe(4)
        }));
        let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(120.0, 80.0));
        log.borrow_mut().events.clear();
        runtime.bridge_mut().mode = TeardownMode::Removed;

        runtime.refresh();

        assert!(runtime.bridge().reduced.is_empty());
        assert!(matches!(
            log.borrow().events.as_slice(),
            [TeardownEvent::Prepare {
                successor_authority: None,
                projection_depth: 0,
                ..
            }]
        ));
    }

    #[test]
    fn replacement_plan_creation_and_drop_is_inert() {
        let (mut bridge, log) = TeardownBridge::new(TeardownMode::Single(active_probe(1)));
        let previous = bridge.surface();
        bridge.mode = TeardownMode::Single(TeardownProbeConfig {
            active: false,
            authority: false,
            marker: 9,
            ..active_probe(9)
        });
        let successor = bridge.surface();
        let previous_index = previous.runtime_traversal_index();
        let successor_index = successor.runtime_traversal_index();
        log.borrow_mut().events.clear();

        let plan = previous.plan_widget_replacements(
            &successor,
            &previous_index.stateful_widget_order,
            &previous_index.widget_paint_order,
            &successor_index.widget_paint_order,
            &successor_index.widget_paths,
            &previous_index.widget_paths,
        );
        drop(plan);

        assert!(log.borrow().events.is_empty());
        assert_eq!(
            previous
                .find_widget(20)
                .and_then(|widget| widget
                    .widget()
                    .as_any()
                    .downcast_ref::<TeardownProbeWidget>())
                .map(|widget| widget.active),
            Some(true)
        );
    }

    #[test]
    fn replacement_plan_veto_precedes_callbacks_and_immediate_fallback_retires_once() {
        let (mut bridge, log) = TeardownBridge::new(TeardownMode::Single(active_probe(1)));
        let mut previous = bridge.surface();
        bridge.mode = TeardownMode::Single(TeardownProbeConfig {
            active: false,
            authority: false,
            marker: 9,
            ..active_probe(9)
        });
        let mut successor = bridge.surface();
        let previous_index = previous.runtime_traversal_index();
        let successor_index = successor.runtime_traversal_index();
        let plan = previous.plan_widget_replacements(
            &successor,
            &previous_index.stateful_widget_order,
            &previous_index.widget_paint_order,
            &successor_index.widget_paint_order,
            &successor_index.widget_paths,
            &previous_index.widget_paths,
        );
        let Some(widget) = successor.find_widget_mut(20) else {
            panic!("successor probe exists");
        };
        widget.widget_mut().common_mut().state.disabled = true;
        log.borrow_mut().events.clear();

        let veto = previous.commit_widget_replacements(
            &successor,
            plan,
            &previous_index.widget_paint_order,
            &successor_index.widget_paint_order,
            &previous_index.widget_paths,
            &successor_index.widget_paths,
        );
        assert_eq!(veto.veto, Some(WidgetReplacementPlanVeto::StaleEvidence));
        assert!(veto.terminal_messages.is_empty());
        assert!(veto.retired_widget_ids.is_empty());
        assert!(log.borrow().events.is_empty());

        let fallback = previous.commit_widget_replacements_immediately(
            &successor,
            &previous_index.stateful_widget_order,
            &previous_index.widget_paint_order,
            &successor_index.widget_paint_order,
            &successor_index.widget_paths,
            &previous_index.widget_paths,
        );
        assert_eq!(fallback.veto, None);
        assert_eq!(fallback.retired_widget_ids, [20]);
        assert_eq!(
            fallback.terminal_messages,
            [TeardownMessage { id: 20, marker: 1 }]
        );
        assert_eq!(
            log.borrow().events,
            [
                TeardownEvent::Prepare {
                    id: 20,
                    successor_authority: None,
                    projection_depth: 0,
                },
                TeardownEvent::Map { id: 20, marker: 1 },
            ]
        );
    }

    #[test]
    fn replacement_commit_is_consuming_stable_and_uses_old_mappers_for_unmapped_output() {
        for maps_output in [true, false] {
            let (mut bridge, log) = TeardownBridge::new(TeardownMode::Multiple([
                TeardownProbeConfig {
                    maps_output,
                    ..active_probe(1)
                },
                TeardownProbeConfig {
                    maps_output,
                    ..active_probe(2)
                },
            ]));
            let mut previous = bridge.surface();
            bridge.mode = TeardownMode::Multiple([
                TeardownProbeConfig {
                    active: false,
                    authority: false,
                    marker: 9,
                    ..active_probe(9)
                },
                TeardownProbeConfig {
                    active: false,
                    authority: false,
                    marker: 9,
                    ..active_probe(9)
                },
            ]);
            let successor = bridge.surface();
            let previous_index = previous.runtime_traversal_index();
            let successor_index = successor.runtime_traversal_index();
            let plan = previous.plan_widget_replacements(
                &successor,
                &previous_index.stateful_widget_order,
                &previous_index.widget_paint_order,
                &successor_index.widget_paint_order,
                &successor_index.widget_paths,
                &previous_index.widget_paths,
            );
            log.borrow_mut().events.clear();

            let result = previous.commit_widget_replacements(
                &successor,
                plan,
                &previous_index.widget_paint_order,
                &successor_index.widget_paint_order,
                &previous_index.widget_paths,
                &successor_index.widget_paths,
            );
            assert_eq!(result.veto, None);
            assert_eq!(result.retired_widget_ids, [20, 21]);
            if maps_output {
                assert_eq!(
                    result.terminal_messages,
                    [
                        TeardownMessage { id: 20, marker: 1 },
                        TeardownMessage { id: 21, marker: 2 },
                    ]
                );
            } else {
                assert!(result.terminal_messages.is_empty());
            }
            let prepares = log
                .borrow()
                .events
                .iter()
                .filter_map(|event| match event {
                    TeardownEvent::Prepare { id, .. } => Some(*id),
                    _ => None,
                })
                .collect::<Vec<_>>();
            assert_eq!(prepares, [20, 21]);
            let mapped = log
                .borrow()
                .events
                .iter()
                .filter_map(|event| match event {
                    TeardownEvent::Map { marker, .. } => Some(*marker),
                    _ => None,
                })
                .collect::<Vec<_>>();
            assert_eq!(mapped, if maps_output { vec![1, 2] } else { Vec::new() });
        }
    }

    #[test]
    fn refresh_installs_successor_and_defers_terminal_reduction_until_returned_batch() {
        let (bridge, log) = TeardownBridge::new(TeardownMode::Single(active_probe(1)));
        let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(120.0, 80.0));
        log.borrow_mut().events.clear();
        runtime.bridge_mut().mode = TeardownMode::Single(TeardownProbeConfig {
            active: false,
            authority: false,
            marker: 9,
            ..active_probe(9)
        });

        let terminal_messages = runtime.refresh_with_scope_inner(RepaintScope::Surface);

        assert_eq!(runtime.bridge().reduced, []);
        assert_eq!(
            runtime
                .surface()
                .find_widget(20)
                .and_then(|widget| widget
                    .widget()
                    .as_any()
                    .downcast_ref::<TeardownProbeWidget>())
                .map(|widget| widget.active),
            Some(false)
        );
        assert!(!runtime.layout().rects.is_empty());
        assert!(
            !log.borrow()
                .events
                .iter()
                .any(|event| matches!(event, TeardownEvent::Reduce { .. }))
        );

        runtime.dispatch_deferred_surface_messages(terminal_messages);

        assert_eq!(
            runtime.bridge().reduced,
            [TeardownMessage { id: 20, marker: 1 }]
        );
        assert_eq!(
            log.borrow()
                .events
                .iter()
                .filter(|event| matches!(event, TeardownEvent::Reduce { .. }))
                .count(),
            1
        );
    }

    #[test]
    fn replacement_commit_passes_none_for_duplicate_or_ambiguous_evidence() {
        for ambiguous in [false, true] {
            let (mut bridge, log) = TeardownBridge::new(TeardownMode::Single(active_probe(1)));
            let mut previous = bridge.surface();
            bridge.mode = TeardownMode::Single(TeardownProbeConfig {
                active: false,
                authority: false,
                marker: 9,
                ..active_probe(9)
            });
            let successor = bridge.surface();
            let previous_index = previous.runtime_traversal_index();
            let successor_index = successor.runtime_traversal_index();
            let duplicate_order = [20, 20];
            let current_paths = if ambiguous {
                std::collections::HashMap::new()
            } else {
                successor_index.widget_paths.clone()
            };
            let plan = previous.plan_widget_replacements(
                &successor,
                &previous_index.stateful_widget_order,
                if ambiguous {
                    &previous_index.widget_paint_order
                } else {
                    &duplicate_order
                },
                if ambiguous {
                    &successor_index.widget_paint_order
                } else {
                    &duplicate_order
                },
                &current_paths,
                &previous_index.widget_paths,
            );
            log.borrow_mut().events.clear();

            let result = previous.commit_widget_replacements(
                &successor,
                plan,
                if ambiguous {
                    &previous_index.widget_paint_order
                } else {
                    &duplicate_order
                },
                if ambiguous {
                    &successor_index.widget_paint_order
                } else {
                    &duplicate_order
                },
                &previous_index.widget_paths,
                &current_paths,
            );
            assert_eq!(result.veto, None);
            assert_eq!(result.retired_widget_ids, [20]);
            assert_eq!(
                result.terminal_messages,
                [TeardownMessage { id: 20, marker: 1 }]
            );
            assert!(matches!(
                log.borrow().events.as_slice(),
                [
                    TeardownEvent::Prepare {
                        successor_authority: None,
                        ..
                    },
                    TeardownEvent::Map { marker: 1, .. }
                ]
            ));
        }
    }

    struct RefreshNumericCodec;

    impl NumericCodec<u32> for RefreshNumericCodec {
        type Error = ();

        fn parse(&self, text: &str) -> NumericParseResult<u32> {
            text.parse()
                .map_or(NumericParseResult::Invalid, NumericParseResult::Valid)
        }

        fn format_editable(
            &self,
            value: &u32,
            output: &mut dyn std::fmt::Write,
        ) -> Result<(), Self::Error> {
            write!(output, "{value}").map_err(|_| ())
        }
    }

    struct RefreshNumericAdjustment;

    impl NumericAdjustment<u32> for RefreshNumericAdjustment {
        type Error = ();

        fn normalized_to_value(&self, normalized: f32) -> Result<u32, Self::Error> {
            Ok((normalized * 100.0).round() as u32)
        }

        fn value_to_normalized(&self, value: &u32) -> Result<f32, Self::Error> {
            Ok(*value as f32 / 100.0)
        }

        fn step(
            &self,
            value: &u32,
            _direction: NumericStepDirection,
            _step: NumericStep,
        ) -> Result<u32, Self::Error> {
            Ok(*value)
        }

        fn scrub(
            &self,
            value: &u32,
            _normalized_delta: f32,
            _step: NumericStep,
        ) -> Result<u32, Self::Error> {
            Ok(*value)
        }

        fn wheel(&self, value: &u32, _delta: f32, _step: NumericStep) -> Result<u32, Self::Error> {
            Ok(*value)
        }
    }

    struct NumericRefreshBridge {
        value: u32,
        reduced: Vec<NumericInputEditBatch<u32>>,
    }

    impl NumericRefreshBridge {
        fn surface(&self) -> UiSurface<NumericInputEditBatch<u32>> {
            let input = NumericInputWidget::try_new(
                self.value,
                RefreshNumericCodec,
                RefreshNumericAdjustment,
                WidgetSizing::fixed(Vector2::new(120.0, 28.0)),
            )
            .expect("numeric refresh fixture should construct");
            let node = SurfaceNode::widget(
                input,
                WidgetMessageMapper::typed(|batch: NumericInputEditBatch<u32>| batch),
            )
            .with_id(20);
            UiSurface::new(node)
        }
    }

    impl RuntimeBridge<NumericInputEditBatch<u32>> for NumericRefreshBridge {
        fn project_surface(&mut self) -> Arc<UiSurface<NumericInputEditBatch<u32>>> {
            crate::runtime::test_arc_surface(self.surface())
        }

        fn reduce_message(&mut self, message: NumericInputEditBatch<u32>) {
            self.reduced.push(message);
        }
    }

    #[test]
    fn numeric_refresh_maps_retiring_cancel_and_does_not_sync_successor_state() {
        let bridge = NumericRefreshBridge {
            value: 7,
            reduced: Vec::new(),
        };
        let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(120.0, 80.0));
        runtime.refresh();
        assert!(runtime.focus_widget(20));
        let _ = runtime.dispatch_focused_input(WidgetInput::text_edit(TextEditCommand::SelectAll));
        let _ = runtime.dispatch_focused_input(WidgetInput::text_edit(
            TextEditCommand::InsertText("8".to_owned()),
        ));

        runtime.bridge_mut().value = 9;
        runtime.refresh();

        assert_eq!(runtime.bridge().reduced.len(), 1);
        let batch = &runtime.bridge().reduced[0];
        assert_eq!(
            batch
                .events()
                .iter()
                .map(|event| event.phase)
                .collect::<Vec<_>>(),
            [EditPhase::Begin, EditPhase::Cancel]
        );
        assert_eq!(batch.events()[0].transaction, batch.events()[1].transaction);
        assert_eq!(batch.events()[0].start_value, 7);
        assert_eq!(batch.events()[1].value, 7);
        assert_eq!(
            batch.events()[1].provenance,
            InteractionProvenance::Keyboard { timestamp: None }
        );
        let successor = runtime
            .surface
            .find_widget(20)
            .expect("numeric successor should remain installed")
            .widget();
        assert_eq!(
            successor.automation_semantics().value_text,
            Some("9".to_owned())
        );
        assert!(!successor.preempts_host_shortcut_key(WidgetKey::Escape));
        assert_eq!(runtime.focused_widget(), None);
    }

    struct RefreshKnobDomainAdjustment;

    impl NumericAdjustment<f32> for RefreshKnobDomainAdjustment {
        type Error = ();

        fn normalized_to_value(&self, normalized: f32) -> Result<f32, Self::Error> {
            Ok(normalized * 100.0)
        }

        fn value_to_normalized(&self, value: &f32) -> Result<f32, Self::Error> {
            Ok(*value / 100.0)
        }

        fn step(
            &self,
            value: &f32,
            _direction: NumericStepDirection,
            _step: NumericStep,
        ) -> Result<f32, Self::Error> {
            Ok(*value)
        }

        fn scrub(
            &self,
            value: &f32,
            _normalized_delta: f32,
            _step: NumericStep,
        ) -> Result<f32, Self::Error> {
            Ok(*value)
        }

        fn wheel(&self, value: &f32, _delta: f32, _step: NumericStep) -> Result<f32, Self::Error> {
            Ok(*value)
        }
    }

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    enum KnobDomainRefreshMode {
        Editable,
        Disabled,
        ReadOnly,
    }

    struct KnobDomainRefreshBridge {
        value: f32,
        mode: KnobDomainRefreshMode,
        reduced: Vec<KnobDomainMessage<()>>,
    }

    impl KnobDomainRefreshBridge {
        fn surface(&self) -> UiSurface<KnobDomainMessage<()>> {
            let mut knob = KnobWidget::new(20, self.value / 100.0)
                .with_default_value(0.2)
                .with_sensitivity(0.01);
            knob.common.state.disabled = self.mode == KnobDomainRefreshMode::Disabled;
            knob.common.state.read_only = self.mode == KnobDomainRefreshMode::ReadOnly;
            let widget = RetainedKnobDomainWidget::new(
                knob,
                Rc::new(RefreshKnobDomainAdjustment),
                self.value,
                20.0,
                0.2,
            );
            UiSurface::new(SurfaceNode::widget(
                widget,
                WidgetMessageMapper::typed(|message: KnobDomainMessage<()>| message),
            ))
        }
    }

    impl RuntimeBridge<KnobDomainMessage<()>> for KnobDomainRefreshBridge {
        fn project_surface(&mut self) -> Arc<UiSurface<KnobDomainMessage<()>>> {
            crate::runtime::test_arc_surface(self.surface())
        }

        fn reduce_message(&mut self, message: KnobDomainMessage<()>) {
            match &message {
                KnobDomainMessage::ValueChanged { value, .. }
                | KnobDomainMessage::GestureEnded { value, .. }
                | KnobDomainMessage::Reset { value, .. } => self.value = *value,
                KnobDomainMessage::GestureCancelled { start_value, .. } => {
                    self.value = *start_value
                }
                _ => {}
            }
            self.reduced.push(message);
        }
    }

    fn exercise_knob_domain_authority_loss(mode: KnobDomainRefreshMode) {
        let mut runtime = SurfaceRuntime::new(
            KnobDomainRefreshBridge {
                value: 20.0,
                mode: KnobDomainRefreshMode::Editable,
                reduced: Vec::new(),
            },
            Vector2::new(40.0, 40.0),
        );
        runtime.refresh();

        let start = Point::new(20.0, 20.0);
        let moved = Point::new(20.0, 10.0);
        assert_eq!(
            runtime.dispatch_event(Event::primary_press(start)),
            Some(20)
        );
        assert_eq!(runtime.pointer_capture(), Some(20));
        assert_eq!(runtime.focused_widget(), Some(20));
        assert_eq!(runtime.dispatch_event(Event::pointer_move(moved)), Some(20));
        assert_eq!(runtime.pointer_capture(), Some(20));
        assert!((runtime.bridge().value - 30.0).abs() < 0.0001);

        runtime.bridge_mut().mode = mode;
        runtime.refresh();

        let cancellations = runtime
            .bridge()
            .reduced
            .iter()
            .filter_map(|message| match message {
                KnobDomainMessage::GestureCancelled {
                    start_value,
                    previous_value,
                    reason,
                    metadata,
                } => Some((*start_value, *previous_value, *reason, *metadata)),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(cancellations.len(), 1);
        let (start_value, previous_value, reason, metadata) = cancellations[0];
        assert!((start_value - 20.0).abs() < 0.0001);
        assert!((previous_value - 30.0).abs() < 0.0001);
        assert_eq!(reason, KnobDomainCancellationReason::DisabledOrReadOnly);
        assert_eq!(metadata, KnobPointerMetadata::empty());
        assert!((runtime.bridge().value - 20.0).abs() < 0.0001);
        assert_eq!(runtime.pointer_capture(), None);
        assert_eq!(runtime.focused_widget(), None);

        let successor = runtime
            .surface
            .find_widget(20)
            .expect("domain successor should remain installed")
            .widget()
            .as_any()
            .downcast_ref::<RetainedKnobDomainWidget<RefreshKnobDomainAdjustment>>()
            .expect("domain successor should retain its concrete wrapper");
        assert!(
            (successor.domain_value - 20.0).abs() < 0.0001,
            "successor domain value was {} while durable value is {}",
            successor.domain_value,
            runtime.bridge().value
        );
        assert!((successor.knob.state.value - 0.2).abs() < 0.0001);
        assert_eq!(
            successor.knob.common.state.disabled,
            mode == KnobDomainRefreshMode::Disabled
        );
        assert_eq!(
            successor.knob.common.state.read_only,
            mode == KnobDomainRefreshMode::ReadOnly
        );

        runtime.refresh();
        let _ = runtime.dispatch_event(Event::primary_release(moved));
        assert_eq!(
            runtime
                .bridge()
                .reduced
                .iter()
                .filter(|message| matches!(message, KnobDomainMessage::GestureCancelled { .. }))
                .count(),
            1
        );
        assert_eq!(runtime.pointer_capture(), None);
        assert_eq!(runtime.focused_widget(), None);
    }

    #[test]
    fn knob_domain_disabled_reprojection_cancels_active_gesture_once() {
        exercise_knob_domain_authority_loss(KnobDomainRefreshMode::Disabled);
    }

    #[test]
    fn knob_domain_read_only_reprojection_cancels_active_gesture_once() {
        exercise_knob_domain_authority_loss(KnobDomainRefreshMode::ReadOnly);
    }

    #[derive(Clone, Copy)]
    enum NumericRefreshOutputMode {
        Compatibility,
        Complete,
    }

    // This test-only mode fixture intentionally carries the complete-mode batch
    // so refresh tests can inspect both output modes through one bridge.
    #[allow(clippy::large_enum_variant)]
    enum NumericRefreshMessage {
        Compatibility(NumericInputEditBatch<u32>),
        Complete(NumericInputInteractionBatch<u32, (), ()>),
    }

    struct NumericModeRefreshBridge {
        value: u32,
        mode: NumericRefreshOutputMode,
        reduced: Vec<NumericRefreshMessage>,
        compatibility_mapper_calls: Rc<Cell<usize>>,
        complete_mapper_calls: Rc<Cell<usize>>,
    }

    impl NumericModeRefreshBridge {
        fn surface(&self) -> UiSurface<NumericRefreshMessage> {
            let mut input = NumericInputWidget::try_new(
                self.value,
                RefreshNumericCodec,
                RefreshNumericAdjustment,
                WidgetSizing::fixed(Vector2::new(120.0, 28.0)),
            )
            .expect("numeric mode refresh fixture should construct");
            let messages = match self.mode {
                NumericRefreshOutputMode::Compatibility => {
                    let calls = Rc::clone(&self.compatibility_mapper_calls);
                    WidgetMessageMapper::typed(move |batch: NumericInputEditBatch<u32>| {
                        calls.set(calls.get() + 1);
                        NumericRefreshMessage::Compatibility(batch)
                    })
                }
                NumericRefreshOutputMode::Complete => {
                    input.set_complete_output_mode();
                    let calls = Rc::clone(&self.complete_mapper_calls);
                    WidgetMessageMapper::typed(
                        move |batch: NumericInputInteractionBatch<u32, (), ()>| {
                            calls.set(calls.get() + 1);
                            NumericRefreshMessage::Complete(batch)
                        },
                    )
                }
            };
            UiSurface::new(SurfaceNode::widget(input, messages).with_id(21))
        }
    }

    impl RuntimeBridge<NumericRefreshMessage> for NumericModeRefreshBridge {
        fn project_surface(&mut self) -> Arc<UiSurface<NumericRefreshMessage>> {
            crate::runtime::test_arc_surface(self.surface())
        }

        fn reduce_message(&mut self, message: NumericRefreshMessage) {
            self.reduced.push(message);
        }
    }

    fn numeric_mode_refresh_message(
        old_mode: NumericRefreshOutputMode,
        new_mode: NumericRefreshOutputMode,
    ) -> (NumericRefreshMessage, usize, usize, bool) {
        let bridge = NumericModeRefreshBridge {
            value: 7,
            mode: old_mode,
            reduced: Vec::new(),
            compatibility_mapper_calls: Rc::new(Cell::new(0)),
            complete_mapper_calls: Rc::new(Cell::new(0)),
        };
        let compatibility_mapper_calls = Rc::clone(&bridge.compatibility_mapper_calls);
        let complete_mapper_calls = Rc::clone(&bridge.complete_mapper_calls);
        let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(120.0, 80.0));
        runtime.refresh();
        assert!(runtime.focus_widget(21));
        assert_eq!(
            runtime.dispatch_focused_input(WidgetInput::text_edit(TextEditCommand::SelectAll,)),
            Some(21)
        );
        assert_eq!(
            runtime.dispatch_focused_input(WidgetInput::text_edit(TextEditCommand::InsertText(
                "8".to_owned(),
            ))),
            Some(21)
        );

        runtime.bridge_mut().mode = new_mode;
        runtime.refresh();

        let message = runtime
            .bridge_mut()
            .reduced
            .pop()
            .expect("mode change should retire one active numeric edit");
        let successor_active = runtime
            .surface
            .find_widget(21)
            .expect("numeric successor should remain installed")
            .widget()
            .preempts_host_shortcut_key(WidgetKey::Escape);
        (
            message,
            compatibility_mapper_calls.get(),
            complete_mapper_calls.get(),
            successor_active,
        )
    }

    #[test]
    fn numeric_refresh_mode_change_uses_only_retiring_mapper_in_both_directions() {
        let (complete_to_compatibility, compatibility_calls, complete_calls, successor_active) =
            numeric_mode_refresh_message(
                NumericRefreshOutputMode::Complete,
                NumericRefreshOutputMode::Compatibility,
            );
        let NumericRefreshMessage::Complete(batch) = complete_to_compatibility else {
            panic!("complete retiring widget must use the complete mapper");
        };
        let [NumericInputInteraction::Edit(edit)] = batch.parts() else {
            panic!("complete retirement must contain one outer Edit");
        };
        assert_eq!(
            edit.events()
                .iter()
                .map(|event| event.phase)
                .collect::<Vec<_>>(),
            [EditPhase::Begin, EditPhase::Cancel]
        );
        assert_eq!(compatibility_calls, 0);
        assert_eq!(complete_calls, 1);
        assert!(!successor_active);

        let (compatibility_to_complete, compatibility_calls, complete_calls, successor_active) =
            numeric_mode_refresh_message(
                NumericRefreshOutputMode::Compatibility,
                NumericRefreshOutputMode::Complete,
            );
        let NumericRefreshMessage::Compatibility(batch) = compatibility_to_complete else {
            panic!("compatibility retiring widget must use the compatibility mapper");
        };
        assert_eq!(
            batch
                .events()
                .iter()
                .map(|event| event.phase)
                .collect::<Vec<_>>(),
            [EditPhase::Begin, EditPhase::Cancel]
        );
        assert_eq!(compatibility_calls, 1);
        assert_eq!(complete_calls, 0);
        assert!(!successor_active);
    }

    #[derive(Clone, Copy)]
    enum LayoutCapabilityMode {
        Exact(&'static str),
        Conservative,
        Incompatible,
    }

    struct RefreshLayoutInteraction {
        revision: LayoutInteractionRevision,
    }

    impl LayoutInteraction<()> for RefreshLayoutInteraction {
        fn revision(&self) -> LayoutInteractionRevision {
            self.revision.clone()
        }
    }

    struct LayoutCapabilityBridge {
        mode: LayoutCapabilityMode,
    }

    impl LayoutCapabilityBridge {
        fn surface(&self) -> UiSurface<()> {
            let revision = match self.mode {
                LayoutCapabilityMode::Exact(value) => LayoutInteractionRevision::exact(value),
                LayoutCapabilityMode::Conservative | LayoutCapabilityMode::Incompatible => {
                    LayoutInteractionRevision::conservative()
                }
            };
            let mut capabilities =
                LayoutCapabilities::new().interaction_local(RefreshLayoutInteraction { revision });
            if matches!(self.mode, LayoutCapabilityMode::Incompatible) {
                capabilities.contract_version = LAYOUT_CAPABILITIES_CONTRACT_VERSION + 1;
            }
            UiSurface::new(
                SurfaceNode::container(1, ContainerPolicy::default(), Vec::new())
                    .with_layout_capabilities(capabilities),
            )
        }
    }

    impl RuntimeBridge<()> for LayoutCapabilityBridge {
        fn project_surface(&mut self) -> Arc<UiSurface<()>> {
            crate::runtime::test_arc_surface(self.surface())
        }

        fn pull_surface(&mut self) -> UiSurface<()> {
            self.surface()
        }
    }

    struct LayoutTargetInteraction {
        regions: Vec<LayoutHitRegion>,
    }

    impl LayoutInteraction<()> for LayoutTargetInteraction {
        fn revision(&self) -> LayoutInteractionRevision {
            LayoutInteractionRevision::exact("layout-targets")
        }

        fn visit_hit_regions(&self, _local_bounds: Rect, visitor: &mut dyn FnMut(LayoutHitRegion)) {
            for region in &self.regions {
                visitor(*region);
            }
        }
    }

    fn layout_region(id: u64, min_x: f32, max_x: f32) -> LayoutHitRegion {
        LayoutHitRegion::new(
            LayoutHitRegionId::new(id),
            Rect::from_min_max(Point::new(min_x, 0.0), Point::new(max_x, 1.0)),
        )
        .expect("test region should be valid")
    }

    struct LayoutTargetBridge {
        incompatible: bool,
        projection_only: bool,
    }

    impl LayoutTargetBridge {
        fn capabilities(
            regions: Vec<LayoutHitRegion>,
            incompatible: bool,
            projection_only: bool,
        ) -> LayoutCapabilities<()> {
            let mut capabilities =
                LayoutCapabilities::new().interaction_local(LayoutTargetInteraction { regions });
            if incompatible {
                capabilities.contract_version = LAYOUT_CAPABILITIES_CONTRACT_VERSION + 1;
            } else if projection_only {
                capabilities.contract_version = LAYOUT_CAPABILITIES_PROJECTION_CONTRACT_VERSION;
            }
            capabilities
        }

        fn surface(&self) -> UiSurface<()> {
            let mut inner_regions = (0..12)
                .map(|index| {
                    layout_region(100 + index, index as f32 / 12.0, (index + 1) as f32 / 12.0)
                })
                .collect::<Vec<_>>();
            inner_regions.push(layout_region(100, 0.9, 1.0));
            let inner = SurfaceNode::container(
                2,
                ContainerPolicy {
                    kind: ContainerKind::Stack,
                    ..ContainerPolicy::default()
                },
                Vec::new(),
            )
            .with_layout_capabilities(Self::capabilities(
                inner_regions,
                self.incompatible,
                self.projection_only,
            ));
            let outer = SurfaceNode::container(
                1,
                ContainerPolicy {
                    kind: ContainerKind::Stack,
                    ..ContainerPolicy::default()
                },
                vec![SurfaceChild::fill(inner)],
            )
            .with_layout_capabilities(Self::capabilities(
                vec![layout_region(900, 0.0, 1.0)],
                self.incompatible,
                self.projection_only,
            ));
            UiSurface::new(outer)
        }
    }

    impl RuntimeBridge<()> for LayoutTargetBridge {
        fn project_surface(&mut self) -> Arc<UiSurface<()>> {
            crate::runtime::test_arc_surface(self.surface())
        }

        fn pull_surface(&mut self) -> UiSurface<()> {
            self.surface()
        }
    }

    struct ClippedLayoutTargetBridge;

    impl RuntimeBridge<()> for ClippedLayoutTargetBridge {
        fn project_surface(&mut self) -> Arc<UiSurface<()>> {
            crate::runtime::test_arc_surface(self.surface())
        }

        fn pull_surface(&mut self) -> UiSurface<()> {
            self.surface()
        }
    }

    impl ClippedLayoutTargetBridge {
        fn surface(&self) -> UiSurface<()> {
            let content = SurfaceNode::container(
                11,
                ContainerPolicy {
                    kind: ContainerKind::Stack,
                    ..ContainerPolicy::default()
                },
                vec![SurfaceChild::fill(SurfaceNode::widget(
                    TextWidget::new(12, "wide", WidgetSizing::fixed(Vector2::new(200.0, 80.0))),
                    WidgetMessageMapper::none(),
                ))],
            )
            .with_layout_capabilities(LayoutTargetBridge::capabilities(
                vec![layout_region(11, 0.0, 1.0)],
                false,
                false,
            ));
            UiSurface::new(SurfaceNode::container(
                10,
                ContainerPolicy {
                    kind: ContainerKind::ScrollView,
                    overflow: OverflowPolicy::Scroll,
                    ..ContainerPolicy::default()
                },
                vec![SurfaceChild::new(SlotParams::fill(), content)],
            ))
        }
    }

    struct OwnClipLayoutTargetBridge;

    impl RuntimeBridge<()> for OwnClipLayoutTargetBridge {
        fn project_surface(&mut self) -> Arc<UiSurface<()>> {
            crate::runtime::test_arc_surface(self.surface())
        }

        fn pull_surface(&mut self) -> UiSurface<()> {
            self.surface()
        }
    }

    impl OwnClipLayoutTargetBridge {
        fn surface(&self) -> UiSurface<()> {
            UiSurface::new(
                SurfaceNode::container(
                    10,
                    ContainerPolicy {
                        kind: ContainerKind::ScrollView,
                        overflow: OverflowPolicy::Scroll,
                        padding: crate::layout::Insets::all(4.0),
                        ..ContainerPolicy::default()
                    },
                    vec![SurfaceChild::fill(SurfaceNode::widget(
                        TextWidget::new(
                            11,
                            "content",
                            WidgetSizing::fixed(Vector2::new(40.0, 20.0)),
                        ),
                        WidgetMessageMapper::none(),
                    ))],
                )
                .with_layout_capabilities(LayoutTargetBridge::capabilities(
                    vec![layout_region(10, 0.0, 1.0)],
                    false,
                    false,
                )),
            )
        }
    }

    struct NoLayoutCapabilityBridge;

    impl RuntimeBridge<()> for NoLayoutCapabilityBridge {
        fn project_surface(&mut self) -> Arc<UiSurface<()>> {
            crate::runtime::test_arc_surface(UiSurface::new(SurfaceNode::widget(
                ButtonWidget::new(20, "plain", WidgetSizing::fixed(Vector2::new(80.0, 28.0))),
                WidgetMessageMapper::none(),
            )))
        }
    }

    #[test]
    fn layout_targets_project_all_regions_in_traversal_order_and_first_duplicate_wins() {
        let runtime = SurfaceRuntime::new(
            LayoutTargetBridge {
                incompatible: false,
                projection_only: false,
            },
            Vector2::new(120.0, 80.0),
        );

        let target = runtime
            .layout_target_at(Point::new(115.0, 40.0))
            .expect("the twelfth region must not be truncated");
        assert_eq!(target.container_id, 2);
        assert_eq!(target.region_id, LayoutHitRegionId::new(111));
        assert_eq!(
            target.bounds,
            Rect::from_min_max(
                Point::new(120.0 * (11.0 / 12.0), 0.0),
                Point::new(120.0, 80.0),
            )
        );

        let first = runtime
            .layout_target_at(Point::new(1.0, 40.0))
            .expect("the first region should remain projected");
        assert_eq!(first.container_id, 2);
        assert_eq!(first.region_id, LayoutHitRegionId::new(100));
        assert_eq!(
            runtime
                .layout_hit_region_diagnostics()
                .duplicate_declarations(),
            1
        );

        let nested = runtime
            .layout_target_at(Point::new(60.0, 40.0))
            .expect("nested target should overlap the outer target");
        assert_eq!(nested.container_id, 2, "nested traversal target is topmost");
    }

    #[test]
    fn layout_targets_reproject_on_full_viewport_and_reused_projection_paths() {
        let mut runtime = SurfaceRuntime::new(
            LayoutTargetBridge {
                incompatible: false,
                projection_only: false,
            },
            Vector2::new(120.0, 80.0),
        );
        let before = runtime.refresh_counters();
        runtime.refresh_with_scope(RepaintScope::Projection);
        assert_eq!(runtime.refresh_counters().layout, before.layout);
        assert_eq!(
            runtime
                .layout_target_at(Point::new(115.0, 40.0))
                .map(|target| target.region_id),
            Some(LayoutHitRegionId::new(111))
        );

        runtime.refresh();
        assert_eq!(
            runtime
                .layout_target_at(Point::new(115.0, 40.0))
                .map(|target| target.container_id),
            Some(2)
        );

        runtime.set_viewport(Vector2::new(240.0, 80.0));
        let target = runtime
            .layout_target_at(Point::new(230.0, 40.0))
            .expect("viewport relayout should reproject current bounds");
        assert_eq!(target.region_id, LayoutHitRegionId::new(111));
        assert_eq!(target.bounds.max.x, 240.0);
    }

    #[test]
    fn layout_targets_respect_scroll_clips_and_unsupported_capabilities_are_ignored() {
        let clipped = SurfaceRuntime::new(ClippedLayoutTargetBridge, Vector2::new(100.0, 50.0));
        let visible = clipped
            .layout_target_at(Point::new(50.0, 25.0))
            .expect("target inside the scroll viewport");
        assert_eq!(visible.container_id, 11);
        assert_eq!(visible.bounds.max.x, 100.0);
        assert_eq!(visible.bounds.max.y, 50.0);
        assert!(
            clipped.layout_target_at(Point::new(150.0, 25.0)).is_none(),
            "content outside its scroll viewport must be excluded"
        );

        let own_clip = SurfaceRuntime::new(OwnClipLayoutTargetBridge, Vector2::new(100.0, 50.0));
        assert!(own_clip.layout_target_at(Point::new(2.0, 2.0)).is_none());
        let own_visible = own_clip
            .layout_target_at(Point::new(5.0, 5.0))
            .expect("own scroll viewport should retain its interior");
        assert_eq!(own_visible.container_id, 10);
        assert_eq!(own_visible.bounds.min, Point::new(4.0, 4.0));

        let mut unsupported = SurfaceRuntime::new(
            LayoutTargetBridge {
                incompatible: true,
                projection_only: false,
            },
            Vector2::new(120.0, 80.0),
        );
        assert!(
            unsupported
                .layout_target_at(Point::new(60.0, 40.0))
                .is_none()
        );
        assert_eq!(
            unsupported
                .layout_hit_region_diagnostics()
                .duplicate_declarations(),
            0
        );
        unsupported.bridge_mut().incompatible = true;
        unsupported.refresh();
        assert!(
            unsupported
                .layout_target_at(Point::new(60.0, 40.0))
                .is_none()
        );
        unsupported.bridge_mut().incompatible = false;
        unsupported.refresh();
        assert!(
            unsupported
                .layout_target_at(Point::new(115.0, 40.0))
                .is_some()
        );
    }

    #[test]
    fn projection_only_capability_version_two_remains_queryable() {
        let runtime = SurfaceRuntime::new(
            LayoutTargetBridge {
                incompatible: false,
                projection_only: true,
            },
            Vector2::new(120.0, 80.0),
        );

        assert!(runtime.layout_target_at(Point::new(60.0, 40.0)).is_some());
    }

    #[test]
    fn layout_target_query_is_observational_and_no_capability_keeps_widget_hit_testing() {
        let mut runtime = SurfaceRuntime::new(
            LayoutTargetBridge {
                incompatible: false,
                projection_only: false,
            },
            Vector2::new(120.0, 80.0),
        );
        runtime.interaction.focus.owner = Some(RuntimeFocusOwner::Widget(999));
        runtime.interaction.hover.container = Some(1);
        runtime.interaction.pointer.current_position = Some(Point::new(8.0, 8.0));
        runtime.interaction.pointer.capture = Some(999);
        runtime.repaint_requested = true;
        let before = (
            runtime.interaction.focus,
            runtime.interaction.hover,
            runtime.interaction.pointer,
            runtime.refresh_counters(),
            runtime.repaint_requested,
            runtime.base_paint_plan_reuse_eligible(),
            runtime.last_refresh_diagnostics(),
        );

        let _ = runtime.layout_target_at(Point::new(60.0, 40.0));

        assert_eq!(runtime.interaction.focus, before.0);
        assert_eq!(runtime.interaction.hover, before.1);
        assert_eq!(runtime.interaction.pointer, before.2);
        assert_eq!(runtime.refresh_counters(), before.3);
        assert_eq!(runtime.repaint_requested, before.4);
        assert_eq!(
            runtime.base_paint_plan_reuse_eligible(),
            before.5,
            "target inspection must not alter reuse authority"
        );
        assert_eq!(runtime.last_refresh_diagnostics(), before.6);

        let plain = SurfaceRuntime::new(NoLayoutCapabilityBridge, Vector2::new(100.0, 40.0));
        assert!(plain.layout_target_at(Point::new(20.0, 14.0)).is_none());
        assert_eq!(plain.widget_at(Point::new(20.0, 14.0)), Some(20));
    }

    fn replacement_widget(id: u64, replace: bool) -> SurfaceNode<()> {
        if replace {
            SurfaceNode::widget(
                ScrollbarWidget::new(
                    id,
                    ScrollbarAxis::Vertical,
                    WidgetSizing::fixed(Vector2::new(16.0, 80.0)),
                ),
                WidgetMessageMapper::none(),
            )
        } else {
            SurfaceNode::widget(
                ButtonWidget::new(
                    id,
                    "Previous",
                    WidgetSizing::fixed(Vector2::new(80.0, 28.0)),
                ),
                WidgetMessageMapper::none(),
            )
        }
    }

    #[test]
    fn incompatible_replacement_discards_controller_ownership_and_reports_identity() {
        let mut runtime =
            SurfaceRuntime::new(ReplacementBridge::default(), Vector2::new(120.0, 80.0));
        runtime.interaction.focus.owner = Some(RuntimeFocusOwner::Widget(20));
        runtime.interaction.pointer.capture = Some(20);
        runtime.interaction.pointer.capture_state = Some((20, Default::default()));
        runtime.interaction.hover.widget = Some(20);
        runtime.bridge_mut().replace = true;

        runtime.refresh();

        assert_eq!(runtime.focused_widget(), None);
        assert_eq!(runtime.pointer_capture(), None);
        assert_eq!(runtime.hovered_widget(), None);
        assert_eq!(runtime.interaction.pointer.capture_state, None);
        let diagnostics = runtime.last_refresh_diagnostics().identity;
        assert_eq!(diagnostics.replacement_count, 1);
        let replacement = diagnostics.replacements[0].expect("replacement diagnostic");
        assert_eq!(replacement.widget_id, 20);
        assert_ne!(replacement.previous_kind, replacement.current_kind);
        assert_eq!(replacement.previous_path.as_slice(), &[] as &[usize]);
        assert_eq!(replacement.current_path.as_slice(), &[] as &[usize]);
        assert_eq!(
            replacement.discarded_ownership,
            SurfaceIdentityOwnership {
                focus: true,
                pointer_capture: true,
                hover: true,
                widget_state: true,
            }
        );
    }

    #[derive(Clone)]
    struct MutableCompatibilityWidget {
        common: crate::widgets::WidgetCommon,
        changed: Rc<Cell<bool>>,
    }

    impl MutableCompatibilityWidget {
        fn new(changed: Rc<Cell<bool>>) -> Self {
            Self {
                common: crate::widgets::WidgetCommon::fixed(20, 80.0, 28.0),
                changed,
            }
        }
    }

    impl crate::widgets::Widget for MutableCompatibilityWidget {
        fn compatibility_kind(&self) -> &'static str {
            if self.changed.get() {
                "test::MutableCompatibilityWidget::changed"
            } else {
                "test::MutableCompatibilityWidget::base"
            }
        }

        fn revision(&self) -> crate::widgets::WidgetRevision {
            crate::widgets::WidgetRevision::exact((), (), (), ())
        }

        fn common(&self) -> &crate::widgets::WidgetCommon {
            &self.common
        }

        fn common_mut(&mut self) -> &mut crate::widgets::WidgetCommon {
            &mut self.common
        }

        fn handle_input(
            &mut self,
            _bounds: crate::gui::types::Rect,
            _input: crate::widgets::WidgetInput,
        ) -> Option<crate::widgets::WidgetOutput> {
            None
        }

        fn append_paint(
            &self,
            _primitives: &mut Vec<crate::runtime::PaintPrimitive>,
            _bounds: crate::gui::types::Rect,
            _layout: &crate::layout::LayoutOutput,
            _theme: &crate::theme::ThemeTokens,
        ) {
        }
    }

    struct MutableCompatibilityBridge {
        surface: UiSurface<()>,
    }

    impl MutableCompatibilityBridge {
        fn new(changed: Rc<Cell<bool>>) -> Self {
            Self {
                surface: UiSurface::new(SurfaceNode::widget(
                    MutableCompatibilityWidget::new(changed),
                    WidgetMessageMapper::none(),
                )),
            }
        }
    }

    impl RuntimeBridge<()> for MutableCompatibilityBridge {
        fn project_surface(&mut self) -> Arc<UiSurface<()>> {
            crate::runtime::test_arc_surface(self.surface.clone())
        }

        fn pull_surface(&mut self) -> UiSurface<()> {
            self.surface.clone()
        }
    }

    #[test]
    fn invalidated_same_id_compatibility_discards_all_controller_ownership() {
        let changed = Rc::new(Cell::new(false));
        let mut runtime = SurfaceRuntime::new(
            MutableCompatibilityBridge::new(Rc::clone(&changed)),
            Vector2::new(120.0, 80.0),
        );
        runtime.interaction.focus.owner = Some(RuntimeFocusOwner::Widget(20));
        runtime.interaction.pointer.capture = Some(20);
        runtime.interaction.pointer.capture_state = Some((20, Default::default()));
        runtime.interaction.hover.widget = Some(20);

        changed.set(true);
        let Some(widget) = runtime.surface.find_widget_mut(20) else {
            panic!("mutable compatibility widget exists");
        };
        widget.widget_mut().common_mut().state.hovered = true;

        runtime.refresh();

        assert_eq!(runtime.focused_widget(), None);
        assert_eq!(runtime.pointer_capture(), None);
        assert_eq!(runtime.hovered_widget(), None);
        assert_eq!(runtime.interaction.pointer.capture_state, None);
        assert_eq!(
            runtime
                .last_refresh_diagnostics()
                .identity
                .replacement_count,
            1
        );
        let replacement = runtime.last_refresh_diagnostics().identity.replacements[0];
        assert!(
            replacement.is_some_and(|replacement| {
                replacement.previous_kind != replacement.current_kind
            })
        );
        assert_eq!(
            replacement.map(|replacement| replacement.discarded_ownership),
            Some(SurfaceIdentityOwnership {
                focus: true,
                pointer_capture: true,
                hover: true,
                widget_state: true,
            })
        );
    }

    struct ReidentifiedWidgetBridge;

    impl RuntimeBridge<()> for ReidentifiedWidgetBridge {
        fn project_surface(&mut self) -> Arc<UiSurface<()>> {
            let widget = SurfaceNode::widget(
                ButtonWidget::new(7, "Stable", WidgetSizing::fixed(Vector2::new(80.0, 28.0))),
                WidgetMessageMapper::none(),
            )
            .with_id(20);
            crate::runtime::test_arc_surface(UiSurface::new(widget))
        }
    }

    #[test]
    fn projection_reidentification_preserves_retained_ownership_across_refreshes() {
        let mut runtime = SurfaceRuntime::new(ReidentifiedWidgetBridge, Vector2::new(120.0, 80.0));
        runtime.interaction.focus.owner = Some(RuntimeFocusOwner::Widget(20));
        runtime.interaction.pointer.capture = Some(20);
        runtime.interaction.pointer.capture_state = Some((20, Default::default()));
        runtime.interaction.hover.widget = Some(20);

        runtime.refresh();
        assert_eq!(runtime.focused_widget(), Some(20));
        assert_eq!(runtime.pointer_capture(), Some(20));
        assert_eq!(runtime.hovered_widget(), Some(20));

        runtime.refresh();
        assert_eq!(runtime.focused_widget(), Some(20));
        assert_eq!(runtime.pointer_capture(), Some(20));
        assert_eq!(runtime.hovered_widget(), Some(20));
        assert_eq!(
            runtime
                .last_refresh_diagnostics()
                .identity
                .replacement_count,
            0
        );
    }

    #[test]
    fn strict_identity_audit_panics_after_committing_cleanup_and_diagnostics() {
        let mut runtime =
            SurfaceRuntime::new(ReplacementBridge::default(), Vector2::new(120.0, 80.0));
        runtime.set_identity_audit(IdentityAudit::strict());
        runtime.interaction.focus.owner = Some(RuntimeFocusOwner::Widget(20));
        runtime.interaction.pointer.capture = Some(20);
        runtime.interaction.pointer.capture_state = Some((20, Default::default()));
        runtime.interaction.hover.widget = Some(20);
        runtime.bridge_mut().replace = true;

        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| runtime.refresh()));
        let payload = result.expect_err("strict identity audit should fail");
        let message = payload
            .downcast_ref::<String>()
            .expect("strict identity audit should use a String payload");
        assert!(message.starts_with("radiant identity audit: strict mode"));
        assert!(message.contains("replacement_count=1"));
        assert!(message.contains("id=20"));
        assert_eq!(runtime.focused_widget(), None);
        assert_eq!(runtime.pointer_capture(), None);
        assert_eq!(runtime.hovered_widget(), None);
        assert_eq!(
            runtime
                .last_refresh_diagnostics()
                .identity
                .replacement_count,
            1
        );
        assert_eq!(
            runtime
                .take_frame_refresh_diagnostics()
                .refresh
                .identity
                .replacement_count,
            1
        );
        let counters = runtime.refresh_counters();
        assert_eq!(counters.reconciliation_attempts, 0);
        assert_eq!(counters.reconciliation_applied, 0);
        assert_eq!(counters.reconciliation_unsupported, 0);
        assert_eq!(counters.reconciliation_fallbacks, 1);
    }

    #[test]
    fn strict_identity_audit_reports_total_count_and_bounded_records() {
        let mut runtime = SurfaceRuntime::new(
            ReplacementBridge {
                replacement_count: 6,
                ..ReplacementBridge::default()
            },
            Vector2::new(800.0, 80.0),
        );
        runtime.set_identity_audit(IdentityAudit::strict());
        runtime.bridge_mut().replace = true;

        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| runtime.refresh()));
        let payload = result.expect_err("strict identity audit should fail");
        let message = payload
            .downcast_ref::<String>()
            .expect("strict identity audit should use a String payload");
        assert!(message.contains("replacement_count=6"));
        assert!(message.contains("stored_count=4"));
        assert!(message.contains("omitted_records=2"));
        assert_eq!(
            runtime
                .last_refresh_diagnostics()
                .identity
                .replacement_count,
            6
        );
        assert!(runtime.last_refresh_diagnostics().identity.replacements[3].is_some());
        assert!(
            runtime.last_refresh_diagnostics().identity.replacements[0]
                .is_some_and(|replacement| replacement.widget_id == 20)
        );
    }

    #[test]
    fn strict_identity_audit_marks_deep_paths_as_truncated() {
        let mut runtime = SurfaceRuntime::new(
            ReplacementBridge {
                deep: true,
                ..ReplacementBridge::default()
            },
            Vector2::new(120.0, 80.0),
        );
        runtime.set_identity_audit(IdentityAudit::strict());
        runtime.bridge_mut().replace = true;

        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| runtime.refresh()));
        let payload = result.expect_err("strict identity audit should fail");
        let message = payload
            .downcast_ref::<String>()
            .expect("strict identity audit should use a String payload");
        assert!(message.contains("truncated"));
        let replacement = runtime.last_refresh_diagnostics().identity.replacements[0]
            .expect("deep replacement diagnostic");
        assert!(replacement.previous_path.truncated);
        assert!(replacement.current_path.truncated);
    }

    #[test]
    fn fresh_surface_refresh_records_bounded_view_delta_summary() {
        let mut runtime =
            SurfaceRuntime::new(ReplacementBridge::default(), Vector2::new(120.0, 80.0));
        runtime.bridge_mut().replace = true;

        runtime.refresh();

        let summary = runtime.last_view_delta_diagnostics;
        assert!(summary.classified);
        assert_eq!(
            summary.effect,
            crate::runtime::surface::ViewDeltaEffect::Structural
        );
        assert_eq!(summary.total_events, 1);
        assert_eq!(summary.recorded_events, 1);
        assert_eq!(summary.omitted_events, 0);
        assert_eq!(
            summary.structural_cause,
            Some(crate::runtime::surface::ViewDeltaCause::IncompatibleWidget)
        );
    }

    #[test]
    fn exact_surface_and_projection_refreshes_reuse_completed_layout() {
        let mut runtime = SurfaceRuntime::new(
            ReplacementBridge {
                exact: true,
                ..ReplacementBridge::default()
            },
            Vector2::new(120.0, 80.0),
        );
        let before = runtime.refresh_counters();

        runtime.refresh();
        let after_surface = runtime.refresh_counters();
        assert_eq!(
            after_surface.application_projection,
            before.application_projection + 1
        );
        assert_eq!(
            after_surface.runtime_projection,
            before.runtime_projection + 1
        );
        assert_eq!(
            after_surface.widget_state_sync,
            before.widget_state_sync + 1
        );
        assert_eq!(after_surface.layout, before.layout);

        runtime.refresh_with_scope(RepaintScope::Projection);
        let after_projection = runtime.refresh_counters();
        assert_eq!(
            after_projection.application_projection,
            after_surface.application_projection + 1
        );
        assert_eq!(
            after_projection.runtime_projection,
            after_surface.runtime_projection + 1
        );
        assert_eq!(
            after_projection.widget_state_sync,
            after_surface.widget_state_sync + 1
        );
        assert_eq!(after_projection.layout, after_surface.layout);
    }

    #[test]
    fn exact_unchanged_candidate_records_applied_attempt_without_node_reuse() {
        let mut runtime = SurfaceRuntime::new(
            ReplacementBridge {
                exact: true,
                ..ReplacementBridge::default()
            },
            Vector2::new(120.0, 80.0),
        );
        let before = runtime.refresh_counters();

        runtime.refresh_with_scope(RepaintScope::Projection);

        let after = runtime.refresh_counters();
        assert_eq!(
            after.reconciliation_attempts,
            before.reconciliation_attempts + 1
        );
        assert_eq!(
            after.reconciliation_applied,
            before.reconciliation_applied + 1
        );
        assert_eq!(
            after.reconciliation_unsupported,
            before.reconciliation_unsupported
        );
        assert_eq!(
            after.reconciliation_fallbacks,
            before.reconciliation_fallbacks
        );
        assert_eq!(
            after.runtime_projection,
            before.runtime_projection + 1,
            "the complete runtime projection remains the correctness path"
        );
        assert_eq!(
            after.layout, before.layout,
            "Applied only retains the completed layout; it does not claim node reuse"
        );
    }

    #[test]
    fn exact_changed_leaf_records_unsupported_attempt_and_uses_full_path() {
        let mut runtime = SurfaceRuntime::new(
            ReplacementBridge {
                semantic_mode: true,
                ..ReplacementBridge::default()
            },
            Vector2::new(120.0, 80.0),
        );
        let before = runtime.refresh_counters();
        runtime.bridge_mut().semantic_changed = true;

        runtime.refresh_with_scope(RepaintScope::Projection);

        let after = runtime.refresh_counters();
        assert_eq!(
            after.reconciliation_attempts,
            before.reconciliation_attempts + 1
        );
        assert_eq!(after.reconciliation_applied, before.reconciliation_applied);
        assert_eq!(
            after.reconciliation_unsupported,
            before.reconciliation_unsupported + 1
        );
        assert_eq!(
            after.reconciliation_fallbacks,
            before.reconciliation_fallbacks + 1
        );
        assert_eq!(
            after.runtime_projection,
            before.runtime_projection + 1,
            "unsupported candidates use the complete runtime projection"
        );
        assert_eq!(after.layout, before.layout);
        assert!(runtime.base_paint_plan_reuse_eligible());
    }

    #[test]
    fn layout_capability_diagnostics_do_not_change_refresh_authority() {
        let mut runtime = SurfaceRuntime::new(
            LayoutCapabilityBridge {
                mode: LayoutCapabilityMode::Exact("same"),
            },
            Vector2::new(120.0, 80.0),
        );
        let _ = runtime.take_frame_refresh_diagnostics();

        runtime.refresh_with_scope(RepaintScope::Projection);
        let baseline = runtime.take_frame_refresh_diagnostics();
        let baseline_layout = runtime.refresh_counters().layout;
        assert_eq!(baseline.effective_scope, RepaintScope::Projection);
        assert!(runtime.base_paint_plan_reuse_eligible());
        assert_eq!(baseline.view_delta.effect, ViewDeltaEffect::Unchanged);
        assert_eq!(baseline.view_delta.reconciliation.mismatch_count, 0);

        for (mode, expected_effect, expected_conservative) in [
            (
                LayoutCapabilityMode::Exact("changed"),
                ViewDeltaEffect::Interaction,
                false,
            ),
            (
                LayoutCapabilityMode::Conservative,
                ViewDeltaEffect::Structural,
                true,
            ),
            (
                LayoutCapabilityMode::Incompatible,
                ViewDeltaEffect::Structural,
                true,
            ),
        ] {
            runtime.bridge_mut().mode = mode;
            runtime.refresh_with_scope(RepaintScope::Projection);

            let frame = runtime.take_frame_refresh_diagnostics();
            assert_eq!(frame.effective_scope, baseline.effective_scope);
            assert_eq!(runtime.refresh_counters().layout, baseline_layout);
            assert!(runtime.base_paint_plan_reuse_eligible());

            let summary = frame.view_delta;
            assert_eq!(summary.effect, ViewDeltaEffect::Unchanged);
            assert_eq!(summary.total_events, 0);
            assert_eq!(summary.recorded_events, 0);
            assert_eq!(summary.omitted_events, 0);
            assert!(!summary.truncated_paths);
            assert_eq!(summary.structural_cause, None);
            assert!(summary.base_paint_reuse_safe);
            assert_eq!(summary.reconciliation.mismatch_count, 0);
            assert!(!summary.damage.full_viewport);
            assert_eq!(summary.damage.candidate_count, 0);

            assert_eq!(summary.diagnostic.effect, expected_effect);
            assert_eq!(summary.diagnostic.total_events, 1);
            assert_eq!(summary.diagnostic.event_count, 1);
            assert_eq!(summary.diagnostic.omitted_events, 0);
            assert!(!summary.diagnostic.truncated_paths);
            assert_eq!(summary.diagnostic.conservative, expected_conservative);
            let event = summary.diagnostic.events[0].expect("layout capability diagnostic");
            assert_eq!(event.cause, ViewDeltaCause::LayoutCapabilities);
            assert_eq!(event.effect, expected_effect);
        }
    }

    #[test]
    fn requested_layout_always_recomputes_even_with_exact_evidence() {
        let mut runtime = SurfaceRuntime::new(
            ReplacementBridge {
                exact: true,
                ..ReplacementBridge::default()
            },
            Vector2::new(120.0, 80.0),
        );
        let before = runtime.refresh_counters().layout;

        runtime.refresh_with_scope(RepaintScope::Layout);

        assert_eq!(runtime.refresh_counters().layout, before + 1);
        assert!(!runtime.base_paint_plan_reuse_eligible());
    }

    #[test]
    fn zero_view_delta_scratch_vetoes_exact_leaf_layout_reuse() {
        let mut runtime = SurfaceRuntime::new(
            ReplacementBridge {
                exact: true,
                ..ReplacementBridge::default()
            },
            Vector2::new(120.0, 80.0),
        );
        runtime.scratch.view_delta = crate::runtime::surface::ViewDeltaScratch::with_capacity(0);
        let before = runtime.refresh_counters().layout;

        runtime.refresh_with_scope(RepaintScope::Surface);
        assert_eq!(runtime.refresh_counters().layout, before + 1);
        assert!(!runtime.base_paint_plan_reuse_eligible());

        runtime.refresh_with_scope(RepaintScope::Projection);
        assert_eq!(runtime.refresh_counters().layout, before + 2);
        assert!(!runtime.base_paint_plan_reuse_eligible());
    }

    #[test]
    fn completed_layout_context_changes_veto_reuse() {
        let mut runtime = SurfaceRuntime::new(
            ReplacementBridge {
                exact: true,
                ..ReplacementBridge::default()
            },
            Vector2::new(120.0, 80.0),
        );
        let baseline = runtime.refresh_counters().layout;

        runtime.viewport.max.x += 1.0;
        runtime.refresh();
        assert_eq!(runtime.refresh_counters().layout, baseline + 1);
        assert!(!runtime.base_paint_plan_reuse_eligible());

        runtime.layout_debug_options = crate::layout::LayoutDebugOptions::bounds_only();
        runtime.refresh();
        assert_eq!(runtime.refresh_counters().layout, baseline + 2);
        assert!(!runtime.base_paint_plan_reuse_eligible());

        runtime.set_window_environment(crate::runtime::WindowEnvironment::new(
            crate::theme::DpiScale::new(2.0),
            None,
            false,
            false,
        ));
        runtime.refresh();
        assert_eq!(runtime.refresh_counters().layout, baseline + 3);
        assert!(!runtime.base_paint_plan_reuse_eligible());

        runtime.layout_state_generation = runtime.layout_state_generation.saturating_add(1);
        runtime.refresh();
        assert_eq!(runtime.refresh_counters().layout, baseline + 4);
        assert!(!runtime.base_paint_plan_reuse_eligible());

        runtime.external_layout_dirty = true;
        runtime.refresh();
        assert_eq!(runtime.refresh_counters().layout, baseline + 5);
        assert!(!runtime.base_paint_plan_reuse_eligible());
    }

    #[test]
    fn paint_only_refresh_skips_view_delta_classification() {
        let mut runtime = SurfaceRuntime::new(
            ReplacementBridge {
                exact: true,
                ..ReplacementBridge::default()
            },
            Vector2::new(120.0, 80.0),
        );

        runtime.refresh_with_scope(RepaintScope::Projection);
        assert!(runtime.base_paint_plan_reuse_eligible());

        runtime.refresh_with_scope(RepaintScope::PaintOnly);

        assert!(!runtime.base_paint_plan_reuse_eligible());
        let summary = runtime.last_view_delta_diagnostics;
        assert!(!summary.classified);
        assert_eq!(summary.total_events, 0);
        assert_eq!(summary.duration, Duration::ZERO);
    }

    #[test]
    fn insufficient_view_delta_scratch_records_structural_fallback() {
        let mut runtime = SurfaceRuntime::new(
            ReplacementBridge {
                replacement_count: 1,
                ..ReplacementBridge::default()
            },
            Vector2::new(120.0, 80.0),
        );
        runtime.scratch.view_delta = crate::runtime::surface::ViewDeltaScratch::with_capacity(0);
        let _ = runtime.take_frame_refresh_diagnostics();
        let before = runtime.refresh_counters();

        runtime.refresh_with_scope(RepaintScope::Projection);

        let summary = runtime.last_view_delta_diagnostics;
        assert!(summary.classified);
        assert_eq!(
            summary.effect,
            crate::runtime::surface::ViewDeltaEffect::Structural
        );
        assert_eq!(
            summary.structural_cause,
            Some(crate::runtime::surface::ViewDeltaCause::InsufficientIdentityEvidence)
        );
        assert_eq!(summary.total_events, 1);
        assert_eq!(summary.recorded_events, 1);
        let frame = runtime.take_frame_refresh_diagnostics();
        assert_eq!(frame.effective_scope, RepaintScope::Surface);
        let after = runtime.refresh_counters();
        assert_eq!(after.layout, before.layout + 1);
        assert_eq!(
            after.reconciliation_attempts,
            before.reconciliation_attempts
        );
        assert_eq!(after.reconciliation_applied, before.reconciliation_applied);
        assert_eq!(
            after.reconciliation_unsupported,
            before.reconciliation_unsupported
        );
        assert_eq!(
            after.reconciliation_fallbacks,
            before.reconciliation_fallbacks + 1
        );
        assert!(!runtime.base_paint_plan_reuse_eligible());
    }

    #[test]
    fn projection_geometry_evidence_promotes_to_layout() {
        let mut runtime = SurfaceRuntime::new(
            ReplacementBridge {
                geometry_mode: true,
                ..ReplacementBridge::default()
            },
            Vector2::new(120.0, 80.0),
        );
        let _ = runtime.take_frame_refresh_diagnostics();
        let layout_before = runtime.refresh_counters().layout;
        runtime.bridge_mut().geometry = true;

        runtime.refresh_with_scope(RepaintScope::Projection);

        let frame = runtime.take_frame_refresh_diagnostics();
        assert_eq!(frame.requested_scope, RepaintScope::Projection);
        assert_eq!(frame.effective_scope, RepaintScope::Layout);
        assert_eq!(runtime.refresh_counters().layout, layout_before + 1);
        assert!(!runtime.base_paint_plan_reuse_eligible());
    }

    #[test]
    fn refresh_frame_records_bounded_surface_damage_candidates() {
        let mut runtime = SurfaceRuntime::new(
            ReplacementBridge {
                geometry_mode: true,
                ..ReplacementBridge::default()
            },
            Vector2::new(120.0, 80.0),
        );
        let _ = runtime.take_frame_refresh_diagnostics();
        runtime.bridge_mut().geometry = true;

        runtime.refresh_with_scope(RepaintScope::Projection);

        let frame = runtime.take_frame_refresh_diagnostics();
        assert!(!frame.view_delta.damage.full_viewport);
        assert_eq!(frame.view_delta.damage.candidate_count, 1);
        let candidate = frame.view_delta.damage.candidates[0]
            .expect("geometry refresh should retain one bounded candidate");
        assert!(candidate.old_bounds.is_some());
        assert!(candidate.new_bounds.is_some());
        assert_eq!(
            candidate.effect,
            crate::runtime::surface::ViewDeltaEffect::Geometry
        );
    }

    #[test]
    fn projection_structural_evidence_promotes_to_surface() {
        let mut runtime =
            SurfaceRuntime::new(ReplacementBridge::default(), Vector2::new(120.0, 80.0));
        let _ = runtime.take_frame_refresh_diagnostics();
        runtime.bridge_mut().replace = true;

        runtime.refresh_with_scope(RepaintScope::Projection);

        let frame = runtime.take_frame_refresh_diagnostics();
        assert_eq!(frame.requested_scope, RepaintScope::Projection);
        assert_eq!(frame.effective_scope, RepaintScope::Surface);
        assert_eq!(frame.refresh.invalidation, SurfaceInvalidation::Projection);
        assert!(!runtime.base_paint_plan_reuse_eligible());
    }

    #[test]
    fn layout_structural_evidence_promotes_to_surface() {
        let mut runtime =
            SurfaceRuntime::new(ReplacementBridge::default(), Vector2::new(120.0, 80.0));
        let _ = runtime.take_frame_refresh_diagnostics();
        runtime.bridge_mut().replace = true;

        runtime.refresh_with_scope(RepaintScope::Layout);

        let frame = runtime.take_frame_refresh_diagnostics();
        assert_eq!(frame.requested_scope, RepaintScope::Layout);
        assert_eq!(frame.effective_scope, RepaintScope::Surface);
    }

    #[test]
    fn opaque_mapper_evidence_promotes_projection_to_surface() {
        let mut runtime =
            SurfaceRuntime::new(ReplacementBridge::default(), Vector2::new(120.0, 80.0));
        let _ = runtime.take_frame_refresh_diagnostics();
        runtime.bridge_mut().mapper_changed = true;

        runtime.refresh_with_scope(RepaintScope::Projection);

        let frame = runtime.take_frame_refresh_diagnostics();
        assert_eq!(frame.requested_scope, RepaintScope::Projection);
        assert_eq!(frame.effective_scope, RepaintScope::Surface);
        assert!(frame.view_delta.base_paint_reuse_safe);
        assert!(!runtime.base_paint_plan_reuse_eligible());
    }

    #[test]
    fn semantic_revision_evidence_promotes_projection_to_projection() {
        let mut runtime = SurfaceRuntime::new(
            ReplacementBridge {
                semantic_mode: true,
                ..ReplacementBridge::default()
            },
            Vector2::new(120.0, 80.0),
        );
        let _ = runtime.take_frame_refresh_diagnostics();
        runtime.bridge_mut().semantic_changed = true;

        runtime.refresh_with_scope(RepaintScope::Projection);

        let frame = runtime.take_frame_refresh_diagnostics();
        assert_eq!(frame.requested_scope, RepaintScope::Projection);
        assert_eq!(frame.effective_scope, RepaintScope::Projection);
        assert_eq!(
            frame.view_delta.effect,
            crate::runtime::surface::ViewDeltaEffect::Interaction
        );
        assert!(runtime.base_paint_plan_reuse_eligible());
    }

    #[test]
    fn unchanged_projection_stays_narrow_and_surface_never_narrows() {
        let mut runtime = SurfaceRuntime::new(
            ReplacementBridge {
                exact: true,
                ..ReplacementBridge::default()
            },
            Vector2::new(120.0, 80.0),
        );
        let _ = runtime.take_frame_refresh_diagnostics();

        runtime.refresh_with_scope(RepaintScope::Projection);
        assert!(runtime.base_paint_plan_reuse_eligible());
        let projection = runtime.take_frame_refresh_diagnostics();
        assert_eq!(projection.effective_scope, RepaintScope::Projection);

        runtime.refresh_with_scope(RepaintScope::Surface);
        assert!(runtime.base_paint_plan_reuse_eligible());
        let surface = runtime.take_frame_refresh_diagnostics();
        assert_eq!(surface.requested_scope, RepaintScope::Surface);
        assert_eq!(surface.effective_scope, RepaintScope::Surface);
    }

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    enum FocusedRefreshMode {
        Compatible,
        Removed,
        Incompatible,
        Disabled,
        ReadOnly,
    }

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    enum FocusedRefreshMessage {
        Press { key: WidgetKey, repeat: bool },
    }

    #[derive(Clone)]
    struct FocusedRefreshWidget {
        common: WidgetCommon,
        captured: Option<WidgetKey>,
    }

    impl FocusedRefreshWidget {
        fn new(mode: FocusedRefreshMode) -> Self {
            let mut common = WidgetCommon::fixed(160, 120.0, 32.0).with_keyboard_focus();
            common.state.disabled = mode == FocusedRefreshMode::Disabled;
            common.state.read_only = mode == FocusedRefreshMode::ReadOnly;
            Self {
                common,
                captured: None,
            }
        }
    }

    impl Widget for FocusedRefreshWidget {
        fn common(&self) -> &WidgetCommon {
            &self.common
        }

        fn common_mut(&mut self) -> &mut WidgetCommon {
            &mut self.common
        }

        fn handle_input(&mut self, _bounds: Rect, input: WidgetInput) -> Option<WidgetOutput> {
            match input {
                WidgetInput::KeyPress { key, repeat, .. } => {
                    if !repeat && key == WidgetKey::ArrowUp {
                        self.captured = Some(key);
                    }
                    Some(WidgetOutput::typed(FocusedRefreshMessage::Press {
                        key,
                        repeat,
                    }))
                }
                _ => None,
            }
        }

        fn synchronize_from_previous(&mut self, previous: &dyn Widget) {
            if let Some(previous) = previous.as_any().downcast_ref::<Self>() {
                self.captured = previous.captured;
            }
        }

        fn participates_in_focused_key_routing(&self) -> bool {
            true
        }

        fn captured_focused_key(&self) -> Option<WidgetKey> {
            self.captured
        }

        fn append_paint(
            &self,
            _primitives: &mut Vec<crate::runtime::PaintPrimitive>,
            _bounds: Rect,
            _layout: &crate::layout::LayoutOutput,
            _theme: &crate::theme::ThemeTokens,
        ) {
        }
    }

    struct FocusedRefreshBridge {
        mode: FocusedRefreshMode,
        messages: Vec<FocusedRefreshMessage>,
    }

    impl RuntimeBridge<FocusedRefreshMessage> for FocusedRefreshBridge {
        fn project_surface(&mut self) -> Arc<UiSurface<FocusedRefreshMessage>> {
            let node = match self.mode {
                FocusedRefreshMode::Compatible
                | FocusedRefreshMode::Disabled
                | FocusedRefreshMode::ReadOnly => SurfaceNode::widget(
                    FocusedRefreshWidget::new(self.mode),
                    WidgetMessageMapper::typed(|message: FocusedRefreshMessage| message),
                ),
                FocusedRefreshMode::Removed => {
                    SurfaceNode::container(1, ContainerPolicy::default(), Vec::new())
                }
                FocusedRefreshMode::Incompatible => SurfaceNode::text(
                    160,
                    "replacement",
                    WidgetSizing::fixed(Vector2::new(120.0, 32.0)),
                ),
            };
            crate::runtime::test_arc_surface(UiSurface::new(node))
        }

        fn reduce_message(&mut self, message: FocusedRefreshMessage) {
            self.messages.push(message);
        }
    }

    fn focused_refresh_runtime() -> SurfaceRuntime<FocusedRefreshBridge, FocusedRefreshMessage> {
        SurfaceRuntime::new(
            FocusedRefreshBridge {
                mode: FocusedRefreshMode::Compatible,
                messages: Vec::new(),
            },
            Vector2::new(120.0, 32.0),
        )
    }

    fn capture_focused_refresh_key(
        runtime: &mut SurfaceRuntime<FocusedRefreshBridge, FocusedRefreshMessage>,
    ) {
        assert!(runtime.focus_widget(160));
        assert_eq!(
            runtime.dispatch_event(Event::KeyPress {
                key: WidgetKey::ArrowUp,
                modifiers: Default::default(),
                repeat: false,
                timestamp: None,
            }),
            Some(160)
        );
    }

    #[test]
    fn focused_key_capture_survives_only_exact_compatible_reprojection() {
        let mut runtime = focused_refresh_runtime();
        capture_focused_refresh_key(&mut runtime);
        runtime.refresh();

        assert_eq!(
            runtime.dispatch_event(Event::KeyPress {
                key: WidgetKey::ArrowUp,
                modifiers: Default::default(),
                repeat: true,
                timestamp: None,
            }),
            Some(160)
        );
        assert_eq!(
            runtime.bridge().messages,
            vec![
                FocusedRefreshMessage::Press {
                    key: WidgetKey::ArrowUp,
                    repeat: false,
                },
                FocusedRefreshMessage::Press {
                    key: WidgetKey::ArrowUp,
                    repeat: true,
                },
            ]
        );
    }

    #[test]
    fn focused_key_capture_stales_on_removal_incompatible_replacement_and_authority_loss() {
        for mode in [
            FocusedRefreshMode::Removed,
            FocusedRefreshMode::Incompatible,
            FocusedRefreshMode::Disabled,
            FocusedRefreshMode::ReadOnly,
        ] {
            let mut runtime = focused_refresh_runtime();
            capture_focused_refresh_key(&mut runtime);
            runtime.bridge_mut().mode = mode;
            runtime.refresh();
            let message_count = runtime.bridge().messages.len();

            assert_eq!(
                runtime.dispatch_event(Event::KeyPress {
                    key: WidgetKey::ArrowUp,
                    modifiers: Default::default(),
                    repeat: true,
                    timestamp: None,
                }),
                None,
                "stale focused-key sample must be ignored for {mode:?}"
            );
            assert_eq!(runtime.bridge().messages.len(), message_count);
        }
    }
}

impl SurfaceIdentityPath {
    fn from_slice(path: &[usize]) -> Self {
        let len = path.len().min(MAX_IDENTITY_PATH_COMPONENTS);
        let mut components = [0; MAX_IDENTITY_PATH_COMPONENTS];
        components[..len].copy_from_slice(&path[..len]);
        Self {
            components,
            len: len as u8,
            truncated: path.len() > len,
        }
    }

    /// Return the non-padding path components.
    pub fn as_slice(&self) -> &[usize] {
        &self.components[..self.len as usize]
    }
}

/// Controller-owned interaction domains discarded for one incompatible replacement.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SurfaceIdentityOwnership {
    /// Keyboard focus was owned by the replaced widget.
    pub focus: bool,
    /// Pointer capture or retained capture state was owned by the replaced widget.
    pub pointer_capture: bool,
    /// Widget hover ownership was owned by the replaced widget.
    pub hover: bool,
    /// Retained widget-local interaction state was intentionally not synchronized.
    pub widget_state: bool,
}

/// One bounded incompatible retained-widget replacement diagnostic.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SurfaceIdentityReplacement {
    /// Stable identity shared by the old and new widget.
    pub widget_id: WidgetId,
    /// Concrete compatibility label of the previous widget.
    pub previous_kind: &'static str,
    /// Concrete compatibility label of the replacement widget.
    pub current_kind: &'static str,
    /// Resolved path of the previous widget.
    pub previous_path: SurfaceIdentityPath,
    /// Resolved path of the replacement widget.
    pub current_path: SurfaceIdentityPath,
    /// Controller-owned domains discarded during replacement.
    pub discarded_ownership: SurfaceIdentityOwnership,
}

/// Bounded identity diagnostics emitted while reconciling one refresh.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SurfaceIdentityDiagnostics {
    /// First replacements in deterministic paint order, up to the fixed bound.
    pub replacements: [Option<SurfaceIdentityReplacement>; MAX_IDENTITY_REPLACEMENTS_PER_REFRESH],
    /// Number of replacements observed, including entries omitted by the bound.
    pub replacement_count: u32,
}

impl Default for SurfaceIdentityDiagnostics {
    fn default() -> Self {
        Self {
            replacements: [None; MAX_IDENTITY_REPLACEMENTS_PER_REFRESH],
            replacement_count: 0,
        }
    }
}

impl SurfaceIdentityDiagnostics {
    const fn startup() -> Self {
        Self {
            replacements: [None; MAX_IDENTITY_REPLACEMENTS_PER_REFRESH],
            replacement_count: 0,
        }
    }

    fn push(&mut self, replacement: SurfaceIdentityReplacement) {
        let index = self.replacement_count as usize;
        if index < self.replacements.len() {
            self.replacements[index] = Some(replacement);
        }
        self.replacement_count = self.replacement_count.saturating_add(1);
    }

    fn merge(&mut self, other: Self) {
        let base = self.replacement_count as usize;
        for (offset, replacement) in other.replacements.into_iter().enumerate() {
            let Some(replacement) = replacement else {
                continue;
            };
            let index = base.saturating_add(offset);
            if index < self.replacements.len() {
                self.replacements[index] = Some(replacement);
            }
        }
        self.replacement_count = self
            .replacement_count
            .saturating_add(other.replacement_count);
    }
}

/// Cumulative counts for independently measurable refresh stages.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SurfaceRefreshCounters {
    /// Host application surface projections pulled by the runtime.
    pub application_projection: u64,
    /// Runtime projection/traversal rebuilds.
    pub runtime_projection: u64,
    /// Exact candidates evaluated by the private reconciliation seam.
    ///
    /// This counts seam decisions, not nodes visited or reused.
    pub reconciliation_attempts: u64,
    /// Exact unchanged candidates accepted by the existing completed-layout
    /// retention path. This is not a node- or subtree-reuse count.
    pub reconciliation_applied: u64,
    /// Exact candidates that reached the seam but have no supported partial
    /// node/subtree operation and therefore use the full refresh path.
    pub reconciliation_unsupported: u64,
    /// Refreshes whose evidence or scope conservatively selected the full path,
    /// including unsupported exact candidates.
    pub reconciliation_fallbacks: u64,
    /// Widget-state synchronization passes.
    pub widget_state_sync: u64,
    /// Layout passes.
    pub layout: u64,
    /// Native/backend-neutral base paint plans rebuilt by the host renderer.
    pub base_paint_plan_rebuilds: u64,
}

impl SurfaceRefreshCounters {
    pub(in crate::runtime) const fn startup() -> Self {
        Self {
            application_projection: 1,
            runtime_projection: 1,
            reconciliation_attempts: 0,
            reconciliation_applied: 0,
            reconciliation_unsupported: 0,
            reconciliation_fallbacks: 0,
            widget_state_sync: 0,
            layout: 1,
            base_paint_plan_rebuilds: 0,
        }
    }
}

/// Independent CPU timing buckets for one surface refresh.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SurfaceRefreshTimings {
    /// Time spent pulling the host application projection.
    pub application_projection: Duration,
    /// Time spent rebuilding runtime projection and traversal.
    pub runtime_projection: Duration,
    /// Time spent synchronizing widget state.
    pub widget_state_sync: Duration,
    /// Time spent recomputing layout.
    pub layout: Duration,
}

impl SurfaceRefreshTimings {
    /// Return the sum of the independently measured refresh stages.
    pub fn total(self) -> Duration {
        self.application_projection
            .saturating_add(self.runtime_projection)
            .saturating_add(self.widget_state_sync)
            .saturating_add(self.layout)
    }

    fn merge(&mut self, other: Self) {
        self.application_projection = self
            .application_projection
            .saturating_add(other.application_projection);
        self.runtime_projection = self
            .runtime_projection
            .saturating_add(other.runtime_projection);
        self.widget_state_sync = self
            .widget_state_sync
            .saturating_add(other.widget_state_sync);
        self.layout = self.layout.saturating_add(other.layout);
    }
}

/// Diagnostics for the most recent typed surface invalidation.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SurfaceRefreshDiagnostics {
    /// Chosen invalidation stage.
    pub invalidation: SurfaceInvalidation,
    /// Independent timing buckets for work performed by that stage.
    pub timings: SurfaceRefreshTimings,
    /// Bounded incompatible retained-widget replacement diagnostics.
    pub identity: SurfaceIdentityDiagnostics,
    /// Bounded runtime-owned layout-interaction state diagnostics.
    pub layout_state: SurfaceLayoutStateDiagnostics,
}

/// Runtime/frame transport for observational view-delta evidence.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct SurfaceRefreshFrameDiagnostics {
    pub(crate) refresh: SurfaceRefreshDiagnostics,
    pub(crate) view_delta: ViewDeltaDiagnostics,
    pub(crate) paint_segments: crate::runtime::PaintSegmentObservation,
    pub(crate) total: Duration,
    pub(crate) requested_scope: RepaintScope,
    pub(crate) effective_scope: RepaintScope,
    has_refresh: bool,
}

impl SurfaceRefreshFrameDiagnostics {
    pub(crate) const fn startup() -> Self {
        Self {
            refresh: SurfaceRefreshDiagnostics::startup(),
            view_delta: ViewDeltaDiagnostics::startup(),
            paint_segments: crate::runtime::PaintSegmentObservation::unavailable(),
            total: Duration::ZERO,
            requested_scope: RepaintScope::Surface,
            effective_scope: RepaintScope::Surface,
            has_refresh: true,
        }
    }

    fn record(
        &mut self,
        refresh: SurfaceRefreshDiagnostics,
        view_delta: ViewDeltaDiagnostics,
        total: Duration,
        requested_scope: RepaintScope,
        effective_scope: RepaintScope,
    ) {
        if !self.has_refresh {
            *self = Self {
                refresh,
                view_delta,
                paint_segments: self.paint_segments,
                total,
                requested_scope,
                effective_scope,
                has_refresh: true,
            };
            return;
        }
        self.refresh.merge(refresh);
        self.view_delta.merge(view_delta);
        self.total = self.total.saturating_add(total);
        self.requested_scope = self.requested_scope.merge(requested_scope);
        self.effective_scope = self.effective_scope.merge(effective_scope);
    }
}

impl Default for SurfaceRefreshFrameDiagnostics {
    fn default() -> Self {
        Self {
            refresh: SurfaceRefreshDiagnostics::default(),
            view_delta: ViewDeltaDiagnostics::default(),
            paint_segments: crate::runtime::PaintSegmentObservation::unavailable(),
            total: Duration::ZERO,
            requested_scope: RepaintScope::PaintOnly,
            effective_scope: RepaintScope::PaintOnly,
            has_refresh: false,
        }
    }
}

impl SurfaceRefreshDiagnostics {
    pub(in crate::runtime) const fn startup() -> Self {
        Self {
            invalidation: SurfaceInvalidation::Surface,
            timings: SurfaceRefreshTimings {
                application_projection: Duration::ZERO,
                runtime_projection: Duration::ZERO,
                widget_state_sync: Duration::ZERO,
                layout: Duration::ZERO,
            },
            identity: SurfaceIdentityDiagnostics::startup(),
            layout_state: SurfaceLayoutStateDiagnostics::startup(),
        }
    }

    fn merge(&mut self, other: Self) {
        self.invalidation = SurfaceInvalidation::from_repaint_scope(
            match (
                self.invalidation.repaint_scope(),
                other.invalidation.repaint_scope(),
            ) {
                (Some(current), Some(next)) => Some(current.merge(next)),
                (Some(scope), None) | (None, Some(scope)) => Some(scope),
                (None, None) => None,
            },
        );
        self.timings.merge(other.timings);
        self.identity.merge(other.identity);
        self.layout_state.merge(other.layout_state);
    }
}

fn can_reuse_completed_layout<Bridge, Message>(
    runtime: &SurfaceRuntime<Bridge, Message>,
    decision: RefreshExecutionDecision,
) -> bool
where
    Bridge: RuntimeBridge<Message>,
{
    if !decision.allows_completed_layout_reuse()
        || !runtime.scratch.view_delta.has_identity_capacity()
    {
        return false;
    }
    if !runtime.virtual_layout.is_empty()
        && runtime
            .virtual_layout
            .requires_materialization(&runtime.layout, false)
    {
        return false;
    }
    let Some(completed) = runtime.completed_layout else {
        return false;
    };
    completed.viewport == effective_layout_viewport(runtime.viewport)
        && completed.window_environment == runtime.window_environment
        && completed.layout_state_generation == runtime.layout_state_generation
        && completed.layout_debug_options == runtime.layout_debug_options
        && !runtime.external_layout_dirty
        && !runtime.layout_engine.has_explicit_dirty()
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct BasePaintPlanContext {
    pub(crate) viewport: Rect,
    pub(crate) window_environment: crate::runtime::WindowEnvironment,
    pub(crate) layout_state_generation: u64,
    pub(crate) layout_debug_options: crate::layout::LayoutDebugOptions,
    pub(crate) hovered_container: Option<crate::layout::NodeId>,
    pub(crate) hovered_widget: Option<WidgetId>,
    pub(crate) hovered_scroll_affordance: Option<crate::layout::NodeId>,
    pub(crate) focused_widget: Option<WidgetId>,
    pub(crate) pointer_capture: Option<WidgetId>,
    pub(crate) pointer_capture_state: Option<(WidgetId, crate::widgets::WidgetState)>,
    pub(crate) scrollbar_drag: Option<crate::layout::NodeId>,
}

fn effective_layout_viewport(viewport: Rect) -> Rect {
    Rect::from_min_size(
        Point::new(viewport.min.x.floor(), viewport.min.y.floor()),
        Vector2::new(
            viewport.width().round().max(0.0),
            viewport.height().round().max(0.0),
        ),
    )
}

impl<Bridge, Message> SurfaceRuntime<Bridge, Message>
where
    Bridge: RuntimeBridge<Message>,
{
    /// Reproject the latest host state using the correctness-first full refresh path.
    pub fn refresh(&mut self) {
        self.refresh_with_scope(RepaintScope::Surface);
    }

    /// Apply one typed repaint scope to the current projected surface.
    ///
    /// A fresh `Surface` or `Projection` may reuse a completed layout only when
    /// exact, geometry-stable view-delta evidence and the completed-layout
    /// context still match. Startup, resize, identity changes, and unknown
    /// custom-host changes remain conservative.
    pub fn refresh_with_scope(&mut self, scope: RepaintScope) {
        let terminal_messages = self.refresh_with_scope_inner(scope);
        self.dispatch_deferred_surface_messages(terminal_messages);
        self.service_pending_current_surface_relayout();
    }

    fn refresh_with_scope_inner(&mut self, scope: RepaintScope) -> Vec<Message> {
        self.validate_managed_pointer_capture_authority();
        self.validate_managed_wheel_sequence_authority();
        self.validate_managed_composition_authority();
        let refresh_started = Instant::now();
        let invalidation = SurfaceInvalidation::from_repaint_scope(Some(scope));
        self.last_layout_state_diagnostics = SurfaceLayoutStateDiagnostics::default();
        if scope.is_paint_only() {
            self.base_paint_plan_reuse_eligible = false;
            let view_delta = ViewDeltaDiagnostics {
                damage: SurfaceDamage::full_viewport(self.viewport),
                ..ViewDeltaDiagnostics::default()
            };
            self.record_refresh_diagnostics(
                SurfaceRefreshDiagnostics {
                    invalidation,
                    timings: SurfaceRefreshTimings::default(),
                    identity: SurfaceIdentityDiagnostics::default(),
                    layout_state: SurfaceLayoutStateDiagnostics::default(),
                },
                Duration::ZERO,
                view_delta,
                RepaintScope::PaintOnly,
            );
            return Vec::new();
        }

        let previous_widget_order = self.traversal.widgets.hit_order.clone();
        let previous_stateful_widget_order = self.traversal.widgets.stateful_order.clone();

        let application_projection_started = Instant::now();
        let mut next_surface = self.bridge.pull_surface();
        next_surface.set_window_environment(self.window_environment);
        let application_projection = application_projection_started.elapsed();
        self.refresh_counters.application_projection = self
            .refresh_counters
            .application_projection
            .saturating_add(1);

        let view_delta_started = Instant::now();
        let had_virtual_layout = !self.virtual_layout.is_empty();
        let mut traversal;
        let mut layout_root;
        let mut raw_probe_source = None;
        let runtime_projection_started = Instant::now();
        if had_virtual_layout {
            if let Some(mut probe) = self.virtual_layout.take_projection_probe() {
                layout_root = next_surface.runtime_projection_reusing_with_scratch_and_source(
                    &mut probe.traversal,
                    &mut self.scratch.projection_scroll_stack,
                    &mut self.scratch.projection_child_path,
                    &mut probe.source,
                );
                raw_probe_source = Some(probe.source);
                traversal = probe.traversal;
            } else {
                let projection = next_surface.runtime_projection();
                let crate::runtime::SurfaceRuntimeProjection {
                    layout_root: projected_layout_root,
                    traversal: projected_traversal,
                    source,
                } = projection;
                layout_root = projected_layout_root;
                raw_probe_source = Some(source);
                traversal = projected_traversal;
            }
        } else {
            std::mem::swap(
                &mut self.traversal.widgets.paths.previous,
                &mut self.traversal.widgets.paths.current,
            );
            traversal = self.take_reusable_traversal_index(true);
            layout_root = next_surface.runtime_projection_reusing_with_scratch(
                &mut traversal,
                &mut self.scratch.projection_scroll_stack,
                &mut self.scratch.projection_child_path,
                &mut self.scratch.projection_source,
            );
        }
        let mut runtime_projection = runtime_projection_started.elapsed();
        self.refresh_counters.runtime_projection =
            self.refresh_counters.runtime_projection.saturating_add(1);

        // Keep the prior admitted traversal available until the cached result
        // has authoritative unchanged evidence; the paired probe is reusable
        // scratch for the raw registration projection.
        self.virtual_layout
            .prepare_surface(&mut next_surface, &traversal.virtual_layout_registrations);

        let mut raw_view_delta =
            classify_view_delta(&self.surface, &next_surface, &mut self.scratch.view_delta);
        let mut execution = RefreshExecutionDecision::from_view_delta(scope, &raw_view_delta);
        let mut effective_scope = execution.effective_scope();
        let reuse_completed_layout = can_reuse_completed_layout(self, execution);
        let unchanged_virtual_layout = had_virtual_layout
            && !self.virtual_layout.is_empty()
            && raw_view_delta.effect == crate::runtime::surface::ViewDeltaEffect::Unchanged
            && reuse_completed_layout;
        let mut paths_prepared = !had_virtual_layout;
        if unchanged_virtual_layout {
            self.virtual_layout
                .store_projection_probe(RuntimeVirtualLayoutProjectionProbe {
                    traversal,
                    source: match raw_probe_source.take() {
                        Some(source) => source,
                        None => {
                            next_surface.runtime_source_traversal_index_reusing(
                                &mut self.scratch.projection_source,
                            );
                            self.scratch.projection_source.clone()
                        }
                    },
                });
            // The installed traversal is authoritative for this exact cached
            // result. Take it directly instead of swapping to the prior
            // reusable path buffer before the unchanged decision is consumed.
            paths_prepared = true;
            traversal = self.take_reusable_traversal_index(true);
            layout_root = self.layout_root.clone();
            next_surface
                .runtime_source_traversal_index_reusing(&mut self.scratch.projection_source);
        } else if !self.virtual_layout.is_empty() {
            let probe = if had_virtual_layout {
                let probe = RuntimeVirtualLayoutProjectionProbe {
                    traversal,
                    source: match raw_probe_source.take() {
                        Some(source) => source,
                        None => {
                            next_surface.runtime_source_traversal_index_reusing(
                                &mut self.scratch.projection_source,
                            );
                            self.scratch.projection_source.clone()
                        }
                    },
                };
                std::mem::swap(
                    &mut self.traversal.widgets.paths.previous,
                    &mut self.traversal.widgets.paths.current,
                );
                paths_prepared = true;
                traversal = self.take_reusable_traversal_index(true);
                Some(probe)
            } else {
                None
            };
            let post_cache_projection_started = Instant::now();
            layout_root = next_surface.runtime_projection_reusing_with_scratch(
                &mut traversal,
                &mut self.scratch.projection_scroll_stack,
                &mut self.scratch.projection_child_path,
                &mut self.scratch.projection_source,
            );
            runtime_projection =
                runtime_projection.saturating_add(post_cache_projection_started.elapsed());
            self.refresh_counters.runtime_projection =
                self.refresh_counters.runtime_projection.saturating_add(1);
            if let Some(probe) = probe {
                self.virtual_layout.store_projection_probe(probe);
            }
        }

        if had_virtual_layout && self.virtual_layout.is_empty() {
            next_surface
                .runtime_source_traversal_index_reusing(&mut self.scratch.projection_source);
        }

        self.base_paint_plan_reuse_eligible =
            execution.allows_base_paint_plan_reuse() && reuse_completed_layout;
        let mut reconciliation_plan = raw_view_delta.reconciliation_plan();
        let mut damage = SurfaceDamage::from_view_delta(
            &raw_view_delta,
            &reconciliation_plan,
            &self.surface,
            &self.layout,
            self.viewport,
        );
        let mut view_delta = raw_view_delta.diagnostics(view_delta_started.elapsed());

        let virtual_layout_pass_required = !self.virtual_layout.is_empty()
            && self.requires_virtual_layout_materialization(!reuse_completed_layout);
        if virtual_layout_pass_required {
            self.layout_engine.layout_with_state_into(
                &layout_root,
                self.viewport,
                &self.layout_state,
                self.layout_debug_options,
                &mut self.layout,
            );
            self.virtual_layout
                .materialize_surface(&mut next_surface, &self.layout);
            raw_view_delta =
                classify_view_delta(&self.surface, &next_surface, &mut self.scratch.view_delta);
            view_delta = raw_view_delta.diagnostics(view_delta_started.elapsed());
            execution = RefreshExecutionDecision::from_view_delta(scope, &raw_view_delta);
            effective_scope = execution.effective_scope();
            reconciliation_plan = raw_view_delta.reconciliation_plan();
            self.base_paint_plan_reuse_eligible = false;
            damage = SurfaceDamage::from_view_delta(
                &raw_view_delta,
                &reconciliation_plan,
                &self.surface,
                &self.layout,
                self.viewport,
            );
            let final_projection_started = Instant::now();
            layout_root = next_surface.runtime_projection_reusing_with_scratch(
                &mut traversal,
                &mut self.scratch.projection_scroll_stack,
                &mut self.scratch.projection_child_path,
                &mut self.scratch.projection_source,
            );
            runtime_projection =
                runtime_projection.saturating_add(final_projection_started.elapsed());
            self.refresh_counters.runtime_projection =
                self.refresh_counters.runtime_projection.saturating_add(1);
        }

        let reconciliation_attempt =
            raw_view_delta.reconciliation_attempt_outcome(scope, execution, reuse_completed_layout);
        self.record_reconciliation_attempt(reconciliation_attempt);

        if had_virtual_layout && !paths_prepared {
            std::mem::swap(
                &mut self.traversal.widgets.paths.previous,
                &mut self.traversal.widgets.paths.current,
            );
        }

        let mut previous_paths = if unchanged_virtual_layout {
            None
        } else {
            Some(std::mem::take(&mut self.traversal.widgets.paths.previous))
        };
        let previous_paths_for_refresh = previous_paths.as_ref().unwrap_or(&traversal.widget_paths);
        let replacement_plan: WidgetReplacementPlan = self.surface.plan_widget_replacements(
            &next_surface,
            &previous_stateful_widget_order,
            &previous_widget_order,
            &traversal.widget_paint_order,
            &traversal.widget_paths,
            previous_paths_for_refresh,
        );
        let replacement_commit: WidgetReplacementCommitResult<Message> =
            self.surface.commit_widget_replacements(
                &next_surface,
                replacement_plan,
                &previous_widget_order,
                &traversal.widget_paint_order,
                previous_paths_for_refresh,
                &traversal.widget_paths,
            );
        let replacement_veto: Option<WidgetReplacementPlanVeto> = replacement_commit.veto;
        let replacement_commit = if replacement_veto.is_some() {
            self.surface.commit_widget_replacements_immediately(
                &next_surface,
                &previous_stateful_widget_order,
                &previous_widget_order,
                &traversal.widget_paint_order,
                &traversal.widget_paths,
                previous_paths_for_refresh,
            )
        } else {
            replacement_commit
        };
        let terminal_messages = replacement_commit.terminal_messages;
        let retired_widget_ids = replacement_commit.retired_widget_ids;
        let wheel_focus_before_refresh = self.interaction.focus.focused_widget();
        let composition_focus_before_refresh = self.interaction.focus.focused_widget();
        let identity = self.discard_incompatible_widget_ownership(
            &next_surface,
            &traversal.widget_paint_order,
            &traversal.widget_paths,
            previous_paths_for_refresh,
        );
        for widget_id in &retired_widget_ids {
            self.discard_widget_ownership(*widget_id);
        }
        let widget_state_sync_started = Instant::now();
        let sync_policy = self.widget_state_sync_policy();
        next_surface.synchronize_widget_state_from_paths_with_evidence(
            &self.surface,
            &traversal.stateful_widget_order,
            &traversal.widget_paths,
            previous_paths_for_refresh,
            &previous_widget_order,
            &traversal.widget_paint_order,
            &retired_widget_ids,
            sync_policy,
        );
        let widget_state_sync = widget_state_sync_started.elapsed();
        self.refresh_counters.widget_state_sync =
            self.refresh_counters.widget_state_sync.saturating_add(1);
        self.reconcile_focused_key_capture_after_refresh(
            &next_surface,
            &previous_widget_order,
            &traversal.widget_paint_order,
            &previous_stateful_widget_order,
            &traversal.stateful_widget_order,
            previous_paths_for_refresh,
            &traversal.widget_paths,
            &retired_widget_ids,
        );
        self.reconcile_managed_wheel_sequence_after_refresh(
            &next_surface,
            &previous_widget_order,
            &traversal.widget_paint_order,
            previous_paths_for_refresh,
            &traversal.widget_paths,
            &retired_widget_ids,
            wheel_focus_before_refresh,
        );
        self.reconcile_managed_composition_after_refresh(
            &next_surface,
            &previous_widget_order,
            &traversal.widget_paint_order,
            previous_paths_for_refresh,
            &traversal.widget_paths,
            &retired_widget_ids,
            composition_focus_before_refresh,
        );
        self.reconcile_managed_pointer_capture_after_refresh(
            &next_surface,
            &previous_widget_order,
            &traversal.widget_paint_order,
            previous_paths_for_refresh,
            &traversal.widget_paths,
            &retired_widget_ids,
        );
        if let Some(previous_paths) = previous_paths.take() {
            self.traversal.widgets.paths.previous = previous_paths;
        }

        self.surface = next_surface;
        self.replace_layout_root(layout_root);
        if self.interaction.pointer.managed_capture.is_some() {
            self.interaction.pointer.capture_state = None;
        }
        self.restore_pointer_capture_state();
        let layout_required = !reuse_completed_layout
            && (effective_scope.refreshes_layout()
                || matches!(scope, RepaintScope::Surface | RepaintScope::Projection));
        let layout = if layout_required {
            let layout_started = Instant::now();
            self.relayout_with_traversal(traversal);
            self.refresh_counters.layout = self.refresh_counters.layout.saturating_add(1);
            layout_started.elapsed()
        } else {
            let candidate = self.prepare_layout_container_state_candidate(&traversal);
            self.install_traversal_with_candidate(traversal, candidate);
            Duration::ZERO
        };
        self.validate_managed_pointer_capture_authority();
        self.validate_managed_wheel_sequence_authority();
        self.validate_managed_composition_authority();
        if let Some(capture) = self.interaction.pointer.managed_capture
            && capture.state == RuntimeManagedPointerCaptureState::Active
        {
            self.capture_pointer_capture_state(capture.widget_id);
        }
        self.clear_stale_interaction_state();
        if let Some(widget_id) = self.interaction.focus.focused_widget() {
            self.restore_focused_widget_state(widget_id);
        }
        self.validate_focused_key_capture_authority();

        // Only the source buffer produced by the final accepted projection is
        // allowed to replace the controller-owned declarative owner evidence.
        // Virtual-layout probes remain provisional until this boundary.
        self.install_declarative_owner_projection();

        view_delta.damage = damage.finish(&self.surface, &self.layout);

        self.record_refresh_diagnostics(
            SurfaceRefreshDiagnostics {
                invalidation,
                timings: SurfaceRefreshTimings {
                    application_projection,
                    runtime_projection,
                    widget_state_sync,
                    layout,
                },
                identity,
                layout_state: self.last_layout_state_diagnostics,
            },
            refresh_started.elapsed(),
            view_delta,
            effective_scope,
        );
        self.enforce_identity_audit(identity);
        terminal_messages
    }

    /// Return diagnostics for the most recent typed invalidation stage.
    pub const fn last_refresh_diagnostics(&self) -> SurfaceRefreshDiagnostics {
        self.last_refresh_diagnostics
    }

    pub(super) fn record_refresh_diagnostics(
        &mut self,
        diagnostics: SurfaceRefreshDiagnostics,
        total: Duration,
        view_delta: ViewDeltaDiagnostics,
        effective_scope: RepaintScope,
    ) {
        self.last_refresh_diagnostics = diagnostics;
        self.last_view_delta_diagnostics = view_delta;
        self.pending_frame_refresh.record(
            diagnostics,
            view_delta,
            total,
            diagnostics
                .invalidation
                .repaint_scope()
                .unwrap_or(RepaintScope::PaintOnly),
            effective_scope,
        );
    }

    fn record_reconciliation_attempt(&mut self, outcome: ReconciliationAttemptOutcome) {
        if outcome.was_attempted() {
            self.refresh_counters.reconciliation_attempts = self
                .refresh_counters
                .reconciliation_attempts
                .saturating_add(1);
        }
        if outcome.was_applied() {
            self.refresh_counters.reconciliation_applied = self
                .refresh_counters
                .reconciliation_applied
                .saturating_add(1);
        } else {
            self.refresh_counters.reconciliation_fallbacks = self
                .refresh_counters
                .reconciliation_fallbacks
                .saturating_add(1);
        }
        if matches!(outcome, ReconciliationAttemptOutcome::Unsupported) {
            self.refresh_counters.reconciliation_unsupported = self
                .refresh_counters
                .reconciliation_unsupported
                .saturating_add(1);
        }
    }

    pub(crate) fn take_frame_refresh_diagnostics(&mut self) -> SurfaceRefreshFrameDiagnostics {
        let mut frame = std::mem::take(&mut self.pending_frame_refresh);
        frame.paint_segments = self.latest_paint_segment_observation;
        frame
    }

    /// Return cumulative refresh-stage counts for this runtime.
    pub const fn refresh_counters(&self) -> SurfaceRefreshCounters {
        self.refresh_counters
    }

    pub(crate) fn base_paint_plan_context(&self) -> BasePaintPlanContext {
        BasePaintPlanContext {
            viewport: self.viewport,
            window_environment: self.window_environment,
            layout_state_generation: self.layout_state_generation,
            layout_debug_options: self.layout_debug_options,
            hovered_container: self.interaction.hover.container,
            hovered_widget: self.interaction.hover.widget,
            hovered_scroll_affordance: self.interaction.hover.scroll_affordance,
            focused_widget: self.interaction.focus.focused_widget(),
            pointer_capture: self.interaction.pointer.capture,
            pointer_capture_state: self.interaction.pointer.capture_state,
            scrollbar_drag: self
                .interaction
                .pointer
                .scroll_drag_capture
                .map(|capture| capture.node_id),
        }
    }

    pub(crate) fn base_paint_plan_reuse_eligible(&self) -> bool {
        self.base_paint_plan_reuse_eligible
    }

    pub(crate) fn record_base_paint_plan_rebuild(&mut self) {
        self.refresh_counters.base_paint_plan_rebuilds = self
            .refresh_counters
            .base_paint_plan_rebuilds
            .saturating_add(1);
    }

    pub(super) fn enforce_identity_audit(&self, identity: SurfaceIdentityDiagnostics) {
        if !self.identity_audit.is_strict() || identity.replacement_count == 0 {
            return;
        }

        let stored_count = identity.replacements.iter().flatten().count() as u32;
        let omitted_count = identity.replacement_count.saturating_sub(stored_count);
        let mut message = String::from(
            "radiant identity audit: strict mode detected incompatible widget replacements; ",
        );
        let _ = write!(
            message,
            "replacement_count={}; stored_count={}; omitted_records={}; records=",
            identity.replacement_count, stored_count, omitted_count
        );
        message.push('[');
        for (index, replacement) in identity.replacements.iter().flatten().enumerate() {
            if index != 0 {
                message.push_str(", ");
            }
            let _ = write!(message, "{{index={}, id=", index);
            let _ = write!(message, "{}; previous_path=", replacement.widget_id);
            append_identity_path(&mut message, replacement.previous_path);
            message.push_str("; current_path=");
            append_identity_path(&mut message, replacement.current_path);
            message.push('}');
        }
        message.push(']');
        std::panic::panic_any(message);
    }

    pub(super) fn discard_incompatible_widget_ownership(
        &mut self,
        next_surface: &crate::runtime::UiSurface<Message>,
        widget_paint_order: &[WidgetId],
        current_paths: &std::collections::HashMap<WidgetId, crate::runtime::WidgetPath>,
        previous_paths: &std::collections::HashMap<WidgetId, crate::runtime::WidgetPath>,
    ) -> SurfaceIdentityDiagnostics {
        let mut diagnostics = SurfaceIdentityDiagnostics::default();
        for widget_id in widget_paint_order {
            let Some(current_path) = current_paths.get(widget_id) else {
                continue;
            };
            let Some(previous_path) = previous_paths.get(widget_id) else {
                continue;
            };
            let Some((previous_kind, previous_valid)) = self
                .surface
                .widget_compatibility_at_path(previous_path.as_slice())
            else {
                continue;
            };
            let Some((current_kind, current_valid)) =
                next_surface.widget_compatibility_at_path(current_path.as_slice())
            else {
                continue;
            };
            if previous_valid && current_valid && previous_kind == current_kind {
                continue;
            }
            let previous_kind = if previous_valid {
                previous_kind
            } else {
                INVALID_COMPATIBILITY_KIND
            };
            let current_kind = if current_valid {
                current_kind
            } else {
                INVALID_COMPATIBILITY_KIND
            };
            let discarded_ownership = self.discard_widget_ownership(*widget_id);
            diagnostics.push(SurfaceIdentityReplacement {
                widget_id: *widget_id,
                previous_kind,
                current_kind,
                previous_path: SurfaceIdentityPath::from_slice(previous_path.as_slice()),
                current_path: SurfaceIdentityPath::from_slice(current_path.as_slice()),
                discarded_ownership,
            });
        }
        diagnostics
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn reconcile_focused_key_capture_after_refresh(
        &mut self,
        next_surface: &crate::runtime::UiSurface<Message>,
        previous_widget_order: &[WidgetId],
        current_widget_order: &[WidgetId],
        previous_stateful_widget_order: &[WidgetId],
        current_stateful_widget_order: &[WidgetId],
        previous_paths: &std::collections::HashMap<WidgetId, crate::runtime::WidgetPath>,
        current_paths: &std::collections::HashMap<WidgetId, crate::runtime::WidgetPath>,
        retired_widget_ids: &[WidgetId],
    ) {
        let Some(capture) = self.interaction.focus.focused_key_capture else {
            return;
        };
        if capture.stale {
            return;
        }

        let widget_id = capture.widget_id;
        let exact_compatible_sync = self.interaction.focus.focused_widget() == Some(widget_id)
            && has_unique_widget_id(previous_widget_order, widget_id)
            && has_unique_widget_id(current_widget_order, widget_id)
            && has_unique_widget_id(previous_stateful_widget_order, widget_id)
            && has_unique_widget_id(current_stateful_widget_order, widget_id)
            && !retired_widget_ids.contains(&widget_id)
            && previous_paths
                .get(&widget_id)
                .zip(current_paths.get(&widget_id))
                .is_some_and(|(previous_path, current_path)| {
                    let Some(previous_widget) =
                        self.surface.find_widget_at_path(widget_id, previous_path)
                    else {
                        return false;
                    };
                    let Some(current_widget) =
                        next_surface.find_widget_at_path(widget_id, current_path)
                    else {
                        return false;
                    };
                    let Some((previous_kind, previous_valid)) = self
                        .surface
                        .widget_compatibility_at_path(previous_path.as_slice())
                    else {
                        return false;
                    };
                    let Some((current_kind, current_valid)) =
                        next_surface.widget_compatibility_at_path(current_path.as_slice())
                    else {
                        return false;
                    };
                    previous_valid
                        && current_valid
                        && previous_kind == current_kind
                        && previous_widget
                            .widget_object()
                            .participates_in_focused_key_routing()
                        && previous_widget.widget_object().captured_focused_key()
                            == Some(capture.key)
                        && current_widget.is_focusable()
                        && !current_widget.widget_object().common().state.read_only
                        && current_widget
                            .widget_object()
                            .participates_in_focused_key_routing()
                        && current_widget.widget_object().captured_focused_key()
                            == Some(capture.key)
                });
        if !exact_compatible_sync {
            self.mark_focused_key_capture_stale(widget_id);
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn reconcile_managed_wheel_sequence_after_refresh(
        &mut self,
        next_surface: &crate::runtime::UiSurface<Message>,
        previous_widget_order: &[WidgetId],
        current_widget_order: &[WidgetId],
        previous_paths: &std::collections::HashMap<WidgetId, crate::runtime::WidgetPath>,
        current_paths: &std::collections::HashMap<WidgetId, crate::runtime::WidgetPath>,
        retired_widget_ids: &[WidgetId],
        focused_widget_before_refresh: Option<WidgetId>,
    ) {
        let RuntimeManagedWheelSequenceState::Active { widget_id } =
            self.interaction.wheel.managed_sequence
        else {
            return;
        };

        // Classify the wheel owner before generic identity cleanup can clear
        // focus or other controller domains. Blocked has no identity and must
        // survive this refresh unless a later lifecycle boundary closes it.
        let hard_stale = retired_widget_ids.contains(&widget_id)
            || !has_unique_widget_id(previous_widget_order, widget_id)
            || !has_unique_widget_id(current_widget_order, widget_id);
        let Some(previous_path) = previous_paths.get(&widget_id) else {
            self.block_managed_wheel_sequence();
            return;
        };
        let Some(current_path) = current_paths.get(&widget_id) else {
            self.block_managed_wheel_sequence();
            return;
        };
        let Some((previous_kind, previous_valid)) = self
            .surface
            .widget_compatibility_at_path(previous_path.as_slice())
        else {
            self.block_managed_wheel_sequence();
            return;
        };
        let Some((current_kind, current_valid)) =
            next_surface.widget_compatibility_at_path(current_path.as_slice())
        else {
            self.block_managed_wheel_sequence();
            return;
        };
        let previous_widget = self.surface.find_widget_at_path(widget_id, previous_path);
        let current_widget = next_surface.find_widget_at_path(widget_id, current_path);
        let previous_live = previous_widget.is_some_and(|widget| {
            self.managed_refresh_wheel_widget_is_live(
                widget,
                widget_id,
                focused_widget_before_refresh,
            )
        });
        let current_live = current_widget.is_some_and(|widget| {
            self.managed_refresh_wheel_widget_is_live(
                widget,
                widget_id,
                focused_widget_before_refresh,
            )
        });
        let exact_compatible = !hard_stale
            && previous_path == current_path
            && previous_valid
            && current_valid
            && previous_kind == current_kind;

        if exact_compatible && previous_live && current_live {
            return;
        }

        // A live owner may be softly retired only for a positively identified
        // same-path incompatible successor that can accept ordinary wheel
        // input. Retention is intentionally not required: this transition
        // releases managed ownership and lets a later sample use hit testing.
        let soft_incompatible_replacement = !hard_stale
            && previous_path == current_path
            && previous_valid
            && current_valid
            && previous_kind != current_kind
            && previous_live
            && current_widget.is_some_and(|widget| {
                self.managed_refresh_wheel_widget_is_admitting(widget, widget_id)
            });
        if soft_incompatible_replacement {
            self.clear_managed_wheel_sequence_for_widget(widget_id);
        } else {
            self.block_managed_wheel_sequence();
        }
    }

    fn managed_refresh_wheel_widget_is_admitting(
        &self,
        widget: &crate::runtime::SurfaceWidget<Message>,
        widget_id: WidgetId,
    ) -> bool {
        let common = widget.widget_object().common();
        widget.id() == widget_id
            && !common.state.disabled
            && !common.state.read_only
            && widget.receives_wheel_input()
    }

    fn managed_refresh_wheel_widget_is_live(
        &self,
        widget: &crate::runtime::SurfaceWidget<Message>,
        widget_id: WidgetId,
        focused_widget: Option<WidgetId>,
    ) -> bool {
        let common = widget.widget_object().common();
        widget.id() == widget_id
            && !common.state.disabled
            && !common.state.read_only
            && (!widget.is_focusable() || focused_widget == Some(widget_id))
            && widget.receives_wheel_input()
            && widget.retains_managed_wheel_sequence()
    }

    pub(super) fn discard_widget_ownership(
        &mut self,
        widget_id: WidgetId,
    ) -> SurfaceIdentityOwnership {
        self.mark_focused_key_capture_stale(widget_id);
        self.clear_managed_composition_for_widget(widget_id);
        let focus = matches!(
            self.interaction.focus.owner,
            Some(RuntimeFocusOwner::Widget(current)) if current == widget_id
        );
        let pointer_capture = self.interaction.pointer.capture == Some(widget_id)
            || self
                .interaction
                .pointer
                .capture_state
                .is_some_and(|(captured_id, _)| captured_id == widget_id)
            || self
                .interaction
                .pointer
                .managed_capture
                .is_some_and(|capture| capture.widget_id == widget_id);
        let hover = self.interaction.hover.widget == Some(widget_id);
        if self.interaction.tooltip.target == Some(widget_id) {
            self.reset_tooltip_hover_intent();
        }
        if focus {
            self.interaction.focus.owner = None;
        }
        if pointer_capture {
            self.clear_managed_pointer_capture_for_widget(widget_id);
            if let Some(button) = self.interaction.pointer.capture_button {
                self.interaction.pointer.set_release_tombstone(button, true);
            }
            self.interaction.pointer.capture = None;
            self.interaction.pointer.capture_button = None;
            self.interaction.pointer.capture_state = None;
        }
        if hover {
            self.interaction.hover.widget = None;
        }
        SurfaceIdentityOwnership {
            focus,
            pointer_capture,
            hover,
            widget_state: true,
        }
    }
}

fn has_unique_widget_id(widget_order: &[WidgetId], widget_id: WidgetId) -> bool {
    let mut found = false;
    for candidate in widget_order {
        if *candidate != widget_id {
            continue;
        }
        if found {
            return false;
        }
        found = true;
    }
    found
}

fn append_identity_path(message: &mut String, path: SurfaceIdentityPath) {
    message.push('[');
    for (index, component) in path.as_slice().iter().enumerate() {
        if index != 0 {
            message.push(',');
        }
        let _ = write!(message, "{component}");
    }
    message.push(']');
    if path.truncated {
        message.push_str(" (truncated)");
    }
}
