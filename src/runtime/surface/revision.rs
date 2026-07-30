//! Pure widget revision relation used by future incremental reconciliation.

// This relation is deliberately staged without a production caller. Keep the
// pure foundation available for contract tests until incremental refresh owns
// its invocation.
#![allow(dead_code)]

use super::widget::{MapperDescriptor, MapperRelation};
use crate::layout::{ContainerPolicy, SlotParams};
use crate::widgets::WidgetStyle;
use crate::widgets::{WidgetId, WidgetRevision, WidgetRevisionComponents};

/// Borrowed revision inputs for one slot-owned child.
#[derive(Clone, Copy)]
pub(crate) struct SurfaceChildRevision<'a, Message> {
    pub(crate) slot: &'a SlotParams,
    pub(crate) child: &'a super::SurfaceNode<Message>,
}

/// Borrowed revision inputs for one surface container.
pub(crate) struct SurfaceContainerRevision<'a, Message> {
    pub(crate) policy: &'a ContainerPolicy,
    pub(crate) style: Option<&'a WidgetStyle>,
    pub(crate) hoverable: bool,
    pub(crate) scroll_mapper: MapperDescriptor,
    pub(crate) children: &'a [super::SurfaceChild<Message>],
}

/// Borrowed revision inputs for one canonical scene layer.
#[derive(Clone, Copy)]
pub(crate) struct SurfaceLayerRevision<'a, Message> {
    pub(crate) kind: super::LayerKind,
    pub(crate) input: Option<&'a super::SurfaceNode<Message>>,
    pub(crate) node: &'a super::SurfaceNode<Message>,
}

/// Borrowed revision inputs for a scene's canonical layer topology.
pub(crate) struct SurfaceSceneRevision<'a, Message> {
    pub(crate) base: &'a super::SurfaceNode<Message>,
    pub(crate) layers: &'a [super::SurfaceLayer<Message>],
}

impl<'a, Message> SurfaceContainerRevision<'a, Message> {
    fn policy_changed(&self, other: &Self) -> bool {
        self.policy != other.policy
    }

    fn style_changed(&self, other: &Self) -> bool {
        self.style != other.style
    }

    fn hoverability_changed(&self, other: &Self) -> bool {
        self.hoverable != other.hoverable
    }

    fn scroll_mapper_relation(&self, other: &Self) -> MapperRelation {
        self.scroll_mapper.relation(&other.scroll_mapper)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SurfaceLayerRelation {
    Matched {
        previous_index: usize,
        input_changed: bool,
    },
    Added,
    Replaced {
        input_changed: bool,
    },
    KindChanged {
        previous_index: usize,
    },
    Ambiguous,
}

impl<Message> super::SurfaceChild<Message> {
    pub(crate) fn revision(&self) -> SurfaceChildRevision<'_, Message> {
        SurfaceChildRevision {
            slot: &self.slot,
            child: &self.child,
        }
    }
}

impl<Message> super::SurfaceContainer<Message> {
    pub(crate) fn revision(&self) -> SurfaceContainerRevision<'_, Message> {
        SurfaceContainerRevision {
            policy: &self.policy,
            style: self.style.as_ref(),
            hoverable: self.hoverable,
            scroll_mapper: self.scroll_mapper_descriptor(),
            children: &self.children,
        }
    }
}

impl<Message> super::SurfaceScene<Message> {
    pub(crate) fn revision(&self) -> SurfaceSceneRevision<'_, Message> {
        SurfaceSceneRevision {
            base: &self.base,
            layers: &self.layers,
        }
    }
}

impl<'a, Message> SurfaceSceneRevision<'a, Message> {
    fn canonical_layer_count(&self) -> usize {
        self.layers.len()
    }

