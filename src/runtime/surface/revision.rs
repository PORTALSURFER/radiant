//! Pure widget revision relation used by future incremental reconciliation.

// This relation is deliberately staged without a production caller. Keep the
// pure foundation available for contract tests until incremental refresh owns
// its invocation.
#![allow(dead_code)]

use super::widget::MapperRelation;
use crate::widgets::{WidgetId, WidgetRevision, WidgetRevisionComponents};

/// The broadest safe effect of one retained widget revision comparison.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum WidgetRevisionEffect {
    Structural,
    Geometry,
    Paint,
    Interaction,
    Unchanged,
}

/// Minimal comparison input for a same-ID retained widget pair.
#[derive(Clone, Debug)]
pub(crate) struct WidgetRevisionSnapshot {
    pub(crate) id: WidgetId,
    pub(crate) compatibility_kind: &'static str,
    pub(crate) revision: WidgetRevision,
}

/// Classify a widget pair without mutating or consulting runtime state.
///
/// Missing widgets, identity changes, compatibility changes, and conservative
/// revisions all take the structural/full fallback. Exact components are
/// compared in broadest-to-narrowest order so a multi-component change can
/// never be mistaken for a narrower effect.
pub(crate) fn classify_widget_revision(
    previous: Option<WidgetRevisionSnapshot>,
    current: Option<WidgetRevisionSnapshot>,
) -> WidgetRevisionEffect {
    let (Some(previous), Some(current)) = (previous, current) else {
        return WidgetRevisionEffect::Structural;
    };
    if previous.id != current.id || previous.compatibility_kind != current.compatibility_kind {
        return WidgetRevisionEffect::Structural;
    }

    let (Some(previous), Some(current)) = (
        previous.revision.exact_components(),
        current.revision.exact_components(),
    ) else {
        return WidgetRevisionEffect::Structural;
    };

    classify_exact_components(previous, current)
}

fn classify_exact_components(
    previous: &WidgetRevisionComponents,
    current: &WidgetRevisionComponents,
) -> WidgetRevisionEffect {
    if !previous.structure_equal(current) {
        WidgetRevisionEffect::Structural
    } else if !previous.geometry_equal(current) {
        WidgetRevisionEffect::Geometry
    } else if !previous.paint_equal(current) {
        WidgetRevisionEffect::Paint
    } else if !previous.interaction_equal(current) {
        WidgetRevisionEffect::Interaction
    } else {
        WidgetRevisionEffect::Unchanged
    }
}

/// Broadest safe effect observed while comparing two immutable surfaces.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum ViewDeltaEffect {
    Structural,
    Geometry,
    Paint,
    Interaction,
    Unchanged,
}

/// Closed set of semantic causes recorded by the observational classifier.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ViewDeltaCause {
    RootIdentity,
    NodeKind,
    NodeIdentity,
    Added,
    Removed,
    Reordered,
    IncompatibleWidget,
    OpaqueWidgetMapper,
    WidgetRevision,
    ContainerPolicy,
    ChildSlot,
    ContainerStyle,
    ContainerHover,
    ScrollMapper,
    SceneLayerKind,
    SceneLayerCount,
    SceneLayerInput,
    OverlayRect,
    OverlayLabel,
    OverlayStyle,
    FloatingInteractive,
}

/// One bounded semantic change record.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ViewDeltaEvent {
    pub(crate) effect: ViewDeltaEffect,
    pub(crate) cause: ViewDeltaCause,
    pub(crate) path: ViewDeltaPath,
}

/// A fixed-size path component used by [`ViewDeltaPath`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ViewDeltaPathComponent {
    Node(WidgetId),
    Child(u64),
    Layer(u64),
    Input,
    Foreground,
}

/// Bounded path identifying the semantic location of an event.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ViewDeltaPath {
    pub(crate) components: [Option<ViewDeltaPathComponent>; MAX_PATH_COMPONENTS],
    pub(crate) len: u8,
    pub(crate) truncated: bool,
}

impl ViewDeltaPath {
    const fn empty() -> Self {
        Self {
            components: [None; MAX_PATH_COMPONENTS],
            len: 0,
            truncated: false,
        }
    }
}

