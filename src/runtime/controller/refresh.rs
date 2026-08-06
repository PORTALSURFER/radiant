//! Revision-backed surface refresh stages and diagnostics.

use super::{SurfaceRuntime, layout_state::SurfaceLayoutStateDiagnostics};
use crate::gui::types::{Point, Rect, Vector2};
use crate::runtime::{
    RepaintScope, RuntimeBridge, SurfaceInvalidation,
    surface::{RefreshExecutionDecision, SurfaceDamage, ViewDeltaDiagnostics, classify_view_delta},
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
            RuntimeBridge, SurfaceChild, SurfaceNode, UiSurface, WidgetMessageMapper,
            surface::{ViewDeltaCause, ViewDeltaEffect},
        },
        widgets::{ButtonWidget, ScrollbarAxis, ScrollbarWidget, TextWidget, WidgetSizing},
    };
    use std::{cell::Cell, rc::Rc, sync::Arc};

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
        runtime.interaction.focus.focused_widget = Some(999);
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
        runtime.interaction.focus.focused_widget = Some(20);
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
        runtime.interaction.focus.focused_widget = Some(20);
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
        runtime.interaction.focus.focused_widget = Some(20);
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
        runtime.interaction.focus.focused_widget = Some(20);
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
        let layout_before = runtime.refresh_counters().layout;

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
        assert_eq!(runtime.refresh_counters().layout, layout_before + 1);
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
            return;
        }

        let application_projection_started = Instant::now();
        let mut next_surface = self.bridge.pull_surface();
        next_surface.set_window_environment(self.window_environment);
        let application_projection = application_projection_started.elapsed();
        self.refresh_counters.application_projection = self
            .refresh_counters
            .application_projection
            .saturating_add(1);

        let view_delta_started = Instant::now();
        std::mem::swap(
            &mut self.traversal.widgets.paths.previous,
            &mut self.traversal.widgets.paths.current,
        );
        let mut traversal = self.take_reusable_traversal_index(true);
        let runtime_projection_started = Instant::now();
        let mut layout_root = next_surface.runtime_projection_reusing_with_scratch(
            &mut traversal,
            &mut self.scratch.projection_scroll_stack,
            &mut self.scratch.projection_child_path,
        );
        let runtime_projection = runtime_projection_started.elapsed();
        self.refresh_counters.runtime_projection =
            self.refresh_counters.runtime_projection.saturating_add(1);

        self.virtual_layout
            .prepare_surface(&mut next_surface, &traversal.virtual_layout_registrations);

        if !self.virtual_layout.is_empty() {
            layout_root = next_surface.runtime_projection_reusing_with_scratch(
                &mut traversal,
                &mut self.scratch.projection_scroll_stack,
                &mut self.scratch.projection_child_path,
            );
        }

        let mut raw_view_delta =
            classify_view_delta(&self.surface, &next_surface, &mut self.scratch.view_delta);
        let mut execution = RefreshExecutionDecision::from_view_delta(scope, &raw_view_delta);
        let mut effective_scope = execution.effective_scope();
        let reuse_completed_layout = can_reuse_completed_layout(self, execution);
        self.base_paint_plan_reuse_eligible =
            execution.allows_base_paint_plan_reuse() && reuse_completed_layout;
        let mut damage = SurfaceDamage::from_view_delta(
            &raw_view_delta,
            &raw_view_delta.reconciliation_plan(),
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
            self.base_paint_plan_reuse_eligible = false;
            damage = SurfaceDamage::from_view_delta(
                &raw_view_delta,
                &raw_view_delta.reconciliation_plan(),
                &self.surface,
                &self.layout,
                self.viewport,
            );
            layout_root = next_surface.runtime_projection_reusing_with_scratch(
                &mut traversal,
                &mut self.scratch.projection_scroll_stack,
                &mut self.scratch.projection_child_path,
            );
        }

        let previous_paths = std::mem::take(&mut self.traversal.widgets.paths.previous);
        let identity = self.discard_incompatible_widget_ownership(
            &next_surface,
            &traversal.widget_paint_order,
            &traversal.widget_paths,
            &previous_paths,
        );
        let widget_state_sync_started = Instant::now();
        let sync_policy = self.widget_state_sync_policy();
        next_surface.synchronize_widget_state_from_paths(
            &self.surface,
            &traversal.stateful_widget_order,
            &traversal.widget_paths,
            &previous_paths,
            sync_policy,
        );
        let widget_state_sync = widget_state_sync_started.elapsed();
        self.refresh_counters.widget_state_sync =
            self.refresh_counters.widget_state_sync.saturating_add(1);
        self.traversal.widgets.paths.previous = previous_paths;

        self.surface = next_surface;
        self.layout_root = layout_root;
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
            self.install_traversal_index(traversal);
            Duration::ZERO
        };
        self.clear_stale_interaction_state();
        if let Some(widget_id) = self.interaction.focus.focused_widget {
            self.restore_focused_widget_state(widget_id);
        }

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
    }

    /// Return diagnostics for the most recent typed invalidation stage.
    pub const fn last_refresh_diagnostics(&self) -> SurfaceRefreshDiagnostics {
        self.last_refresh_diagnostics
    }

    fn record_refresh_diagnostics(
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
            focused_widget: self.interaction.focus.focused_widget,
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

    fn enforce_identity_audit(&self, identity: SurfaceIdentityDiagnostics) {
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

    fn discard_incompatible_widget_ownership(
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

    fn discard_widget_ownership(&mut self, widget_id: WidgetId) -> SurfaceIdentityOwnership {
        let focus = self.interaction.focus.focused_widget == Some(widget_id);
        let pointer_capture = self.interaction.pointer.capture == Some(widget_id)
            || self
                .interaction
                .pointer
                .capture_state
                .is_some_and(|(captured_id, _)| captured_id == widget_id);
        let hover = self.interaction.hover.widget == Some(widget_id);
        if self.interaction.tooltip.target == Some(widget_id) {
            self.reset_tooltip_hover_intent();
        }
        if focus {
            self.interaction.focus.focused_widget = None;
        }
        if pointer_capture {
            self.interaction.pointer.capture = None;
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