    fn canonical_layer_at(&self, ordinal: usize) -> Option<SurfaceLayerRevision<'a, Message>> {
        let mut remaining = ordinal;
        for kind in super::LayerKind::ORDER {
            for layer in self.layers.iter().filter(|layer| layer.kind == kind) {
                if remaining == 0 {
                    return Some(SurfaceLayerRevision {
                        kind: layer.kind,
                        input: layer.input.as_ref(),
                        node: &layer.node,
                    });
                }
                remaining -= 1;
            }
        }
        None
    }

    fn layer_count_changed(&self, other: &Self) -> bool {
        self.canonical_layer_count() != other.canonical_layer_count()
    }

    fn topology_ambiguous(&self, other: &Self) -> bool {
        has_duplicate_layer_keys(self)
            || has_duplicate_layer_keys(other)
            || has_duplicate_layer_node_ids(self)
            || has_duplicate_layer_node_ids(other)
    }

    fn layer_order_inverted(&self, current: &Self) -> bool {
        for current_index in 0..current.canonical_layer_count() {
            let Some(current_layer) = current.canonical_layer_at(current_index) else {
                continue;
            };
            let Some(previous_index) = find_layer_node_unique(self, current_layer.node.id()) else {
                continue;
            };
            for later_index in (current_index + 1)..current.canonical_layer_count() {
                let Some(later_layer) = current.canonical_layer_at(later_index) else {
                    continue;
                };
                let Some(later_previous_index) =
                    find_layer_node_unique(self, later_layer.node.id())
                else {
                    continue;
                };
                if previous_index > later_previous_index {
                    return true;
                }
            }
        }
        false
    }

    fn relation_for_current(&self, current: &Self, index: usize) -> SurfaceLayerRelation {
        if self.topology_ambiguous(current) {
            return SurfaceLayerRelation::Ambiguous;
        }
        let Some(current_layer) = current.canonical_layer_at(index) else {
            return SurfaceLayerRelation::Added;
        };
        if let Some(previous_index) = find_layer_key(self, &current_layer) {
            let Some(previous_layer) = self.canonical_layer_at(previous_index) else {
                return SurfaceLayerRelation::Ambiguous;
            };
            return SurfaceLayerRelation::Matched {
                previous_index,
                input_changed: previous_layer.input.is_some() != current_layer.input.is_some(),
            };
        }
        if let Some(previous_index) = find_layer_node_unique(self, current_layer.node.id()) {
            let Some(previous_layer) = self.canonical_layer_at(previous_index) else {
                return SurfaceLayerRelation::Ambiguous;
            };
            if previous_layer.kind != current_layer.kind {
                return SurfaceLayerRelation::KindChanged { previous_index };
            }
        }
        if let Some(previous_layer) = self.canonical_layer_at(index)
            && previous_layer.kind == current_layer.kind
            && find_layer_key(current, &previous_layer).is_none()
        {
            return SurfaceLayerRelation::Replaced {
                input_changed: previous_layer.input.is_some() != current_layer.input.is_some(),
            };
        }
        SurfaceLayerRelation::Added
    }

    fn previous_is_removed(&self, current: &Self, index: usize) -> bool {
        let Some(previous_layer) = self.canonical_layer_at(index) else {
            return false;
        };
        if find_layer_key(current, &previous_layer).is_some() {
            return false;
        }
        if find_layer_node_unique(current, previous_layer.node.id()).is_some() {
            return false;
        }
        if let Some(current_layer) = current.canonical_layer_at(index)
            && current_layer.kind == previous_layer.kind
            && find_layer_key(self, &current_layer).is_none()
        {
            return false;
        }
        true
    }
}

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