/// Bounded observational relation between two immutable surfaces.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ViewDelta {
    pub(crate) effect: ViewDeltaEffect,
    pub(crate) total_events: u32,
    pub(crate) events: [Option<ViewDeltaEvent>; MAX_VIEW_DELTA_EVENTS],
    pub(crate) event_count: u8,
    pub(crate) omitted_events: u32,
    pub(crate) truncated_paths: bool,
}

const MAX_VIEW_DELTA_EVENTS: usize = 16;
const MAX_PATH_COMPONENTS: usize = 8;

impl ViewDelta {
    const fn new() -> Self {
        Self {
            effect: ViewDeltaEffect::Unchanged,
            total_events: 0,
            events: [None; MAX_VIEW_DELTA_EVENTS],
            event_count: 0,
            omitted_events: 0,
            truncated_paths: false,
        }
    }

    fn record(&mut self, effect: ViewDeltaEffect, cause: ViewDeltaCause, path: ViewDeltaPath) {
        self.effect = broader_effect(self.effect, effect);
        self.total_events = self.total_events.saturating_add(1);
        self.truncated_paths |= path.truncated;
        if usize::from(self.event_count) < MAX_VIEW_DELTA_EVENTS {
            self.events[usize::from(self.event_count)] = Some(ViewDeltaEvent {
                effect,
                cause,
                path,
            });
            self.event_count += 1;
        } else {
            self.omitted_events = self.omitted_events.saturating_add(1);
        }
    }
}

fn broader_effect(left: ViewDeltaEffect, right: ViewDeltaEffect) -> ViewDeltaEffect {
    if left <= right { left } else { right }
}

#[derive(Clone, Copy)]
struct PathStack {
    path: ViewDeltaPath,
    depth: u16,
}

impl PathStack {
    const fn new() -> Self {
        Self {
            path: ViewDeltaPath::empty(),
            depth: 0,
        }
    }

    fn push(&mut self, component: ViewDeltaPathComponent) {
        self.depth = self.depth.saturating_add(1);
        let index = usize::from(self.depth.saturating_sub(1));
        if self.depth <= MAX_PATH_COMPONENTS as u16 {
            self.path.components[index] = Some(component);
            self.path.len = self.depth as u8;
        }
        self.path.truncated = self.depth > MAX_PATH_COMPONENTS as u16;
    }

    fn pop(&mut self) {
        if self.depth == 0 {
            return;
        }
        self.depth -= 1;
        if self.depth < MAX_PATH_COMPONENTS as u16 {
            self.path.components[usize::from(self.depth)] = None;
            self.path.len = self.depth as u8;
        }
        self.path.truncated = self.depth > MAX_PATH_COMPONENTS as u16;
    }
}

/// Compare two surfaces without allocating, mutating, or consulting runtime state.
pub(crate) fn classify_view_delta<Message>(
    previous: &super::UiSurface<Message>,
    current: &super::UiSurface<Message>,
) -> ViewDelta {
    let mut delta = ViewDelta::new();
    let mut path = PathStack::new();
    if previous.root.id() != current.root.id() {
        path.push(ViewDeltaPathComponent::Node(current.root.id()));
        delta.record(
            ViewDeltaEffect::Structural,
            ViewDeltaCause::RootIdentity,
            path.path,
        );
    } else {
        compare_node(&previous.root, &current.root, &mut path, &mut delta);
    }
    delta
}

