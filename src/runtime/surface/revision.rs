//! Pure widget revision relation used by future incremental reconciliation.

// This relation is deliberately staged without a production caller. Keep the
// pure foundation available for contract tests until incremental refresh owns
// its invocation.
#![allow(dead_code)]

use super::widget::{
    MapperDescriptor, MapperRelation, SurfaceWidgetRevisionEvidence, WidgetCapabilityEvidence,
};
use crate::layout::{ContainerPolicy, SlotParams};
use crate::widgets::WidgetStyle;
use crate::widgets::{WidgetId, WidgetRevision, WidgetRevisionComponents};
use std::collections::HashSet;
use std::time::Duration;

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
    pub(crate) valid: bool,
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
    if !previous.valid
        || !current.valid
        || previous.id != current.id
        || previous.compatibility_kind != current.compatibility_kind
    {
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

fn classify_cached_widget_revision(
    previous: &SurfaceWidgetRevisionEvidence,
    current: &SurfaceWidgetRevisionEvidence,
) -> WidgetRevisionEffect {
    if !previous.valid
        || !current.valid
        || previous.id != current.id
        || previous.compatibility_kind != current.compatibility_kind
    {
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

fn classify_cached_widget_capabilities(
    previous: &WidgetCapabilityEvidence,
    current: &WidgetCapabilityEvidence,
) -> WidgetRevisionEffect {
    if previous.contract_version != crate::widgets::WIDGET_CAPABILITIES_CONTRACT_VERSION
        || current.contract_version != crate::widgets::WIDGET_CAPABILITIES_CONTRACT_VERSION
    {
        return WidgetRevisionEffect::Structural;
    }
    if previous.semantics_revision.is_some() != current.semantics_revision.is_some() {
        return WidgetRevisionEffect::Structural;
    }
    let (Some(previous), Some(current)) = (
        previous.semantics_revision.as_ref(),
        current.semantics_revision.as_ref(),
    ) else {
        return WidgetRevisionEffect::Unchanged;
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
    InsufficientIdentityEvidence,
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

/// Bounded summary retained by refresh diagnostics for one classifier pass.
///
/// This is deliberately crate-private: the classifier is observational
/// evidence for runtime alignment work, not a refresh-policy API.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ViewDeltaDiagnostics {
    pub(crate) classified: bool,
    pub(crate) duration: Duration,
    pub(crate) effect: ViewDeltaEffect,
    pub(crate) total_events: u32,
    pub(crate) recorded_events: u8,
    pub(crate) omitted_events: u32,
    pub(crate) truncated_paths: bool,
    pub(crate) structural_cause: Option<ViewDeltaCause>,
    /// Whether every recorded change is safe for backend-neutral base paint
    /// plan reuse. This is private evidence, not a public refresh policy API.
    pub(crate) base_paint_reuse_safe: bool,
}

impl Default for ViewDeltaDiagnostics {
    fn default() -> Self {
        Self::startup()
    }
}

impl ViewDeltaDiagnostics {
    pub(crate) const fn startup() -> Self {
        Self {
            classified: false,
            duration: Duration::ZERO,
            effect: ViewDeltaEffect::Unchanged,
            total_events: 0,
            recorded_events: 0,
            omitted_events: 0,
            truncated_paths: false,
            structural_cause: None,
            base_paint_reuse_safe: true,
        }
    }

    pub(crate) fn merge(&mut self, other: Self) {
        if !other.classified {
            return;
        }
        if !self.classified {
            *self = other;
            return;
        }
        self.duration = self.duration.saturating_add(other.duration);
        self.effect = broader_effect(self.effect, other.effect);
        self.total_events = self.total_events.saturating_add(other.total_events);
        self.recorded_events = self.recorded_events.saturating_add(other.recorded_events);
        self.omitted_events = self.omitted_events.saturating_add(other.omitted_events);
        self.truncated_paths |= other.truncated_paths;
        self.base_paint_reuse_safe &= other.base_paint_reuse_safe;
        if self.structural_cause.is_none() {
            self.structural_cause = other.structural_cause;
        }
    }
}

/// Caller-owned identity workspace for allocation-free view-delta scans.
pub(crate) struct ViewDeltaScratch {
    identities: HashSet<WidgetId>,
}

impl ViewDeltaScratch {
    /// Reserve identity capacity before entering classification.
    pub(crate) fn with_capacity(capacity: usize) -> Self {
        Self {
            identities: HashSet::with_capacity(capacity),
        }
    }

    pub(crate) fn has_identity_capacity(&self) -> bool {
        self.identities.capacity() != 0
    }

    fn begin_scan(&mut self, required: usize) -> bool {
        if self.identities.capacity() < required {
            return false;
        }
        self.identities.clear();
        true
    }

    fn insert(&mut self, identity: WidgetId) -> bool {
        self.identities.insert(identity)
    }
}

/// Capacity chosen during runtime construction so refresh-time scans do not
/// allocate. Wider surfaces conservatively record an insufficient-evidence
/// structural result instead.
pub(crate) const DEFAULT_VIEW_DELTA_SCRATCH_CAPACITY: usize = 4096;

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

    pub(crate) fn diagnostics(self, duration: Duration) -> ViewDeltaDiagnostics {
        let structural_cause = self
            .events
            .iter()
            .flatten()
            .find(|event| event.effect == ViewDeltaEffect::Structural)
            .map(|event| event.cause);
        let base_paint_reuse_safe = self.events.iter().flatten().all(|event| {
            matches!(
                event.cause,
                ViewDeltaCause::WidgetCapabilities
                    | ViewDeltaCause::OpaqueWidgetMapper
                    | ViewDeltaCause::WidgetRevision
            )
        });
        ViewDeltaDiagnostics {
            classified: true,
            duration,
            effect: self.effect,
            total_events: self.total_events,
            recorded_events: self.event_count,
            omitted_events: self.omitted_events,
            truncated_paths: self.truncated_paths,
            structural_cause,
            base_paint_reuse_safe,
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

/// Compare two surfaces using caller-owned identity scratch without allocating,
/// mutating, or consulting runtime state. Scratch capacity must be prepared by
/// the caller; insufficient capacity widens the affected container structurally.
pub(crate) fn classify_view_delta<Message>(
    previous: &super::UiSurface<Message>,
    current: &super::UiSurface<Message>,
    scratch: &mut ViewDeltaScratch,
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
        compare_node(
            &previous.root,
            &current.root,
            &mut path,
            &mut delta,
            scratch,
        );
    }
    delta
}

fn compare_node<Message>(
    previous: &super::SurfaceNode<Message>,
    current: &super::SurfaceNode<Message>,
    path: &mut PathStack,
    delta: &mut ViewDelta,
    scratch: &mut ViewDeltaScratch,
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
            compare_scene(previous, current, path, delta, scratch)
        }
        (super::SurfaceNode::Container(previous), super::SurfaceNode::Container(current)) => {
            compare_container(previous, current, path, delta, scratch)
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
            compare_container(
                &previous.container,
                &current.container,
                path,
                delta,
                scratch,
            );
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
    let previous_evidence = previous.revision_evidence();
    let current_evidence = current.revision_evidence();
    if previous_evidence.compatibility_kind != current_evidence.compatibility_kind {
        delta.record(
            ViewDeltaEffect::Structural,
            ViewDeltaCause::IncompatibleWidget,
            path.path,
        );
        return;
    }
    let capability_effect = classify_cached_widget_capabilities(
        &previous_evidence.capabilities,
        &current_evidence.capabilities,
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
    let effect = classify_cached_widget_revision(previous_evidence, current_evidence);
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
    scratch: &mut ViewDeltaScratch,
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
    compare_container_children(&previous_revision, &current_revision, path, delta, scratch);
}

fn compare_container_children<Message>(
    previous: &SurfaceContainerRevision<'_, Message>,
    current: &SurfaceContainerRevision<'_, Message>,
    path: &mut PathStack,
    delta: &mut ViewDelta,
    scratch: &mut ViewDeltaScratch,
) {
    let previous_duplicates = has_duplicate_child_ids(previous.children, scratch);
    let current_duplicates = has_duplicate_child_ids(current.children, scratch);
    if previous_duplicates.is_none() || current_duplicates.is_none() {
        delta.record(
            ViewDeltaEffect::Structural,
            ViewDeltaCause::InsufficientIdentityEvidence,
            path.path,
        );
        return;
    }
    if previous_duplicates == Some(true) || current_duplicates == Some(true) {
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
        return;
    }

    // Pair by ordinal identity exactly once. A mismatch is a topology change;
    // stop at this container instead of searching siblings or descending into
    // potentially unrelated retained nodes.
    for (index, (previous_child, current_child)) in previous
        .children
        .iter()
        .zip(current.children.iter())
        .enumerate()
    {
        path.push(ViewDeltaPathComponent::Child(index as u64));
        let current_revision = current_child.revision();
        if previous_child.child.id() != current_child.child.id() {
            delta.record(
                ViewDeltaEffect::Structural,
                ViewDeltaCause::Reordered,
                path.path,
            );
            path.pop();
            return;
        }
        let previous_revision = previous_child.revision();
        if previous_revision.slot != current_revision.slot {
            delta.record(
                ViewDeltaEffect::Geometry,
                ViewDeltaCause::ChildSlot,
                path.path,
            );
        }
        compare_node(
            previous_revision.child,
            current_revision.child,
            path,
            delta,
            scratch,
        );
        path.pop();
    }
}

fn has_duplicate_child_ids<Message>(
    children: &[super::SurfaceChild<Message>],
    scratch: &mut ViewDeltaScratch,
) -> Option<bool> {
    if !scratch.begin_scan(children.len()) {
        return None;
    }
    Some(
        children
            .iter()
            .any(|child| !scratch.insert(child.child.id())),
    )
}

fn compare_scene<Message>(
    previous: &super::SurfaceScene<Message>,
    current: &super::SurfaceScene<Message>,
    path: &mut PathStack,
    delta: &mut ViewDelta,
    scratch: &mut ViewDeltaScratch,
) {
    let previous_revision = previous.revision();
    let current_revision = current.revision();
    // Validate scene identity/topology before descending into the base tree.
    // A bounded scratch failure or ambiguous layer pairing must not spend work
    // traversing a subtree whose retained relationship is already unsafe.
    let previous_duplicates = has_duplicate_scene_node_ids(&previous_revision, scratch);
    let current_duplicates = has_duplicate_scene_node_ids(&current_revision, scratch);
    if previous_duplicates.is_none() || current_duplicates.is_none() {
        delta.record(
            ViewDeltaEffect::Structural,
            ViewDeltaCause::InsufficientIdentityEvidence,
            path.path,
        );
        return;
    }
    if previous_duplicates == Some(true) || current_duplicates == Some(true) {
        delta.record(
            ViewDeltaEffect::Structural,
            ViewDeltaCause::AmbiguousPairing,
            path.path,
        );
        return;
    }
    if previous_revision.canonical_layer_count() != current_revision.canonical_layer_count() {
        delta.record(
            ViewDeltaEffect::Structural,
            ViewDeltaCause::SceneLayerCount,
            path.path,
        );
        return;
    }
    if !preflight_scene_layers(&previous_revision, &current_revision, path, delta) {
        return;
    }

    compare_node(
        previous_revision.base,
        current_revision.base,
        path,
        delta,
        scratch,
    );

    let mut ordinal = 0_u64;
    for kind in super::LayerKind::ORDER {
        let previous_layers = previous_revision
            .layers
            .iter()
            .filter(|layer| layer.kind == kind);
        let current_layers = current_revision
            .layers
            .iter()
            .filter(|layer| layer.kind == kind);
        for (previous_layer, current_layer) in previous_layers.zip(current_layers) {
            path.push(ViewDeltaPathComponent::Layer(ordinal));
            ordinal = ordinal.saturating_add(1);
            compare_layer_pair(
                SurfaceLayerRevision {
                    kind: previous_layer.kind,
                    input: previous_layer.input.as_ref(),
                    node: &previous_layer.node,
                },
                SurfaceLayerRevision {
                    kind: current_layer.kind,
                    input: current_layer.input.as_ref(),
                    node: &current_layer.node,
                },
                path,
                delta,
                scratch,
            );
            path.pop();
        }
    }
}

fn preflight_scene_layers<Message>(
    previous: &SurfaceSceneRevision<'_, Message>,
    current: &SurfaceSceneRevision<'_, Message>,
    path: &PathStack,
    delta: &mut ViewDelta,
) -> bool {
    let mut ordinal = 0_u64;
    for kind in super::LayerKind::ORDER {
        let mut previous_layers = previous.layers.iter().filter(|layer| layer.kind == kind);
        let mut current_layers = current.layers.iter().filter(|layer| layer.kind == kind);
        loop {
            match (previous_layers.next(), current_layers.next()) {
                (None, None) => break,
                (Some(_), None) | (None, Some(_)) => {
                    delta.record(
                        ViewDeltaEffect::Structural,
                        ViewDeltaCause::SceneLayerCount,
                        path.path,
                    );
                    return false;
                }
                (Some(previous), Some(current)) => {
                    let mut layer_path = *path;
                    layer_path.push(ViewDeltaPathComponent::Layer(ordinal));
                    ordinal = ordinal.saturating_add(1);
                    if previous.node.id() != current.node.id() {
                        delta.record(
                            ViewDeltaEffect::Structural,
                            ViewDeltaCause::Reordered,
                            layer_path.path,
                        );
                        return false;
                    }
                    if previous.input.is_some() != current.input.is_some() {
                        delta.record(
                            ViewDeltaEffect::Structural,
                            ViewDeltaCause::SceneLayerInput,
                            layer_path.path,
                        );
                        return false;
                    }
                }
            }
        }
    }
    true
}

fn has_duplicate_scene_node_ids<Message>(
    scene: &SurfaceSceneRevision<'_, Message>,
    scratch: &mut ViewDeltaScratch,
) -> Option<bool> {
    if !scratch.begin_scan(scene.layers.len()) {
        return None;
    }
    for kind in super::LayerKind::ORDER {
        for layer in scene.layers.iter().filter(|layer| layer.kind == kind) {
            if !scratch.insert(layer.node.id()) {
                return Some(true);
            }
        }
    }
    Some(false)
}

fn compare_layer_pair<Message>(
    previous: SurfaceLayerRevision<'_, Message>,
    current: SurfaceLayerRevision<'_, Message>,
    path: &mut PathStack,
    delta: &mut ViewDelta,
    scratch: &mut ViewDeltaScratch,
) {
    if let (Some(previous), Some(current)) = (previous.input, current.input) {
        path.push(ViewDeltaPathComponent::Input);
        compare_node(previous, current, path, delta, scratch);
        path.pop();
    }
    path.push(ViewDeltaPathComponent::Foreground);
    compare_node(previous.node, current.node, path, delta, scratch);
    path.pop();
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
            valid: true,
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
                    valid: true,
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
            valid: true,
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
    use super::{
        ViewDeltaCause, ViewDeltaEffect, ViewDeltaScratch, WidgetRevisionEffect,
        WidgetRevisionSnapshot, classify_view_delta as classify_with_scratch,
        classify_widget_revision,
    };
    use crate::{
        gui::types::{Point, Rect, Vector2},
        layout::{ContainerKind, ContainerPolicy},
        runtime::{
            EventMapper, LayerKind, SurfaceChild, SurfaceLayer, SurfaceNode, UiSurface,
            WidgetMessageMapper,
        },
        widgets::{
            ButtonMessage, ButtonWidget, ColorMarkerRunWidget, ColorMarkerWidget,
            FeedbackOverlayWidget, MarkerRunWidget, ToggleMessage, ToggleWidget, Widget,
            WidgetCapabilities, WidgetCommon, WidgetInput, WidgetOutput, WidgetRevision,
            WidgetSemantics, WidgetSemanticsRevision, WidgetSizing, WidgetStyle, WidgetTone,
        },
    };
    use std::{cell::Cell, hint::black_box, rc::Rc, time::Instant};

    fn surface(root: SurfaceNode<()>) -> UiSurface<()> {
        UiSurface::new(root)
    }

    fn classify_view_delta(previous: &UiSurface<()>, current: &UiSurface<()>) -> super::ViewDelta {
        let mut scratch = ViewDeltaScratch::with_capacity(4096);
        classify_with_scratch(previous, current, &mut scratch)
    }

    fn component_changes(previous: &dyn Widget, current: &dyn Widget) -> (bool, bool, bool, bool) {
        let previous_revision = Widget::revision(previous);
        let Some(previous) = previous_revision.exact_components() else {
            return (true, true, true, true);
        };
        let current_revision = Widget::revision(current);
        let Some(current) = current_revision.exact_components() else {
            return (true, true, true, true);
        };
        (
            !previous.structure_equal(current),
            !previous.geometry_equal(current),
            !previous.paint_equal(current),
            !previous.interaction_equal(current),
        )
    }

    #[test]
    #[ignore = "release-mode scaling benchmark"]
    fn classifier_release_scaling_250_500_1000() {
        for size in [250_usize, 500, 1000] {
            for (variant, expected) in [
                ("unchanged", ViewDeltaEffect::Unchanged),
                ("last_paint", ViewDeltaEffect::Paint),
                ("last_geometry", ViewDeltaEffect::Geometry),
                ("reorder", ViewDeltaEffect::Structural),
                ("duplicate", ViewDeltaEffect::Structural),
            ] {
                let previous = surface(benchmark_tree(size, "base"));
                let current = surface(benchmark_tree(size, variant));
                let mut scratch = ViewDeltaScratch::with_capacity(size);
                let started = Instant::now();
                let mut effect = ViewDeltaEffect::Unchanged;
                for _ in 0..32 {
                    effect =
                        black_box(classify_with_scratch(&previous, &current, &mut scratch).effect);
                }
                let average_us = started.elapsed().as_secs_f64() * 1_000_000.0 / 32.0;
                assert_eq!(effect, expected, "variant={variant} size={size}");
                println!("classifier_n={size} variant={variant} average_us={average_us:.3}");
            }
        }
    }

    fn benchmark_tree(size: usize, variant: &str) -> SurfaceNode<()> {
        let mut children = (0..size)
            .map(|id| {
                let style = if variant == "last_paint" && id + 1 == size {
                    WidgetStyle::strong(WidgetTone::Accent)
                } else {
                    WidgetStyle::normal(WidgetTone::Neutral)
                };
                let size = if variant == "last_geometry" && id + 1 == size {
                    Vector2::new(3.0, 3.0)
                } else {
                    Vector2::new(2.0, 2.0)
                };
                SurfaceChild::fill(SurfaceNode::overlay_marker(
                    (id + 2) as u64,
                    Rect::from_min_size(Point::new(0.0, 0.0), size),
                    style,
                ))
            })
            .collect::<Vec<_>>();
        if variant == "reorder" && size >= 2 {
            children.swap(size - 2, size - 1);
        } else if variant == "duplicate" && size >= 2 {
            children[size - 1] = SurfaceChild::fill(SurfaceNode::overlay_marker(
                size as u64,
                Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(2.0, 2.0)),
                WidgetStyle::normal(WidgetTone::Neutral),
            ));
        }
        SurfaceNode::container(1, ContainerPolicy::default(), children)
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
    fn passive_widgets_supply_exact_revision_evidence_and_safe_float_fallbacks() {
        let color = crate::gui::types::Rgba8::new(20, 40, 60, 255);
        let marker = ColorMarkerWidget::new(Some(color));
        assert_ne!(marker.revision(), WidgetRevision::conservative());
        assert_ne!(
            marker.revision(),
            marker
                .clone()
                .with_side(marker.props.side.saturating_add(1))
                .revision()
        );
        let mut invalid_sizing_marker = marker.clone();
        invalid_sizing_marker.common.sizing.preferred.x = f32::NAN;
        assert_eq!(
            invalid_sizing_marker.revision(),
            WidgetRevision::conservative()
        );
        for baseline in [f32::NAN, f32::INFINITY] {
            let mut invalid_baseline_marker = marker.clone();
            invalid_baseline_marker.common.sizing.baseline = Some(baseline);
            assert_eq!(
                invalid_baseline_marker.revision(),
                WidgetRevision::conservative()
            );
        }

        let run = MarkerRunWidget::new(Some(color), 3);
        assert_ne!(run.revision(), WidgetRevision::conservative());
        assert_ne!(
            run.revision(),
            run.clone()
                .with_gap(run.props.gap.saturating_add(1))
                .revision()
        );

        let color_run = ColorMarkerRunWidget::new(vec![color]);
        assert_ne!(color_run.revision(), WidgetRevision::conservative());
        assert_ne!(
            color_run.revision(),
            color_run
                .clone()
                .with_side(color_run.props.side.saturating_add(1))
                .revision()
        );

        let overlay = FeedbackOverlayWidget::fill().with_progress(0.5, color);
        assert_ne!(overlay.revision(), WidgetRevision::conservative());
        assert_eq!(
            FeedbackOverlayWidget::fill()
                .with_progress(f32::NAN, color)
                .revision(),
            WidgetRevision::conservative()
        );
        assert_eq!(
            FeedbackOverlayWidget::fill()
                .with_edge(color, f32::INFINITY, crate::gui::paint::BorderSides::ALL)
                .revision(),
            WidgetRevision::conservative()
        );
    }

    #[test]
    fn passive_widget_view_delta_effects_follow_geometry_paint_and_interaction_contracts() {
        let color = crate::gui::types::Rgba8::new(20, 40, 60, 255);
        let previous = surface(SurfaceNode::widget(
            ColorMarkerWidget::from_parts(crate::widgets::ColorMarkerWidgetParts {
                id: 1,
                sizing: WidgetSizing::fixed(Vector2::new(10.0, 10.0)),
                props: crate::widgets::ColorMarkerProps::new(Some(color)),
            }),
            WidgetMessageMapper::none(),
        ));
        let geometry = surface(SurfaceNode::widget(
            ColorMarkerWidget::from_parts(crate::widgets::ColorMarkerWidgetParts {
                id: 1,
                sizing: WidgetSizing::fixed(Vector2::new(12.0, 10.0)),
                props: crate::widgets::ColorMarkerProps::new(Some(color)),
            }),
            WidgetMessageMapper::none(),
        ));
        assert_eq!(
            classify_view_delta(&previous, &geometry).effect,
            ViewDeltaEffect::Geometry
        );

        for baseline in [f32::NAN, f32::INFINITY] {
            let invalid_baseline = surface(SurfaceNode::widget(
                ColorMarkerWidget::from_parts(crate::widgets::ColorMarkerWidgetParts {
                    id: 1,
                    sizing: WidgetSizing {
                        min: Vector2::new(10.0, 10.0),
                        preferred: Vector2::new(10.0, 10.0),
                        baseline: Some(baseline),
                    },
                    props: crate::widgets::ColorMarkerProps::new(Some(color)),
                }),
                WidgetMessageMapper::none(),
            ));
            assert_eq!(
                classify_view_delta(&previous, &invalid_baseline).effect,
                ViewDeltaEffect::Structural
            );
        }

        let mut paint_widget =
            ColorMarkerWidget::new(Some(crate::gui::types::Rgba8::new(90, 40, 60, 255)))
                .with_align(crate::widgets::ColorMarkerAlign::Left);
        paint_widget.common.id = 1;
        paint_widget.common.sizing = WidgetSizing::fixed(Vector2::new(10.0, 10.0));
        let paint = surface(SurfaceNode::widget(
            paint_widget,
            WidgetMessageMapper::none(),
        ));
        assert_eq!(
            classify_view_delta(&previous, &paint).effect,
            ViewDeltaEffect::Paint
        );

        let mut interaction_widget = ColorMarkerWidget::new(Some(color));
        interaction_widget.common.id = 1;
        interaction_widget.common.sizing = WidgetSizing::fixed(Vector2::new(10.0, 10.0));
        interaction_widget.common.tooltip = Some("hint".to_owned());
        let interaction = surface(SurfaceNode::widget(
            interaction_widget,
            WidgetMessageMapper::none(),
        ));
        assert_eq!(
            classify_view_delta(&previous, &interaction).effect,
            ViewDeltaEffect::Interaction
        );
    }

    #[test]
    fn button_and_toggle_revisioned_mappers_preserve_typed_relations() {
        let sizing = WidgetSizing::fixed(Vector2::new(80.0, 28.0));
        let button = |mapper| {
            surface(SurfaceNode::widget(
                ButtonWidget::new(1, "Run", sizing),
                WidgetMessageMapper::button_mapped(mapper),
            ))
        };
        let toggle = |mapper| {
            surface(SurfaceNode::widget(
                ToggleWidget::new(1, "Snap", sizing),
                WidgetMessageMapper::toggle_mapped(mapper),
            ))
        };

        let button_equal = button(EventMapper::with_revision(
            1_u32,
            |_message: ButtonMessage| (),
        ))
        .clone();
        let button_same = button(EventMapper::with_revision(
            1_u32,
            |_message: ButtonMessage| (),
        ));
        assert_eq!(
            classify_view_delta(&button_equal, &button_same).effect,
            ViewDeltaEffect::Unchanged
        );
        let button_changed = button(EventMapper::with_revision(
            2_u32,
            |_message: ButtonMessage| (),
        ));
        assert_eq!(
            classify_view_delta(&button_equal, &button_changed).effect,
            ViewDeltaEffect::Interaction
        );
        let button_opaque = button(EventMapper::new(|_message: ButtonMessage| ()));
        assert_eq!(
            classify_view_delta(&button_equal, &button_opaque).effect,
            ViewDeltaEffect::Structural
        );

        let toggle_equal = toggle(EventMapper::with_revision(
            1_u32,
            |_message: ToggleMessage| (),
        ))
        .clone();
        let toggle_same = toggle(EventMapper::with_revision(
            1_u32,
            |_message: ToggleMessage| (),
        ));
        assert_eq!(
            classify_view_delta(&toggle_equal, &toggle_same).effect,
            ViewDeltaEffect::Unchanged
        );
        let toggle_changed = toggle(EventMapper::with_revision(
            2_u32,
            |_message: ToggleMessage| (),
        ));
        assert_eq!(
            classify_view_delta(&toggle_equal, &toggle_changed).effect,
            ViewDeltaEffect::Interaction
        );
        let toggle_opaque = toggle(EventMapper::new(|_message: ToggleMessage| ()));
        assert_eq!(
            classify_view_delta(&toggle_equal, &toggle_opaque).effect,
            ViewDeltaEffect::Structural
        );
    }

    #[test]
    fn button_and_toggle_revisions_partition_geometry_paint_and_safe_fallbacks() {
        let sizing = WidgetSizing::fixed(Vector2::new(80.0, 28.0));
        let base_button = ButtonWidget::new(1, "Run", sizing);
        let mut geometry_button = base_button.clone();
        geometry_button.common.sizing.preferred.x = 90.0;
        let mut paint_button = base_button.clone();
        paint_button.props.text_align = crate::widgets::TextAlign::Left;
        let mut disabled_button = base_button.clone();
        disabled_button.common.state.disabled = true;
        let mut interaction_button = base_button.clone();
        interaction_button.common.tooltip = Some("hint".to_owned());
        let snapshot = |widget: &dyn Widget| WidgetRevisionSnapshot {
            id: widget.common().id,
            compatibility_kind: widget.compatibility_kind(),
            revision: Widget::revision(widget),
            valid: true,
        };
        assert_eq!(
            classify_widget_revision(
                Some(snapshot(&base_button)),
                Some(snapshot(&geometry_button))
            ),
            WidgetRevisionEffect::Geometry
        );
        assert_eq!(
            classify_widget_revision(Some(snapshot(&base_button)), Some(snapshot(&paint_button))),
            WidgetRevisionEffect::Paint
        );
        assert_eq!(
            classify_widget_revision(
                Some(snapshot(&base_button)),
                Some(snapshot(&disabled_button)),
            ),
            WidgetRevisionEffect::Paint
        );
        assert_eq!(
            classify_widget_revision(
                Some(snapshot(&base_button)),
                Some(snapshot(&interaction_button)),
            ),
            WidgetRevisionEffect::Interaction
        );
        let mut invalid_button = base_button.clone();
        invalid_button.common.sizing.preferred.x = f32::NAN;
        assert_eq!(
            Widget::revision(&invalid_button),
            WidgetRevision::conservative()
        );
        assert_eq!(
            Widget::revision(
                &base_button
                    .clone()
                    .with_trailing_icon(crate::gui::svg::SvgIcon::empty())
            ),
            WidgetRevision::conservative()
        );

        let base_toggle = ToggleWidget::new(1, "Snap", sizing);
        let mut geometry_toggle = base_toggle.clone();
        geometry_toggle.common.sizing.preferred.x = 90.0;
        let mut paint_toggle = base_toggle.clone();
        paint_toggle.state.checked = true;
        paint_toggle.common.state.active = true;
        let mut selected_toggle = base_toggle.clone();
        selected_toggle.common.state.selected = true;
        let mut disabled_toggle = base_toggle.clone();
        disabled_toggle.common.state.disabled = true;
        let mut interaction_toggle = base_toggle.clone();
        interaction_toggle.common.tooltip = Some("hint".to_owned());
        assert_eq!(
            classify_widget_revision(
                Some(snapshot(&base_toggle)),
                Some(snapshot(&geometry_toggle))
            ),
            WidgetRevisionEffect::Geometry
        );
        assert_eq!(
            classify_widget_revision(Some(snapshot(&base_toggle)), Some(snapshot(&paint_toggle))),
            WidgetRevisionEffect::Paint
        );
        assert_eq!(
            classify_widget_revision(
                Some(snapshot(&base_toggle)),
                Some(snapshot(&selected_toggle)),
            ),
            WidgetRevisionEffect::Paint
        );
        assert_eq!(
            classify_widget_revision(
                Some(snapshot(&base_toggle)),
                Some(snapshot(&disabled_toggle)),
            ),
            WidgetRevisionEffect::Paint
        );
        assert_eq!(
            classify_widget_revision(
                Some(snapshot(&base_toggle)),
                Some(snapshot(&interaction_toggle))
            ),
            WidgetRevisionEffect::Interaction
        );
        let mut invalid_toggle = base_toggle;
        invalid_toggle.common.sizing.baseline = Some(f32::INFINITY);
        assert_eq!(
            Widget::revision(&invalid_toggle),
            WidgetRevision::conservative()
        );
    }

    #[test]
    fn button_and_toggle_component_matrices_cover_paint_interaction_and_transient_inputs() {
        let sizing = WidgetSizing::fixed(Vector2::new(80.0, 28.0));
        let button = ButtonWidget::new(1, "Run", sizing);
        let mut button_automation = button.clone();
        button_automation.common.state.automation_active = true;
        assert_eq!(
            component_changes(&button, &button_automation),
            (false, false, true, false)
        );
        let mut button_active = button.clone();
        button_active.common.state.active = true;
        assert_eq!(
            component_changes(&button, &button_active),
            (false, false, true, true)
        );
        let mut button_selected = button.clone();
        button_selected.common.state.selected = true;
        assert_eq!(
            component_changes(&button, &button_selected),
            (false, false, true, false)
        );
        let mut button_disabled = button.clone();
        button_disabled.common.state.disabled = true;
        assert_eq!(
            component_changes(&button, &button_disabled),
            (false, false, true, true)
        );
        let mut button_style = button.clone();
        button_style.common.style = WidgetStyle::strong(WidgetTone::Accent);
        assert_eq!(
            component_changes(&button, &button_style),
            (false, false, true, false)
        );
        let mut button_bounds = button.clone();
        button_bounds.common.paint.bounds = crate::widgets::PaintBounds::AllowOverflow;
        assert_eq!(
            component_changes(&button, &button_bounds),
            (false, false, true, false)
        );
        let mut button_focus_paint = button.clone();
        button_focus_paint.common.paint.paints_focus = false;
        assert_eq!(
            component_changes(&button, &button_focus_paint),
            (false, false, true, false)
        );
        let mut button_state_layers = button.clone();
        button_state_layers.common.paint.paints_state_layers = false;
        assert_eq!(
            component_changes(&button, &button_state_layers),
            (false, false, true, true)
        );
        let mut button_parent_hover = button.clone();
        button_parent_hover.common.paint.suppresses_container_hover = true;
        assert_eq!(
            component_changes(&button, &button_parent_hover),
            (false, false, true, true)
        );
        let mut button_label = button.clone();
        button_label.props.label = "Stop".into();
        let mut button_trailing = button.clone();
        button_trailing.props.trailing_label = Some("⌘R".into());
        let mut button_alignment = button.clone();
        button_alignment.props.text_align = crate::widgets::TextAlign::Right;
        for changed in [
            &button_label as &dyn Widget,
            &button_trailing,
            &button_alignment,
        ] {
            assert_eq!(
                component_changes(&button, changed),
                (false, false, true, false)
            );
        }
        let mut button_focus = button.clone();
        button_focus.common.focus = crate::widgets::FocusBehavior::Pointer;
        let mut button_tooltip = button.clone();
        button_tooltip.common.tooltip = Some("hint".into());
        let mut button_policy = button.clone().with_secondary_click().with_drag();
        button_policy.common.state.read_only = true;
        assert_eq!(
            component_changes(&button, &button_focus),
            (false, false, false, true)
        );
        assert_eq!(
            component_changes(&button, &button_tooltip),
            (false, false, false, true)
        );
        assert_eq!(
            component_changes(&button, &button_policy),
            (false, false, false, true)
        );
        let mut button_transient = button.clone();
        button_transient.common.state.hovered = true;
        button_transient.common.state.pressed = true;
        button_transient.common.state.focused = true;
        button_transient.state.armed = true;
        button_transient.state.dragged = true;
        button_transient.state.press_position = Some(Point::new(4.0, 5.0));
        assert_eq!(
            component_changes(&button, &button_transient),
            (false, false, false, false)
        );

        let mut button_semantic_label = button.clone();
        button_semantic_label.props.label = "Stop".into();
        let mut button_semantic_trailing = button.clone();
        button_semantic_trailing.props.trailing_label = Some("⌘R".into());
        let mut button_semantic_selected = button.clone();
        button_semantic_selected.common.state.selected = true;
        let mut button_semantic_disabled = button.clone();
        button_semantic_disabled.common.state.disabled = true;
        let mut button_semantic_read_only = button.clone();
        button_semantic_read_only.common.state.read_only = true;
        for changed in [
            &button_semantic_label,
            &button_semantic_trailing,
            &button_semantic_selected,
            &button_semantic_disabled,
            &button_semantic_read_only,
        ] {
            assert_ne!(
                WidgetSemantics::revision(&button),
                WidgetSemantics::revision(changed)
            );
        }

        let toggle = ToggleWidget::new(2, "Snap", sizing);
        let mut toggle_automation = toggle.clone();
        toggle_automation.common.state.automation_active = true;
        assert_eq!(
            component_changes(&toggle, &toggle_automation),
            (false, false, true, false)
        );
        let toggle_checked = toggle.clone().with_checked(true);
        assert_eq!(
            component_changes(&toggle, &toggle_checked),
            (false, false, true, true)
        );
        let mut toggle_active = toggle.clone();
        toggle_active.common.state.active = true;
        assert_eq!(
            component_changes(&toggle, &toggle_active),
            (false, false, true, false)
        );
        let mut toggle_disabled = toggle.clone();
        toggle_disabled.common.state.disabled = true;
        assert_eq!(
            component_changes(&toggle, &toggle_disabled),
            (false, false, true, true)
        );
        let mut toggle_style = toggle.clone();
        toggle_style.common.style = WidgetStyle::strong(WidgetTone::Accent);
        assert_eq!(
            component_changes(&toggle, &toggle_style),
            (false, false, true, false)
        );
        let mut toggle_state_layers = toggle.clone();
        toggle_state_layers.common.paint.paints_state_layers = false;
        let mut toggle_parent_hover = toggle.clone();
        toggle_parent_hover.common.paint.suppresses_container_hover = true;
        for changed in [&toggle_state_layers as &dyn Widget, &toggle_parent_hover] {
            assert_eq!(
                component_changes(&toggle, changed),
                (false, false, true, true)
            );
        }
        let mut toggle_bounds = toggle.clone();
        toggle_bounds.common.paint.bounds = crate::widgets::PaintBounds::AllowOverflow;
        let mut toggle_focus_paint = toggle.clone();
        toggle_focus_paint.common.paint.paints_focus = false;
        assert_eq!(
            component_changes(&toggle, &toggle_bounds),
            (false, false, true, false)
        );
        assert_eq!(
            component_changes(&toggle, &toggle_focus_paint),
            (false, false, true, false)
        );
        let mut toggle_label = toggle.clone();
        toggle_label.props.label = "Latch".into();
        let mut toggle_selected = toggle.clone();
        toggle_selected.common.state.selected = true;
        for changed in [&toggle_label as &dyn Widget, &toggle_selected] {
            assert_eq!(
                component_changes(&toggle, changed),
                (false, false, true, false)
            );
        }
        let mut toggle_focus = toggle.clone();
        toggle_focus.common.focus = crate::widgets::FocusBehavior::Pointer;
        let mut toggle_tooltip = toggle.clone();
        toggle_tooltip.common.tooltip = Some("hint".into());
        let mut toggle_read_only = toggle.clone();
        toggle_read_only.common.state.read_only = true;
        assert_eq!(
            component_changes(&toggle, &toggle_focus),
            (false, false, false, true)
        );
        assert_eq!(
            component_changes(&toggle, &toggle_tooltip),
            (false, false, false, true)
        );
        assert_eq!(
            component_changes(&toggle, &toggle_read_only),
            (false, false, false, true)
        );
        let mut toggle_transient = toggle.clone();
        toggle_transient.common.state.hovered = true;
        toggle_transient.common.state.pressed = true;
        toggle_transient.common.state.focused = true;
        toggle_transient.state.armed = true;
        assert_eq!(
            component_changes(&toggle, &toggle_transient),
            (false, false, false, false)
        );

        let mut toggle_semantic_label = toggle.clone();
        toggle_semantic_label.props.label = "Latch".into();
        let toggle_semantic_checked = toggle.clone().with_checked(true);
        let mut toggle_semantic_selected = toggle.clone();
        toggle_semantic_selected.common.state.selected = true;
        let mut toggle_semantic_disabled = toggle.clone();
        toggle_semantic_disabled.common.state.disabled = true;
        let mut toggle_semantic_read_only = toggle.clone();
        toggle_semantic_read_only.common.state.read_only = true;
        for changed in [
            &toggle_semantic_label,
            &toggle_semantic_checked,
            &toggle_semantic_selected,
            &toggle_semantic_disabled,
            &toggle_semantic_read_only,
        ] {
            assert_ne!(
                WidgetSemantics::revision(&toggle),
                WidgetSemantics::revision(changed)
            );
        }
    }

    #[test]
    fn insufficient_identity_scratch_widens_to_structural_without_descent() {
        let previous = surface(SurfaceNode::container(
            1,
            ContainerPolicy::default(),
            vec![child(2), child(3), child(4)],
        ));
        let current = surface(SurfaceNode::container(
            1,
            ContainerPolicy::default(),
            vec![child(2), child(3), child(4)],
        ));
        let mut scratch = ViewDeltaScratch::with_capacity(0);
        let delta = classify_with_scratch(&previous, &current, &mut scratch);
        assert_eq!(delta.effect, ViewDeltaEffect::Structural);
        assert!(has_cause(
            &delta,
            ViewDeltaCause::InsufficientIdentityEvidence
        ));
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
        assert!(delta.event_count > 0);
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
        assert!(has_cause(&delta, ViewDeltaCause::Reordered));

        let ambiguous = surface(SurfaceNode::container(
            1,
            ContainerPolicy::default(),
            vec![child(2), child(2)],
        ));
        let delta = classify_view_delta(&previous, &ambiguous);
        assert_eq!(delta.effect, ViewDeltaEffect::Structural);
        assert!(
            has_cause(&delta, ViewDeltaCause::AmbiguousPairing)
                || has_cause(&delta, ViewDeltaCause::Added)
        );

        let previous = surface(SurfaceNode::container(
            1,
            ContainerPolicy::default(),
            vec![child(2), child(3), child(4)],
        ));
        let non_adjacent_duplicate = surface(SurfaceNode::container(
            1,
            ContainerPolicy::default(),
            vec![child(2), child(3), child(2)],
        ));
        let delta = classify_view_delta(&previous, &non_adjacent_duplicate);
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
        assert!(has_cause(&delta, ViewDeltaCause::SceneLayerCount));
        assert!(!has_cause(&delta, ViewDeltaCause::Reordered));
        assert!(!has_cause(&delta, ViewDeltaCause::SceneLayerKind));

        let unmatched_kind = surface(SurfaceNode::scene(
            10,
            base.clone(),
            vec![SurfaceLayer::new(LayerKind::Modal, child(3).child)],
        ));
        let delta = classify_view_delta(&previous, &unmatched_kind);
        assert!(has_cause(&delta, ViewDeltaCause::SceneLayerCount));
        assert!(!has_cause(&delta, ViewDeltaCause::SceneLayerKind));

        let moved_kind = surface(SurfaceNode::scene(
            10,
            base,
            vec![SurfaceLayer::new(LayerKind::Modal, child(2).child)],
        ));
        let delta = classify_view_delta(&previous, &moved_kind);
        assert_eq!(delta.effect, ViewDeltaEffect::Structural);
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
        assert!(
            has_cause(&delta, ViewDeltaCause::AmbiguousPairing)
                || has_cause(&delta, ViewDeltaCause::SceneLayerCount)
        );
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
        assert!(
            has_cause(&delta, ViewDeltaCause::AmbiguousPairing)
                || has_cause(&delta, ViewDeltaCause::SceneLayerCount)
        );
    }

    #[test]
    fn non_adjacent_scene_foreground_ids_are_ambiguous() {
        let base = SurfaceNode::overlay_marker(
            1,
            Rect::from_min_size(Point::new(0.0, 0.0), Vector2::new(2.0, 2.0)),
            WidgetStyle::normal(WidgetTone::Neutral),
        );
        let previous = surface(SurfaceNode::scene(
            10,
            base.clone(),
            vec![
                SurfaceLayer::new(LayerKind::Floating, child(2).child),
                SurfaceLayer::new(LayerKind::Modal, child(3).child),
                SurfaceLayer::new(LayerKind::Tooltip, child(4).child),
            ],
        ));
        let duplicate = surface(SurfaceNode::scene(
            10,
            base,
            vec![
                SurfaceLayer::new(LayerKind::Floating, child(2).child),
                SurfaceLayer::new(LayerKind::Modal, child(3).child),
                SurfaceLayer::new(LayerKind::Tooltip, child(2).child),
            ],
        ));
        let delta = classify_view_delta(&previous, &duplicate);
        assert_eq!(delta.effect, ViewDeltaEffect::Structural);
        assert!(has_cause(&delta, ViewDeltaCause::AmbiguousPairing));
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

    #[derive(Clone)]
    struct HookCountingWidget {
        common: WidgetCommon,
        revision_calls: Rc<Cell<u32>>,
        capability_calls: Rc<Cell<u32>>,
    }

    impl HookCountingWidget {
        fn new(id: u64, revision_calls: Rc<Cell<u32>>, capability_calls: Rc<Cell<u32>>) -> Self {
            Self {
                common: WidgetCommon::fixed(id, 40.0, 20.0),
                revision_calls,
                capability_calls,
            }
        }
    }

    impl Widget for HookCountingWidget {
        fn revision(&self) -> WidgetRevision {
            self.revision_calls
                .set(self.revision_calls.get().saturating_add(1));
            WidgetRevision::exact((), (), (), ())
        }

        fn capabilities(&self) -> WidgetCapabilities<'_> {
            self.capability_calls
                .set(self.capability_calls.get().saturating_add(1));
            WidgetCapabilities::none()
        }

        fn common(&self) -> &WidgetCommon {
            &self.common
        }

        fn common_mut(&mut self) -> &mut WidgetCommon {
            &mut self.common
        }

        fn handle_input(&mut self, _bounds: Rect, _input: WidgetInput) -> Option<WidgetOutput> {
            None
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

    #[test]
    fn cached_widget_evidence_does_not_reinvoke_revision_or_capability_hooks() {
        let revision_calls = Rc::new(Cell::new(0));
        let capability_calls = Rc::new(Cell::new(0));
        let previous = surface(SurfaceNode::static_widget(HookCountingWidget::new(
            1,
            Rc::clone(&revision_calls),
            Rc::clone(&capability_calls),
        )));
        let current = previous.clone();
        assert_eq!(revision_calls.get(), 1);
        assert_eq!(capability_calls.get(), 1);

        for _ in 0..3 {
            assert_eq!(
                classify_view_delta(&previous, &current).effect,
                ViewDeltaEffect::Unchanged
            );
        }
        assert_eq!(revision_calls.get(), 1);
        assert_eq!(capability_calls.get(), 1);
    }

    #[test]
    fn public_widget_mutation_invalidates_cached_evidence_conservatively() {
        let previous = surface(SurfaceNode::static_widget(RevisionWidget::new(1)));
        let mut current = previous.clone();
        let Some(widget) = current.find_widget_mut(1) else {
            return;
        };
        widget.widget_mut().common_mut().state.hovered = true;

        let delta = classify_view_delta(&previous, &current);
        assert_eq!(delta.effect, ViewDeltaEffect::Structural);
        assert!(has_cause(&delta, ViewDeltaCause::WidgetRevision));
    }

    #[test]
    fn runtime_state_mutation_preserves_cached_declarative_evidence() {
        let previous = surface(SurfaceNode::static_widget(RevisionWidget::new(1)));
        let mut current = previous.clone();
        let Some(widget) = current.find_widget_mut(1) else {
            return;
        };
        widget
            .widget_object_mut_runtime()
            .common_mut()
            .state
            .hovered = true;

        assert_eq!(
            classify_view_delta(&previous, &current).effect,
            ViewDeltaEffect::Unchanged
        );
    }
}