/// Classify the optional semantic capability without evaluating semantic
/// output methods such as role, label, value, or metadata accessors.
///
/// Capability presence and unsupported descriptor contracts are structural.
/// A conservative or unavailable revision is also structural; two exact
/// revisions compare by typed equality, with a changed or type-mismatched
/// value taking the interaction path.
fn classify_widget_capabilities(
    previous: crate::widgets::WidgetCapabilities<'_>,
    current: crate::widgets::WidgetCapabilities<'_>,
) -> WidgetRevisionEffect {
    if previous.contract_version != crate::widgets::WIDGET_CAPABILITIES_CONTRACT_VERSION
        || current.contract_version != crate::widgets::WIDGET_CAPABILITIES_CONTRACT_VERSION
    {
        return WidgetRevisionEffect::Structural;
    }
    if previous.has_semantics() != current.has_semantics() {
        return WidgetRevisionEffect::Structural;
    }
    if !previous.has_semantics() {
        return WidgetRevisionEffect::Unchanged;
    }

    let (Some(previous), Some(current)) =
        (previous.semantics_revision(), current.semantics_revision())
    else {
        return WidgetRevisionEffect::Structural;
    };
    if !previous.is_exact() || !current.is_exact() {
        WidgetRevisionEffect::Structural
    } else if previous == current {
        WidgetRevisionEffect::Unchanged
    } else {
        WidgetRevisionEffect::Interaction
    }
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
    Replaced,
    AmbiguousPairing,
    IncompatibleWidget,
    WidgetCapabilities,
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
    let capability_effect = classify_widget_capabilities(
        previous.widget().capabilities(),
        current.widget().capabilities(),
    );
    if capability_effect != WidgetRevisionEffect::Unchanged {
        delta.record(
            capability_effect.into(),
            ViewDeltaCause::WidgetCapabilities,
            path.path,
        );
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
    let previous_revision = previous_container.revision();
    let current_revision = current_container.revision();
    if previous_revision.policy_changed(&current_revision) {
        delta.record(
            ViewDeltaEffect::Geometry,
            ViewDeltaCause::ContainerPolicy,
            path.path,
        );
    }
    if previous_revision.style_changed(&current_revision) {
        delta.record(
            ViewDeltaEffect::Paint,
            ViewDeltaCause::ContainerStyle,
            path.path,
        );
    }
    if previous_revision.hoverability_changed(&current_revision) {
        delta.record(
            ViewDeltaEffect::Interaction,
            ViewDeltaCause::ContainerHover,
            path.path,
        );
    }
    match previous_revision.scroll_mapper_relation(&current_revision) {
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
    compare_container_children(&previous_revision, &current_revision, path, delta);
}

fn child_index<Message>(children: &[super::SurfaceChild<Message>], id: WidgetId) -> Option<usize> {
    children.iter().position(|child| child.child.id() == id)
}

fn has_duplicate_child_ids<Message>(children: &[super::SurfaceChild<Message>]) -> bool {
    children.iter().enumerate().any(|(index, child)| {
        children[index + 1..]
            .iter()
            .any(|candidate| candidate.child.id() == child.child.id())
    })
}

fn child_order_inverted<Message>(
    previous: &[super::SurfaceChild<Message>],
    current: &[super::SurfaceChild<Message>],
) -> bool {
    for (current_index, current_child) in current.iter().enumerate() {
        let Some(previous_index) = child_index(previous, current_child.child.id()) else {
            continue;
        };
        for later_child in &current[current_index + 1..] {
            let Some(later_previous_index) = child_index(previous, later_child.child.id()) else {
                continue;
            };
            if previous_index > later_previous_index {
                return true;
            }
        }
    }
    false
}

fn compare_container_children<Message>(
    previous: &SurfaceContainerRevision<'_, Message>,
    current: &SurfaceContainerRevision<'_, Message>,
    path: &mut PathStack,
    delta: &mut ViewDelta,
) {
    if has_duplicate_child_ids(previous.children) || has_duplicate_child_ids(current.children) {
        delta.record(
            ViewDeltaEffect::Structural,
            ViewDeltaCause::AmbiguousPairing,
            path.path,
        );
        return;
    }

    if previous.children.len() != current.children.len() {
        let cause = if previous.children.len() < current.children.len() {
            ViewDeltaCause::Added
        } else {
            ViewDeltaCause::Removed
        };
        delta.record(ViewDeltaEffect::Structural, cause, path.path);
    }

    let reordered = child_order_inverted(previous.children, current.children);
    let mut reordered_recorded = false;
    for (index, current_child) in current.children.iter().enumerate() {
        path.push(ViewDeltaPathComponent::Child(index as u64));
        let current_revision = current_child.revision();
        if let Some(previous_index) = child_index(previous.children, current_child.child.id()) {
            if reordered && !reordered_recorded {
                delta.record(
                    ViewDeltaEffect::Structural,
                    ViewDeltaCause::Reordered,
                    path.path,
                );
                reordered_recorded = true;
            }
            let previous_revision = previous.children[previous_index].revision();
            if previous_revision.slot != current_revision.slot {
                delta.record(
                    ViewDeltaEffect::Geometry,
                    ViewDeltaCause::ChildSlot,
                    path.path,
                );
            }
            compare_node(previous_revision.child, current_revision.child, path, delta);
        } else if index < previous.children.len()
            && child_index(current.children, previous.children[index].child.id()).is_none()
        {
            delta.record(
                ViewDeltaEffect::Structural,
                ViewDeltaCause::Replaced,
                path.path,
            );
        } else {
            delta.record(
                ViewDeltaEffect::Structural,
                ViewDeltaCause::Added,
                path.path,
            );
        }
        path.pop();
    }

    for (index, previous_child) in previous.children.iter().enumerate() {
        if child_index(current.children, previous_child.child.id()).is_some() {
            continue;
        }
        if index < current.children.len()
            && child_index(previous.children, current.children[index].child.id()).is_none()
        {
            continue;
        }
        delta.record(
            ViewDeltaEffect::Structural,
            ViewDeltaCause::Removed,
            path.path,
        );
    }
}

fn compare_scene<Message>(
    previous: &super::SurfaceScene<Message>,
    current: &super::SurfaceScene<Message>,
    path: &mut PathStack,
    delta: &mut ViewDelta,
) {
    let previous_revision = previous.revision();
    let current_revision = current.revision();
    compare_node(previous_revision.base, current_revision.base, path, delta);
    let previous_count = previous_revision.canonical_layer_count();
    let current_count = current_revision.canonical_layer_count();
    if previous_revision.layer_count_changed(&current_revision) {
        delta.record(
            ViewDeltaEffect::Structural,
            ViewDeltaCause::SceneLayerCount,
            path.path,
        );
    }
    if previous_revision.topology_ambiguous(&current_revision) {
        delta.record(
            ViewDeltaEffect::Structural,
            ViewDeltaCause::AmbiguousPairing,
            path.path,
        );
        return;
    }

    let mut reordered_recorded = false;
    let layer_order_inverted = previous_revision.layer_order_inverted(&current_revision);
    for index in 0..current_count {
        path.push(ViewDeltaPathComponent::Layer(index as u64));
        match previous_revision.relation_for_current(&current_revision, index) {
            SurfaceLayerRelation::Matched {
                previous_index,
                input_changed,
            } => {
                let Some(previous_layer) = previous_revision.canonical_layer_at(previous_index)
                else {
                    path.pop();
                    continue;
                };
                let Some(current_layer) = current_revision.canonical_layer_at(index) else {
                    path.pop();
                    continue;
                };
                if layer_order_inverted && !reordered_recorded {
                    delta.record(
                        ViewDeltaEffect::Structural,
                        ViewDeltaCause::Reordered,
                        path.path,
                    );
                    reordered_recorded = true;
                }
                if input_changed {
                    delta.record(
                        ViewDeltaEffect::Structural,
                        ViewDeltaCause::SceneLayerInput,
                        path.path,
                    );
                } else {
                    compare_layer_pair(previous_layer, current_layer, path, delta);
                }
            }
            SurfaceLayerRelation::Replaced { input_changed } => {
                delta.record(
                    ViewDeltaEffect::Structural,
                    ViewDeltaCause::Replaced,
                    path.path,
                );
                if input_changed {
                    delta.record(
                        ViewDeltaEffect::Structural,
                        ViewDeltaCause::SceneLayerInput,
                        path.path,
                    );
                }
            }
            SurfaceLayerRelation::KindChanged { .. } => {
                delta.record(
                    ViewDeltaEffect::Structural,
                    ViewDeltaCause::SceneLayerKind,
                    path.path,
                );
            }
            SurfaceLayerRelation::Added => {
                delta.record(
                    ViewDeltaEffect::Structural,
                    ViewDeltaCause::Added,
                    path.path,
                );
            }
            SurfaceLayerRelation::Ambiguous => {
                delta.record(
                    ViewDeltaEffect::Structural,
                    ViewDeltaCause::AmbiguousPairing,
                    path.path,
                );
            }
        }
        path.pop();
    }

    for index in 0..previous_count {
        if previous_revision.canonical_layer_at(index).is_none() {
            continue;
        }
        if !previous_revision.previous_is_removed(&current_revision, index) {
            continue;
        }
        delta.record(
            ViewDeltaEffect::Structural,
            ViewDeltaCause::Removed,
            path.path,
        );
    }
}

fn compare_layer_pair<Message>(
    previous: SurfaceLayerRevision<'_, Message>,
    current: SurfaceLayerRevision<'_, Message>,
    path: &mut PathStack,
    delta: &mut ViewDelta,
) {
    if let (Some(previous), Some(current)) = (previous.input, current.input) {
        path.push(ViewDeltaPathComponent::Input);
        compare_node(previous, current, path, delta);
        path.pop();
    }
    path.push(ViewDeltaPathComponent::Foreground);
    compare_node(previous.node, current.node, path, delta);
    path.pop();
}

fn layer_key<Message>(layer: &SurfaceLayerRevision<'_, Message>) -> (super::LayerKind, WidgetId) {
    (layer.kind, layer.node.id())
}

fn find_layer_key<Message>(
    scene: &SurfaceSceneRevision<'_, Message>,
    needle: &SurfaceLayerRevision<'_, Message>,
) -> Option<usize> {
    (0..scene.canonical_layer_count()).find(|index| {
        scene
            .canonical_layer_at(*index)
            .is_some_and(|layer| layer_key(&layer) == layer_key(needle))
    })
}

fn find_layer_node_unique<Message>(
    scene: &SurfaceSceneRevision<'_, Message>,
    node_id: WidgetId,
) -> Option<usize> {
    let mut found = None;
    for index in 0..scene.canonical_layer_count() {
        let Some(layer) = scene.canonical_layer_at(index) else {
            continue;
        };
        if layer.node.id() != node_id {
            continue;
        }
        if found.is_some() {
            return None;
        }
        found = Some(index);
    }
    found
}

fn has_duplicate_layer_keys<Message>(scene: &SurfaceSceneRevision<'_, Message>) -> bool {
    (0..scene.canonical_layer_count()).any(|index| {
        let Some(layer) = scene.canonical_layer_at(index) else {
            return false;
        };
        ((index + 1)..scene.canonical_layer_count()).any(|candidate| {
            scene
                .canonical_layer_at(candidate)
                .is_some_and(|candidate| layer_key(&candidate) == layer_key(&layer))
        })
    })
}

fn has_duplicate_layer_node_ids<Message>(scene: &SurfaceSceneRevision<'_, Message>) -> bool {
    (0..scene.canonical_layer_count()).any(|index| {
        let Some(layer) = scene.canonical_layer_at(index) else {
            return false;
        };
        ((index + 1)..scene.canonical_layer_count()).any(|candidate| {
            scene
                .canonical_layer_at(candidate)
                .is_some_and(|candidate| candidate.node.id() == layer.node.id())
        })
    })
}

#[cfg(test)]
mod tests {
    use super::{
        WidgetRevisionEffect, WidgetRevisionSnapshot, classify_widget_capabilities,
        classify_widget_revision,
    };
    use crate::layout::Vector2;
    use crate::widgets::{
        TextWidget, WIDGET_CAPABILITIES_CONTRACT_VERSION, Widget, WidgetCapabilities,
        WidgetRevision, WidgetSemantics, WidgetSemanticsRevision, WidgetSizing,
    };

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

    struct TestSemantics {
        revision: WidgetSemanticsRevision,
    }

    impl WidgetSemantics for TestSemantics {
        fn revision(&self) -> WidgetSemanticsRevision {
            self.revision.clone()
        }

        fn automation_label(&self) -> Option<String> {
            std::panic::panic_any("semantic output must not be evaluated by the classifier")
        }
    }

    fn capabilities(semantics: Option<&TestSemantics>) -> WidgetCapabilities<'_> {
        semantics.map_or_else(WidgetCapabilities::none, |semantics| {
            WidgetCapabilities::new().semantics(semantics)
        })
    }

    #[test]
    fn semantic_capability_classifier_is_conservative_for_presence_contract_and_unavailable() {
        let exact = TestSemantics {
            revision: WidgetSemanticsRevision::exact("label"),
        };
        assert_eq!(
            classify_widget_capabilities(capabilities(None), capabilities(Some(&exact))),
            WidgetRevisionEffect::Structural
        );
        assert_eq!(
            classify_widget_capabilities(
                WidgetCapabilities {
                    contract_version: WIDGET_CAPABILITIES_CONTRACT_VERSION + 1,
                    semantics: None,
                },
                WidgetCapabilities::none(),
            ),
            WidgetRevisionEffect::Structural
        );
        let conservative = TestSemantics {
            revision: WidgetSemanticsRevision::conservative(),
        };
        assert_eq!(
            classify_widget_capabilities(
                capabilities(Some(&conservative)),
                capabilities(Some(&exact)),
            ),
            WidgetRevisionEffect::Structural
        );
    }

    #[test]
    fn exact_semantic_capability_evidence_is_interaction_scoped_and_does_not_call_outputs() {
        let previous = TestSemantics {
            revision: WidgetSemanticsRevision::exact("label"),
        };
        let equal = TestSemantics {
            revision: WidgetSemanticsRevision::exact("label"),
        };
        let changed = TestSemantics {
            revision: WidgetSemanticsRevision::exact("changed"),
        };
        let mismatched = TestSemantics {
            revision: WidgetSemanticsRevision::exact(1_u32),
        };

        assert_eq!(
            classify_widget_capabilities(capabilities(Some(&previous)), capabilities(Some(&equal)),),
            WidgetRevisionEffect::Unchanged
        );
        assert_eq!(
            classify_widget_capabilities(
                capabilities(Some(&previous)),
                capabilities(Some(&changed)),
            ),
            WidgetRevisionEffect::Interaction
        );
        assert_eq!(
            classify_widget_capabilities(
                capabilities(Some(&previous)),
                capabilities(Some(&mismatched)),
            ),
            WidgetRevisionEffect::Interaction
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
            Widget, WidgetCapabilities, WidgetCommon, WidgetInput, WidgetOutput, WidgetRevision,
            WidgetSemantics, WidgetSemanticsRevision, WidgetStyle, WidgetTone,
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

    fn has_cause(delta: &super::ViewDelta, cause: ViewDeltaCause) -> bool {
        delta.events[..usize::from(delta.event_count)]
            .iter()
            .flatten()
            .any(|event| event.cause == cause)
    }

    fn child(id: u64) -> SurfaceChild<()> {
        SurfaceChild::fill(SurfaceNode::overlay_marker(
            id,
            Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(2.0, 2.0)),
            WidgetStyle::normal(WidgetTone::Neutral),
        ))
    }

    #[test]
    fn child_topology_reports_reorder_and_matches_by_identity() {
        let previous = surface(SurfaceNode::container(
            1,
            ContainerPolicy::default(),
            vec![child(2), child(3)],
        ));
        let current = surface(SurfaceNode::container(
            1,
            ContainerPolicy::default(),
            vec![child(3), child(2)],
        ));
        let delta = classify_view_delta(&previous, &current);
        assert_eq!(delta.effect, ViewDeltaEffect::Structural);
        assert!(has_cause(&delta, ViewDeltaCause::Reordered));
        assert!(!has_cause(&delta, ViewDeltaCause::Replaced));
    }

    #[test]
    fn child_insertion_and_removal_do_not_look_like_reorder() {
        let previous = surface(SurfaceNode::container(
            1,
            ContainerPolicy::default(),
            vec![child(2), child(3)],
        ));
        let inserted = surface(SurfaceNode::container(
            1,
            ContainerPolicy::default(),
            vec![child(2), child(4), child(3)],
        ));
        let delta = classify_view_delta(&previous, &inserted);
        assert_eq!(delta.effect, ViewDeltaEffect::Structural);
        assert!(has_cause(&delta, ViewDeltaCause::Added));
        assert!(!has_cause(&delta, ViewDeltaCause::Reordered));

        let removed = surface(SurfaceNode::container(
            1,
            ContainerPolicy::default(),
            vec![child(2), child(3)],
        ));
        let delta = classify_view_delta(&inserted, &removed);
        assert_eq!(delta.effect, ViewDeltaEffect::Structural);
        assert!(has_cause(&delta, ViewDeltaCause::Removed));
        assert!(!has_cause(&delta, ViewDeltaCause::Reordered));
    }

    #[test]
    fn retained_child_slot_changes_are_geometry() {
        let previous = surface(SurfaceNode::container(
            1,
            ContainerPolicy::default(),
            vec![child(2)],
        ));
        let current = surface(SurfaceNode::container(
            1,
            ContainerPolicy::default(),
            vec![SurfaceChild::new(
                crate::layout::SlotParams {
                    margin: crate::layout::Insets::all(2.0),
                    ..crate::layout::SlotParams::fill()
                },
                child(2).child,
            )],
        ));
        let delta = classify_view_delta(&previous, &current);
        assert_eq!(delta.effect, ViewDeltaEffect::Geometry);
        assert!(has_cause(&delta, ViewDeltaCause::ChildSlot));
    }

    #[test]
    fn child_topology_reports_replacement_and_ambiguous_pairing_conservatively() {
        let previous = surface(SurfaceNode::container(
            1,
            ContainerPolicy::default(),
            vec![child(2)],
        ));
        let replacement = surface(SurfaceNode::container(
            1,
            ContainerPolicy::default(),
            vec![child(3)],
        ));
        let delta = classify_view_delta(&previous, &replacement);
        assert_eq!(delta.effect, ViewDeltaEffect::Structural);
        assert!(has_cause(&delta, ViewDeltaCause::Replaced));

        let ambiguous = surface(SurfaceNode::container(
            1,
            ContainerPolicy::default(),
            vec![child(2), child(2)],
        ));
        let delta = classify_view_delta(&previous, &ambiguous);
        assert_eq!(delta.effect, ViewDeltaEffect::Structural);
        assert!(has_cause(&delta, ViewDeltaCause::AmbiguousPairing));
    }

    #[test]
    fn container_hoverability_is_interaction() {
        let previous = surface(SurfaceNode::column(1, 0.0, Vec::new()));
        let current =
            surface(SurfaceNode::column(1, 0.0, Vec::new()).with_container_hoverable(true));
        let delta = classify_view_delta(&previous, &current);
        assert_eq!(delta.effect, ViewDeltaEffect::Interaction);
        assert!(has_cause(&delta, ViewDeltaCause::ContainerHover));
    }

    #[test]
    fn canonical_layers_preserve_cross_kind_order_but_report_same_kind_reorder() {
        let base = SurfaceNode::overlay_marker(
            1,
            Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(2.0, 2.0)),
            WidgetStyle::normal(WidgetTone::Neutral),
        );
        let previous = surface(SurfaceNode::scene(
            10,
            base.clone(),
            vec![
                SurfaceLayer::new(LayerKind::Tooltip, child(3).child),
                SurfaceLayer::new(LayerKind::Floating, child(2).child),
            ],
        ));
        let cross_kind_same = surface(SurfaceNode::scene(
            10,
            base.clone(),
            vec![
                SurfaceLayer::new(LayerKind::Floating, child(2).child),
                SurfaceLayer::new(LayerKind::Tooltip, child(3).child),
            ],
        ));
        assert_eq!(
            classify_view_delta(&previous, &cross_kind_same).effect,
            ViewDeltaEffect::Unchanged
        );

        let previous = surface(SurfaceNode::scene(
            10,
            base.clone(),
            vec![
                SurfaceLayer::new(LayerKind::Tooltip, child(2).child),
                SurfaceLayer::new(LayerKind::Tooltip, child(3).child),
            ],
        ));
        let current = surface(SurfaceNode::scene(
            10,
            base,
            vec![
                SurfaceLayer::new(LayerKind::Tooltip, child(3).child),
                SurfaceLayer::new(LayerKind::Tooltip, child(2).child),
            ],
        ));
        let delta = classify_view_delta(&previous, &current);
        assert_eq!(delta.effect, ViewDeltaEffect::Structural);
        assert!(has_cause(&delta, ViewDeltaCause::Reordered));
    }

    #[test]
    fn scene_layer_insertion_removal_and_kind_moves_pair_conservatively() {
        let base = SurfaceNode::overlay_marker(
            1,
            Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(2.0, 2.0)),
            WidgetStyle::normal(WidgetTone::Neutral),
        );
        let previous = surface(SurfaceNode::scene(
            10,
            base.clone(),
            vec![SurfaceLayer::new(LayerKind::Floating, child(2).child)],
        ));
        let inserted = surface(SurfaceNode::scene(
            10,
            base.clone(),
            vec![
                SurfaceLayer::new(LayerKind::Floating, child(2).child),
                SurfaceLayer::new(LayerKind::Tooltip, child(3).child),
            ],
        ));
        let delta = classify_view_delta(&previous, &inserted);
        assert_eq!(delta.effect, ViewDeltaEffect::Structural);
        assert!(has_cause(&delta, ViewDeltaCause::Added));
        assert!(!has_cause(&delta, ViewDeltaCause::Reordered));
        assert!(!has_cause(&delta, ViewDeltaCause::SceneLayerKind));

        let unmatched_kind = surface(SurfaceNode::scene(
            10,
            base.clone(),
            vec![SurfaceLayer::new(LayerKind::Modal, child(3).child)],
        ));
        let delta = classify_view_delta(&previous, &unmatched_kind);
        assert!(has_cause(&delta, ViewDeltaCause::Added));
        assert!(has_cause(&delta, ViewDeltaCause::Removed));
        assert!(!has_cause(&delta, ViewDeltaCause::SceneLayerKind));

        let moved_kind = surface(SurfaceNode::scene(
            10,
            base,
            vec![SurfaceLayer::new(LayerKind::Modal, child(2).child)],
        ));
        let delta = classify_view_delta(&previous, &moved_kind);
        assert!(has_cause(&delta, ViewDeltaCause::SceneLayerKind));
        assert!(!has_cause(&delta, ViewDeltaCause::Added));
        assert!(!has_cause(&delta, ViewDeltaCause::Removed));
    }

    #[test]
    fn duplicate_scene_layer_keys_stop_descent_with_ambiguous_pairing() {
        let base = SurfaceNode::overlay_marker(
            1,
            Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(2.0, 2.0)),
            WidgetStyle::normal(WidgetTone::Neutral),
        );
        let previous = surface(SurfaceNode::scene(
            10,
            base.clone(),
            vec![SurfaceLayer::new(LayerKind::Tooltip, child(2).child)],
        ));
        let duplicate = surface(SurfaceNode::scene(
            10,
            base,
            vec![
                SurfaceLayer::new(LayerKind::Tooltip, child(2).child),
                SurfaceLayer::new(LayerKind::Tooltip, child(2).child),
            ],
        ));
        let delta = classify_view_delta(&previous, &duplicate);
        assert_eq!(delta.effect, ViewDeltaEffect::Structural);
        assert!(has_cause(&delta, ViewDeltaCause::AmbiguousPairing));
        assert!(!has_cause(&delta, ViewDeltaCause::Reordered));
        assert!(!has_cause(&delta, ViewDeltaCause::SceneLayerKind));
        assert!(!has_cause(&delta, ViewDeltaCause::SceneLayerInput));
    }

    #[test]
    fn duplicate_scene_foreground_ids_across_kinds_are_ambiguous() {
        let base = SurfaceNode::overlay_marker(
            1,
            Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(2.0, 2.0)),
            WidgetStyle::normal(WidgetTone::Neutral),
        );
        let previous = surface(SurfaceNode::scene(
            10,
            base.clone(),
            vec![SurfaceLayer::new(LayerKind::Floating, child(2).child)],
        ));
        let duplicate = surface(SurfaceNode::scene(
            10,
            base,
            vec![
                SurfaceLayer::new(LayerKind::Floating, child(2).child),
                SurfaceLayer::new(LayerKind::Modal, child(2).child),
            ],
        ));
        let delta = classify_view_delta(&previous, &duplicate);
        assert_eq!(delta.effect, ViewDeltaEffect::Structural);
        assert!(has_cause(&delta, ViewDeltaCause::AmbiguousPairing));
        assert!(!has_cause(&delta, ViewDeltaCause::SceneLayerKind));
        assert!(!has_cause(&delta, ViewDeltaCause::Added));
        assert!(!has_cause(&delta, ViewDeltaCause::Removed));
        assert!(!has_cause(&delta, ViewDeltaCause::NodeIdentity));
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

    #[derive(Clone)]
    struct SemanticRevisionWidget {
        common: WidgetCommon,
        semantics_revision: WidgetSemanticsRevision,
    }

    impl SemanticRevisionWidget {
        fn new(id: u64, semantics_revision: WidgetSemanticsRevision) -> Self {
            Self {
                common: WidgetCommon::fixed(id, 40.0, 20.0),
                semantics_revision,
            }
        }
    }

    impl WidgetSemantics for SemanticRevisionWidget {
        fn revision(&self) -> WidgetSemanticsRevision {
            self.semantics_revision.clone()
        }

        fn automation_label(&self) -> Option<String> {
            std::panic::panic_any("semantic output must not be evaluated by ViewDelta")
        }
    }

    impl Widget for SemanticRevisionWidget {
        fn revision(&self) -> WidgetRevision {
            WidgetRevision::exact((), (), (), ())
        }

        fn capabilities(&self) -> WidgetCapabilities<'_> {
            WidgetCapabilities::new().semantics(self)
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

    fn semantic_revision_surface(revision: WidgetSemanticsRevision) -> UiSurface<()> {
        surface(SurfaceNode::widget(
            SemanticRevisionWidget::new(1, revision),
            crate::runtime::WidgetMessageMapper::none(),
        ))
    }

    #[test]
    fn view_delta_classifies_exact_semantic_changes_as_interaction() {
        let previous = semantic_revision_surface(WidgetSemanticsRevision::exact("before"));
        let equal = semantic_revision_surface(WidgetSemanticsRevision::exact("before"));
        let changed = semantic_revision_surface(WidgetSemanticsRevision::exact("after"));

        assert_eq!(
            classify_view_delta(&previous, &equal).effect,
            ViewDeltaEffect::Unchanged
        );
        let delta = classify_view_delta(&previous, &changed);
        assert_eq!(delta.effect, ViewDeltaEffect::Interaction);
        assert!(
            delta.events[..usize::from(delta.event_count)]
                .iter()
                .flatten()
                .any(|event| event.cause == ViewDeltaCause::WidgetCapabilities)
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