fn compare_node<Message>(
    previous: &super::SurfaceNode<Message>,
    current: &super::SurfaceNode<Message>,
    path: &mut PathStack,
    delta: &mut ViewDelta,
) {
    path.push(ViewDeltaPathComponent::Node(current.id()));
    if previous.id() != current.id() {
        delta.record(
            ViewDeltaEffect::Structural,
            ViewDeltaCause::NodeIdentity,
            path.path,
        );
        path.pop();
        return;
    }
    match (previous, current) {
        (super::SurfaceNode::Scene(previous), super::SurfaceNode::Scene(current)) => {
            compare_scene(previous, current, path, delta)
        }
        (super::SurfaceNode::Container(previous), super::SurfaceNode::Container(current)) => {
            compare_container(previous, current, path, delta)
        }
        (super::SurfaceNode::Widget(previous), super::SurfaceNode::Widget(current)) => {
            compare_widget(previous, current, path, delta)
        }
        (super::SurfaceNode::Overlay(previous), super::SurfaceNode::Overlay(current)) => {
            if previous.rect != current.rect {
                delta.record(
                    ViewDeltaEffect::Geometry,
                    ViewDeltaCause::OverlayRect,
                    path.path,
                );
            }
            if previous.label != current.label {
                delta.record(
                    ViewDeltaEffect::Paint,
                    ViewDeltaCause::OverlayLabel,
                    path.path,
                );
            }
            if previous.style != current.style {
                delta.record(
                    ViewDeltaEffect::Paint,
                    ViewDeltaCause::OverlayStyle,
                    path.path,
                );
            }
        }
        (
            super::SurfaceNode::FloatingLayer(previous),
            super::SurfaceNode::FloatingLayer(current),
        ) => {
            compare_container(&previous.container, &current.container, path, delta);
            if previous.interactive != current.interactive {
                delta.record(
                    ViewDeltaEffect::Interaction,
                    ViewDeltaCause::FloatingInteractive,
                    path.path,
                );
            }
        }
        _ => delta.record(
            ViewDeltaEffect::Structural,
            ViewDeltaCause::NodeKind,
            path.path,
        ),
    }
    path.pop();
}

fn compare_widget<Message>(
    previous: &super::SurfaceWidget<Message>,
    current: &super::SurfaceWidget<Message>,
    path: &PathStack,
    delta: &mut ViewDelta,
) {
    if previous.compatibility_kind() != current.compatibility_kind() {
        delta.record(
            ViewDeltaEffect::Structural,
            ViewDeltaCause::IncompatibleWidget,
            path.path,
        );
        return;
    }
    for relation in [
        previous
            .output_mapper_descriptor()
            .relation(&current.output_mapper_descriptor()),
        previous
            .native_file_drop_mapper_descriptor()
            .relation(&current.native_file_drop_mapper_descriptor()),
    ] {
        match relation {
            MapperRelation::Structural => delta.record(
                ViewDeltaEffect::Structural,
                ViewDeltaCause::OpaqueWidgetMapper,
                path.path,
            ),
            MapperRelation::Interaction => delta.record(
                ViewDeltaEffect::Interaction,
                ViewDeltaCause::OpaqueWidgetMapper,
                path.path,
            ),
            MapperRelation::Unchanged => {}
        }
    }
    let effect = classify_widget_revision(
        Some(WidgetRevisionSnapshot {
            id: previous.id(),
            compatibility_kind: previous.compatibility_kind(),
            revision: previous.revision(),
        }),
        Some(WidgetRevisionSnapshot {
            id: current.id(),
            compatibility_kind: current.compatibility_kind(),
            revision: current.revision(),
        }),
    );
    if effect != WidgetRevisionEffect::Unchanged {
        delta.record(effect.into(), ViewDeltaCause::WidgetRevision, path.path);
    }
}

impl From<WidgetRevisionEffect> for ViewDeltaEffect {
    fn from(effect: WidgetRevisionEffect) -> Self {
        match effect {
            WidgetRevisionEffect::Structural => Self::Structural,
            WidgetRevisionEffect::Geometry => Self::Geometry,
            WidgetRevisionEffect::Paint => Self::Paint,
            WidgetRevisionEffect::Interaction => Self::Interaction,
            WidgetRevisionEffect::Unchanged => Self::Unchanged,
        }
    }
}

fn compare_container<Message>(
    previous_container: &super::SurfaceContainer<Message>,
    current_container: &super::SurfaceContainer<Message>,
    path: &mut PathStack,
    delta: &mut ViewDelta,
) {
    if previous_container.policy != current_container.policy {
        delta.record(
            ViewDeltaEffect::Geometry,
            ViewDeltaCause::ContainerPolicy,
            path.path,
        );
    }
    if previous_container.style != current_container.style {
        delta.record(
            ViewDeltaEffect::Paint,
            ViewDeltaCause::ContainerStyle,
            path.path,
        );
    }
    if previous_container.hoverable != current_container.hoverable {
        delta.record(
            ViewDeltaEffect::Paint,
            ViewDeltaCause::ContainerHover,
            path.path,
        );
    }
    match previous_container
        .scroll_mapper_descriptor()
        .relation(&current_container.scroll_mapper_descriptor())
    {
        MapperRelation::Structural => delta.record(
            ViewDeltaEffect::Structural,
            ViewDeltaCause::ScrollMapper,
            path.path,
        ),
        MapperRelation::Interaction => delta.record(
            ViewDeltaEffect::Interaction,
            ViewDeltaCause::ScrollMapper,
            path.path,
        ),
        MapperRelation::Unchanged => {}
    }
    if previous_container.children.len() != current_container.children.len() {
        let cause = if previous_container.children.len() < current_container.children.len() {
            ViewDeltaCause::Added
        } else {
            ViewDeltaCause::Removed
        };
        delta.record(ViewDeltaEffect::Structural, cause, path.path);
    }
    for (index, (previous, current)) in previous_container
        .children
        .iter()
        .zip(&current_container.children)
        .enumerate()
    {
        path.push(ViewDeltaPathComponent::Child(index as u64));
        if previous.slot != current.slot {
            delta.record(
                ViewDeltaEffect::Geometry,
                ViewDeltaCause::ChildSlot,
                path.path,
            );
        }
        if previous.child.id() != current.child.id()
            && previous_container
                .children
                .iter()
                .any(|candidate| candidate.child.id() == current.child.id())
            && current_container
                .children
                .iter()
                .any(|candidate| candidate.child.id() == previous.child.id())
        {
            delta.record(
                ViewDeltaEffect::Structural,
                ViewDeltaCause::Reordered,
                path.path,
            );
        } else {
            compare_node(&previous.child, &current.child, path, delta);
        }
        path.pop();
    }
}

fn compare_scene<Message>(
    previous: &super::SurfaceScene<Message>,
    current: &super::SurfaceScene<Message>,
    path: &mut PathStack,
    delta: &mut ViewDelta,
) {
    compare_node(&previous.base, &current.base, path, delta);
    let previous_count = previous.layers.len();
    let current_count = current.layers.len();
    if previous_count != current_count {
        delta.record(
            ViewDeltaEffect::Structural,
            ViewDeltaCause::SceneLayerCount,
            path.path,
        );
    }
    for (index, (previous, current)) in previous
        .ordered_layers()
        .zip(current.ordered_layers())
        .enumerate()
    {
        path.push(ViewDeltaPathComponent::Layer(index as u64));
        if previous.kind != current.kind {
            delta.record(
                ViewDeltaEffect::Structural,
                ViewDeltaCause::SceneLayerKind,
                path.path,
            );
            path.pop();
            continue;
        }
        if previous.input.is_some() != current.input.is_some() {
            delta.record(
                ViewDeltaEffect::Structural,
                ViewDeltaCause::SceneLayerInput,
                path.path,
            );
            path.pop();
            continue;
        }
        if let (Some(previous), Some(current)) = (&previous.input, &current.input) {
            path.push(ViewDeltaPathComponent::Input);
            compare_node(previous, current, path, delta);
            path.pop();
        }
        path.push(ViewDeltaPathComponent::Foreground);
        compare_node(&previous.node, &current.node, path, delta);
        path.pop();
        path.pop();
    }
}

#[cfg(test)]
mod tests {
    use super::{WidgetRevisionEffect, WidgetRevisionSnapshot, classify_widget_revision};
    use crate::layout::Vector2;
    use crate::widgets::{TextWidget, Widget, WidgetRevision, WidgetSizing};

    const KIND: &str = "test::Widget";

    fn snapshot(id: WidgetId, revision: WidgetRevision) -> WidgetRevisionSnapshot {
        WidgetRevisionSnapshot {
            id,
            compatibility_kind: KIND,
            revision,
        }
    }

    fn exact(structure: u64, geometry: u64, paint: u64, interaction: u64) -> WidgetRevision {
        WidgetRevision::exact(structure, geometry, paint, interaction)
    }

    use crate::widgets::WidgetId;

    #[test]
    fn missing_identity_and_replacements_take_structural_fallback() {
        let current = Some(snapshot(7, exact(1, 1, 1, 1)));
        assert_eq!(
            classify_widget_revision(None, current.clone()),
            WidgetRevisionEffect::Structural
        );
        assert_eq!(
            classify_widget_revision(current, None),
            WidgetRevisionEffect::Structural
        );
        assert_eq!(
            classify_widget_revision(
                Some(snapshot(7, exact(1, 1, 1, 1))),
                Some(snapshot(8, exact(1, 1, 1, 1))),
            ),
            WidgetRevisionEffect::Structural
        );
        assert_eq!(
            classify_widget_revision(
                Some(snapshot(7, exact(1, 1, 1, 1))),
                Some(WidgetRevisionSnapshot {
                    id: 7,
                    compatibility_kind: "test::OtherWidget",
                    revision: exact(1, 1, 1, 1),
                }),
            ),
            WidgetRevisionEffect::Structural
        );
    }

    #[test]
    fn conservative_revision_always_takes_structural_fallback() {
        let exact = Some(snapshot(7, exact(1, 1, 1, 1)));
        let conservative = Some(snapshot(7, WidgetRevision::conservative()));
        assert_eq!(
            classify_widget_revision(exact.clone(), conservative.clone()),
            WidgetRevisionEffect::Structural
        );
        assert_eq!(
            classify_widget_revision(conservative, exact),
            WidgetRevisionEffect::Structural
        );
    }

    #[test]
    fn exact_components_choose_the_broadest_changed_effect() {
        let base = Some(snapshot(7, exact(1, 1, 1, 1)));
        for (revision, expected) in [
            (exact(2, 1, 1, 1), WidgetRevisionEffect::Structural),
            (exact(1, 2, 1, 1), WidgetRevisionEffect::Geometry),
            (exact(1, 1, 2, 1), WidgetRevisionEffect::Paint),
            (exact(1, 1, 1, 2), WidgetRevisionEffect::Interaction),
            (exact(1, 1, 1, 1), WidgetRevisionEffect::Unchanged),
        ] {
            assert_eq!(
                classify_widget_revision(base.clone(), Some(snapshot(7, revision))),
                expected
            );
        }
        assert_eq!(
            classify_widget_revision(base, Some(snapshot(7, exact(2, 2, 2, 2)))),
            WidgetRevisionEffect::Structural
        );
    }

    #[test]
    fn typed_component_mismatch_widens_to_that_component_effect() {
        let previous = Some(snapshot(
            7,
            WidgetRevision::exact("structure", 1_u32, "paint", "interaction"),
        ));
        let current = Some(snapshot(
            7,
            WidgetRevision::exact("structure", 1_u64, "paint", "interaction"),
        ));

        assert_eq!(
            classify_widget_revision(previous, current),
            WidgetRevisionEffect::Geometry
        );
    }

    #[test]
    fn text_widget_revisions_reach_the_classifier_with_safe_effects() {
        let base = TextWidget::new(7, "hello", WidgetSizing::fixed(Vector2::new(80.0, 20.0)));
        let mut geometry = base.clone();
        geometry.wrap = crate::widgets::TextWrap::Word;
        let mut paint = base.clone();
        paint.align = crate::widgets::TextAlign::Center;
        let mut interaction = base.clone();
        interaction.common.tooltip = Some(String::from("hint"));
        let snapshot = |widget: &dyn Widget| WidgetRevisionSnapshot {
            id: widget.common().id,
            compatibility_kind: widget.compatibility_kind(),
            revision: widget.revision(),
        };

        let previous = snapshot(&base);
        assert_eq!(
            classify_widget_revision(Some(previous.clone()), Some(snapshot(&geometry))),
            WidgetRevisionEffect::Geometry
        );
        assert_eq!(
            classify_widget_revision(Some(previous.clone()), Some(snapshot(&paint))),
            WidgetRevisionEffect::Paint
        );
        assert_eq!(
            classify_widget_revision(Some(previous.clone()), Some(snapshot(&interaction))),
            WidgetRevisionEffect::Interaction
        );
        assert_eq!(
            classify_widget_revision(Some(previous.clone()), Some(snapshot(&base))),
            WidgetRevisionEffect::Unchanged
        );
    }
}

#[cfg(test)]
mod view_delta_tests {
    use super::{ViewDeltaCause, ViewDeltaEffect, classify_view_delta};
    use crate::{
        gui::types::{Point, Rect, Vector2},
        layout::{ContainerKind, ContainerPolicy},
        runtime::{EventMapper, LayerKind, SurfaceChild, SurfaceLayer, SurfaceNode, UiSurface},
        widgets::{
            Widget, WidgetCommon, WidgetInput, WidgetOutput, WidgetRevision, WidgetStyle,
            WidgetTone,
        },
    };

    fn surface(root: SurfaceNode<()>) -> UiSurface<()> {
        UiSurface::new(root)
    }

    #[test]
    fn identical_synthetic_tree_is_unchanged() {
        let root = SurfaceNode::container(
            1,
            ContainerPolicy::default(),
            vec![SurfaceChild::fill(SurfaceNode::overlay_marker(
                2,
                Rect::from_min_size(Point::new(1.0, 2.0), Vector2::new(3.0, 4.0)),
                WidgetStyle::normal(WidgetTone::Neutral),
            ))],
        );
        let previous = surface(root.clone());
        let current = surface(root);
        let delta = classify_view_delta(&previous, &current);
        assert_eq!(delta.effect, ViewDeltaEffect::Unchanged);
        assert_eq!(delta.total_events, 0);
        assert_eq!(delta.event_count, 0);
    }

    #[test]
    fn container_policy_and_overlay_style_choose_broadest_effect() {
        let previous = surface(SurfaceNode::styled_container(
            1,
            ContainerPolicy::default(),
            WidgetStyle::normal(WidgetTone::Neutral),
            vec![SurfaceChild::fill(SurfaceNode::overlay_marker(
                2,
                Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(4.0, 4.0)),
                WidgetStyle::normal(WidgetTone::Neutral),
            ))],
        ));
        let current = surface(SurfaceNode::styled_container(
            1,
            ContainerPolicy {
                kind: ContainerKind::Row,
                ..ContainerPolicy::default()
            },
            WidgetStyle::strong(WidgetTone::Accent),
            vec![SurfaceChild::fill(SurfaceNode::overlay_marker(
                2,
                Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(4.0, 4.0)),
                WidgetStyle::strong(WidgetTone::Accent),
            ))],
        ));
        let delta = classify_view_delta(&previous, &current);
        assert_eq!(delta.effect, ViewDeltaEffect::Geometry);
        assert!(delta.total_events >= 3);
        assert!(
            delta.events[..usize::from(delta.event_count)]
                .iter()
                .flatten()
                .any(|event| event.cause == ViewDeltaCause::ContainerPolicy)
        );
    }

    #[test]
    fn scene_layers_compare_in_canonical_order_and_input_presence_is_structural() {
        let base = SurfaceNode::overlay_marker(
            1,
            Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(2.0, 2.0)),
            WidgetStyle::normal(WidgetTone::Neutral),
        );
        let previous = surface(SurfaceNode::scene(
            10,
            base.clone(),
            vec![SurfaceLayer::new(LayerKind::Tooltip, base.clone())],
        ));
        let current = surface(SurfaceNode::scene(
            10,
            base,
            vec![SurfaceLayer::with_input(
                LayerKind::Tooltip,
                Some(SurfaceNode::overlay_marker(
                    3,
                    Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(2.0, 2.0)),
                    WidgetStyle::normal(WidgetTone::Neutral),
                )),
                SurfaceNode::overlay_marker(
                    2,
                    Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(2.0, 2.0)),
                    WidgetStyle::normal(WidgetTone::Neutral),
                ),
            )],
        ));
        let delta = classify_view_delta(&previous, &current);
        assert_eq!(delta.effect, ViewDeltaEffect::Structural);
        assert!(
            delta.events[..usize::from(delta.event_count)]
                .iter()
                .flatten()
                .any(|event| event.cause == ViewDeltaCause::SceneLayerInput)
        );
    }

    #[test]
    fn event_and_path_bounds_are_saturating_and_fixed() {
        let previous_children = (0..20)
            .map(|id| {
                SurfaceChild::fill(SurfaceNode::overlay_marker(
                    id,
                    Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(2.0, 2.0)),
                    WidgetStyle::normal(WidgetTone::Neutral),
                ))
            })
            .collect();
        let current_children = (0..20)
            .map(|id| {
                SurfaceChild::fill(SurfaceNode::overlay_marker(
                    id,
                    Rect::from_min_size(Point::new(1.0, 0.0), Vector2::new(2.0, 2.0)),
                    WidgetStyle::normal(WidgetTone::Neutral),
                ))
            })
            .collect();
        let delta = classify_view_delta(
            &surface(SurfaceNode::container(
                100,
                ContainerPolicy::default(),
                previous_children,
            )),
            &surface(SurfaceNode::container(
                100,
                ContainerPolicy::default(),
                current_children,
            )),
        );
        assert_eq!(delta.effect, ViewDeltaEffect::Geometry);
        assert_eq!(delta.event_count, 16);
        assert_eq!(delta.total_events, 20);
        assert_eq!(delta.omitted_events, 4);

        fn deep(rect_x: f32) -> SurfaceNode<()> {
            let mut node = SurfaceNode::overlay_marker(
                900,
                Rect::from_min_size(Point::new(rect_x, 0.0), Vector2::new(2.0, 2.0)),
                WidgetStyle::normal(WidgetTone::Neutral),
            );
            for id in 0..10 {
                node = SurfaceNode::container(
                    id,
                    ContainerPolicy::default(),
                    vec![SurfaceChild::fill(node)],
                );
            }
            node
        }
        let delta = classify_view_delta(&surface(deep(0.0)), &surface(deep(1.0)));
        assert!(delta.truncated_paths);
        assert!(
            delta.events[..usize::from(delta.event_count)]
                .iter()
                .flatten()
                .all(|event| event.path.len <= 8)
        );
    }

    #[test]
    fn floating_interaction_is_narrower_than_embedded_geometry() {
        let previous = surface(SurfaceNode::floating_layer(
            1,
            Point::new(0.0, 0.0),
            Vector2::new(20.0, 20.0),
            SurfaceNode::overlay_marker(
                2,
                Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(2.0, 2.0)),
                WidgetStyle::normal(WidgetTone::Neutral),
            ),
            false,
        ));
        let current = surface(SurfaceNode::floating_layer(
            1,
            Point::new(0.0, 0.0),
            Vector2::new(20.0, 20.0),
            SurfaceNode::overlay_marker(
                2,
                Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(2.0, 2.0)),
                WidgetStyle::normal(WidgetTone::Neutral),
            ),
            true,
        ));
        let delta = classify_view_delta(&previous, &current);
        assert_eq!(delta.effect, ViewDeltaEffect::Interaction);
        assert!(
            delta.events[..usize::from(delta.event_count)]
                .iter()
                .flatten()
                .any(|event| event.cause == ViewDeltaCause::FloatingInteractive)
        );
    }

    #[derive(PartialEq, Eq)]
    struct MapperRevision(u32);

    fn scroll_surface(
        mapper: Option<EventMapper<crate::runtime::ScrollUpdate, Option<()>>>,
    ) -> UiSurface<()> {
        let node = SurfaceNode::column(1, 0.0, Vec::new());
        let node = match mapper {
            Some(mapper) => node.with_scroll_message_mapped(mapper),
            None => node,
        };
        surface(node)
    }

    #[test]
    fn exact_scroll_mappers_compare_as_unchanged_or_interaction() {
        let previous = scroll_surface(Some(EventMapper::with_revision(MapperRevision(1), |_| {
            None
        })));
        let equal = scroll_surface(Some(EventMapper::with_revision(MapperRevision(1), |_| {
            Some(())
        })));
        let changed = scroll_surface(Some(EventMapper::with_revision(MapperRevision(2), |_| {
            None
        })));
        assert_eq!(
            classify_view_delta(&previous, &equal).effect,
            ViewDeltaEffect::Unchanged
        );
        assert_eq!(
            classify_view_delta(&previous, &changed).effect,
            ViewDeltaEffect::Interaction
        );
        assert_eq!(
            classify_view_delta(&previous, &scroll_surface(None)).effect,
            ViewDeltaEffect::Interaction
        );
    }

    #[test]
    fn conservative_scroll_mapper_stays_structural() {
        let conservative = scroll_surface(Some(EventMapper::new(|_| None)));
        let absent = scroll_surface(None);
        assert_eq!(
            classify_view_delta(&conservative, &absent).effect,
            ViewDeltaEffect::Structural
        );
        assert_eq!(
            classify_view_delta(&absent, &conservative).effect,
            ViewDeltaEffect::Structural
        );
    }

    #[derive(Clone)]
    struct RevisionWidget {
        common: WidgetCommon,
        revision: WidgetRevision,
    }

    impl RevisionWidget {
        fn new(id: u64) -> Self {
            Self {
                common: WidgetCommon::fixed(id, 40.0, 20.0),
                revision: WidgetRevision::exact(1_u8, 1_u8, 1_u8, 1_u8),
            }
        }
    }

    impl Widget for RevisionWidget {
        fn revision(&self) -> WidgetRevision {
            self.revision.clone()
        }

        fn common(&self) -> &WidgetCommon {
            &self.common
        }

        fn common_mut(&mut self) -> &mut WidgetCommon {
            &mut self.common
        }

        fn handle_input(
            &mut self,
            _bounds: crate::gui::types::Rect,
            _input: WidgetInput,
        ) -> Option<WidgetOutput> {
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

    fn mapped_widget_surface(
        output: Option<EventMapper<WidgetOutput, Option<()>>>,
    ) -> UiSurface<()> {
        let mapper = output
            .map(crate::runtime::WidgetMessageMapper::dynamic_mapped)
            .unwrap_or_else(crate::runtime::WidgetMessageMapper::none);
        surface(SurfaceNode::widget(RevisionWidget::new(1), mapper))
    }

    #[test]
    fn exact_output_mappers_are_unchanged_or_interaction() {
        let previous =
            mapped_widget_surface(Some(EventMapper::with_revision(MapperRevision(1), |_| {
                None
            })));
        let equal =
            mapped_widget_surface(Some(EventMapper::with_revision(MapperRevision(1), |_| {
                Some(())
            })));
        let changed =
            mapped_widget_surface(Some(EventMapper::with_revision(MapperRevision(2), |_| {
                None
            })));
        assert_eq!(
            classify_view_delta(&previous, &equal).effect,
            ViewDeltaEffect::Unchanged
        );
        assert_eq!(
            classify_view_delta(&previous, &changed).effect,
            ViewDeltaEffect::Interaction
        );
        assert_eq!(
            classify_view_delta(&previous, &mapped_widget_surface(None)).effect,
            ViewDeltaEffect::Interaction
        );
    }

    #[test]
    fn native_drop_mapper_evidence_is_classified() {
        let base = SurfaceNode::static_widget(RevisionWidget::new(1));
        let previous = surface(
            base.clone()
                .with_native_file_drop_mapped(EventMapper::with_revision(
                    MapperRevision(1),
                    |_| (),
                )),
        );
        let equal = surface(
            base.clone()
                .with_native_file_drop_mapped(EventMapper::with_revision(
                    MapperRevision(1),
                    |_| (),
                )),
        );
        let changed = surface(
            base.clone()
                .with_native_file_drop_mapped(EventMapper::with_revision(
                    MapperRevision(2),
                    |_| (),
                )),
        );
        assert_eq!(
            classify_view_delta(&previous, &equal).effect,
            ViewDeltaEffect::Unchanged
        );
        assert_eq!(
            classify_view_delta(&previous, &changed).effect,
            ViewDeltaEffect::Interaction
        );
    }
}
