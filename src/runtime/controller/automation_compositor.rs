//! Private, atomic composition of virtual semantic evidence into automation.
//!
//! This module consumes only the exact classification output of the preceding
//! virtual-layout boundary.  It does not query providers, inspect runtime
//! lifecycle state, or publish into a live surface.  The input snapshot is
//! cloned only after all structural and identity preflight has succeeded.

use super::virtual_layout::{
    VirtualLayoutSemanticClassificationBatch, VirtualLayoutSemanticClassificationInput,
    VirtualLayoutSemanticClassificationOrigin,
};
use crate::{
    gui::{
        automation::{
            AutomationBounds, AutomationNodeId, AutomationNodeSnapshot, AutomationTargetAuthority,
            GuiAutomationSnapshot, GuiAutomationTargetSnapshot,
        },
        layout_core::{
            VirtualLayoutCoordinateSpace, VirtualLayoutSemanticProjection,
            VirtualLayoutSemanticRangeRequest, VirtualLayoutSemanticRequest,
        },
    },
    layout::{NodeId, VIRTUAL_LAYOUT_MAX_QUERY_ENTRIES},
};
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

/// A private composed automation snapshot plus the authority sidecar required
/// to keep unmaterialized semantic roots unavailable to action dispatch.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct VirtualLayoutAutomationComposition {
    snapshot: GuiAutomationSnapshot,
    unmaterialized_ids: BTreeSet<AutomationNodeId>,
}

#[allow(dead_code)]
impl VirtualLayoutAutomationComposition {
    fn new(
        snapshot: GuiAutomationSnapshot,
        unmaterialized_ids: BTreeSet<AutomationNodeId>,
    ) -> Self {
        Self {
            snapshot,
            unmaterialized_ids,
        }
    }

    /// Return the staged public-schema snapshot without exposing the private
    /// authority sidecar.
    pub(crate) fn snapshot(&self) -> &GuiAutomationSnapshot {
        &self.snapshot
    }

    /// Consume the private composition and return its unchanged public-schema
    /// snapshot.
    pub(crate) fn into_snapshot(self) -> GuiAutomationSnapshot {
        self.snapshot
    }

    /// Flatten the staged snapshot while attaching runtime authority.  The
    /// serialized [`AutomationNodeSnapshot`] schema remains unchanged.
    pub(crate) fn target_snapshot(&self, runtime_generation: u64) -> GuiAutomationTargetSnapshot {
        let mut snapshot = self.snapshot.target_snapshot();
        for target in &mut snapshot.targets {
            target.authority = Some(AutomationTargetAuthority {
                runtime_generation,
                materialized: !self.unmaterialized_ids.contains(&target.id),
            });
        }
        snapshot.schema_version = 2;
        snapshot
    }

    /// Return the private IDs that carry unmaterialized authority.
    pub(crate) fn unmaterialized_ids(&self) -> &BTreeSet<AutomationNodeId> {
        &self.unmaterialized_ids
    }
}

/// Typed failures for one all-or-nothing automation composition attempt.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum VirtualLayoutAutomationCompositionError {
    /// A classification batch is structurally inconsistent with its range.
    MalformedClassification,
    /// Opaque equality was not stable during preflight.
    UnstableEquality,
    /// The same key was supplied at two logical indices.
    KeyIndexDrift,
    /// Different virtual entries claimed one logical index.
    DuplicateLogicalIndex,
    /// Two overlapping entries could not be compared unambiguously.
    AmbiguousOverlap,
    /// Same-key/index evidence disagreed in one or more fields.
    ConflictingOverlap,
    /// A custom coordinate space has no compositor transform contract.
    CoordinateTransformUnavailable,
    /// The union for one live container exceeds its admitted budget.
    AggregateBudgetExceeded,
    /// The complete composed union exceeds the hard query cap.
    HardQueryCapExceeded,
    /// No ordinary node has the requested virtual-container ID.
    MissingContainerAnchor,
    /// More than one ordinary node has the requested virtual-container ID.
    DuplicateContainerAnchor,
    /// Virtual-container anchors are nested in the ordinary tree.
    NestedContainerAnchor,
    /// An anchor occurs below a generated virtual wrapper.
    WrongParentAnchor,
    /// A materialized generated wrapper root is absent.
    MissingPayloadRoot,
    /// A materialized generated wrapper root occurs more than once.
    DuplicatePayloadRoot,
    /// A generated wrapper exists, but not as the exact direct child required.
    WrongParentPayloadRoot,
    /// The final automation-node namespace contains a collision.
    AutomationNodeIdCollision,
    /// The classification no longer matches one live virtual-layout record.
    LiveClassificationMismatch,
    /// No unique live record exists for one classification container.
    LiveRecordUnavailable,
}

#[derive(Clone)]
struct NormalizedEntry {
    container_id: NodeId,
    projection: VirtualLayoutSemanticProjection,
    origin: VirtualLayoutSemanticClassificationOrigin,
    request: NormalizedRequest,
}

#[derive(Clone)]
enum NormalizedRequest {
    Range(VirtualLayoutSemanticRangeRequest),
    Pin(VirtualLayoutSemanticRequest),
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct NodePath(Vec<usize>);

#[derive(Clone)]
struct AnchorPlan {
    path: NodePath,
}

/// Compose already-classified virtual semantic evidence into one staged
/// automation snapshot.  The function is pure with respect to the caller's
/// snapshot and all runtime state.
pub(super) fn compose_virtual_layout_automation_snapshot(
    ordinary: &GuiAutomationSnapshot,
    batches: &[VirtualLayoutSemanticClassificationBatch],
) -> Result<VirtualLayoutAutomationComposition, VirtualLayoutAutomationCompositionError> {
    let inputs = batches
        .iter()
        .cloned()
        .map(VirtualLayoutSemanticClassificationInput::Range)
        .collect::<Vec<_>>();
    compose_virtual_layout_automation_snapshot_inputs(ordinary, &inputs)
}

/// Compose normalized range and first-class pin classifications together.
/// This is the compositor half of the owner-mediated publication kernel; it
/// remains a pure staged operation and never invokes a provider.
pub(super) fn compose_virtual_layout_automation_snapshot_inputs(
    ordinary: &GuiAutomationSnapshot,
    inputs: &[VirtualLayoutSemanticClassificationInput],
) -> Result<VirtualLayoutAutomationComposition, VirtualLayoutAutomationCompositionError> {
    if inputs.is_empty() {
        return Ok(VirtualLayoutAutomationComposition::new(
            ordinary.clone(),
            BTreeSet::new(),
        ));
    }

    let entries = normalize_inputs(inputs)?;
    let entries_by_container = group_entries(entries)?;
    let ordinary_locations = collect_ordinary_locations(&ordinary.root);
    let ordinary_ids: HashSet<_> = ordinary_locations.keys().cloned().collect();
    let anchors = resolve_anchors(&ordinary_locations, &entries_by_container)?;
    let generated_wrapper_locations =
        collect_generated_wrapper_locations(&ordinary_locations, &entries_by_container);
    reject_anchor_below_generated_wrapper(&anchors, &generated_wrapper_locations)?;
    reject_nested_anchors(&anchors)?;
    let wrapper_locations = validate_payload_roots(
        &ordinary.root,
        &ordinary_locations,
        &anchors,
        &entries_by_container,
    )?;

    reject_provider_id_collisions(&ordinary_ids, &entries_by_container, &wrapper_locations)?;

    let mut staged = ordinary.clone();
    let mut unmaterialized_ids = BTreeSet::new();
    for (container_id, entries) in &entries_by_container {
        let anchor_id = AutomationNodeId::new(container_id.to_string());
        if !apply_container_segment(
            &mut staged.root,
            &anchor_id,
            entries,
            &mut unmaterialized_ids,
        ) {
            // All anchors were resolved above.  A false result means the
            // staged tree no longer matches the preflight tree, so fail closed
            // instead of publishing a partial result.
            return Err(VirtualLayoutAutomationCompositionError::MissingContainerAnchor);
        }
    }

    let mut final_ids = HashSet::new();
    if !audit_unique_ids(&staged.root, &mut final_ids) {
        return Err(VirtualLayoutAutomationCompositionError::AutomationNodeIdCollision);
    }
    if entries_by_container
        .values()
        .flatten()
        .any(|entry| !final_ids.contains(entry.projection.automation_node_id()))
    {
        return Err(VirtualLayoutAutomationCompositionError::MalformedClassification);
    }

    Ok(VirtualLayoutAutomationComposition::new(
        staged,
        unmaterialized_ids,
    ))
}

pub(super) fn ordinary_virtual_layout_automation_snapshot(
    ordinary: &GuiAutomationSnapshot,
) -> VirtualLayoutAutomationComposition {
    VirtualLayoutAutomationComposition::new(ordinary.clone(), BTreeSet::new())
}

fn normalize_inputs(
    inputs: &[VirtualLayoutSemanticClassificationInput],
) -> Result<Vec<NormalizedEntry>, VirtualLayoutAutomationCompositionError> {
    let mut entries = Vec::new();
    let mut container_fences = BTreeMap::<NodeId, VirtualLayoutSemanticRangeRequest>::new();
    let mut pin_scopes =
        BTreeMap::<NodeId, (VirtualLayoutSemanticRequest, VirtualLayoutCoordinateSpace)>::new();
    let raw_input_cap = VIRTUAL_LAYOUT_MAX_QUERY_ENTRIES
        .checked_add(64)
        .ok_or(VirtualLayoutAutomationCompositionError::HardQueryCapExceeded)?;

    for input in inputs {
        match input {
            VirtualLayoutSemanticClassificationInput::Range(batch) => {
                let request = batch.request();
                let range = request.range();
                if batch.classifications().len() != range.length()
                    || range.length() == 0
                    || range.length() > request.budget().max_entries()
                    || range.length() > VIRTUAL_LAYOUT_MAX_QUERY_ENTRIES
                {
                    return Err(VirtualLayoutAutomationCompositionError::MalformedClassification);
                }
                if !matches!(
                    request.coordinate_space(),
                    VirtualLayoutCoordinateSpace::Logical
                ) {
                    return Err(
                        VirtualLayoutAutomationCompositionError::CoordinateTransformUnavailable,
                    );
                }
                if let Some(previous_request) = container_fences.get(&request.container_id())
                    && !same_range_scope(previous_request, request)?
                {
                    return Err(VirtualLayoutAutomationCompositionError::MalformedClassification);
                }
                if let Some((previous_request, previous_coordinate_space)) =
                    pin_scopes.get(&request.container_id())
                    && (!same_item_scope(request, previous_request)?
                        || !same_coordinate_space(
                            previous_coordinate_space,
                            request.coordinate_space(),
                        )?)
                {
                    return Err(VirtualLayoutAutomationCompositionError::MalformedClassification);
                }
                container_fences
                    .entry(request.container_id())
                    .or_insert_with(|| request.clone());
                if entries
                    .len()
                    .checked_add(batch.classifications().len())
                    .is_none_or(|length| length > raw_input_cap)
                {
                    return Err(VirtualLayoutAutomationCompositionError::HardQueryCapExceeded);
                }

                for (offset, classification) in batch.classifications().iter().enumerate() {
                    let projection = classification.projection();
                    validate_classification(batch, projection, offset)?;
                    entries.push(NormalizedEntry {
                        container_id: request.container_id(),
                        projection: projection.clone(),
                        origin: classification.origin(),
                        request: NormalizedRequest::Range(request.clone()),
                    });
                }
            }
            VirtualLayoutSemanticClassificationInput::Pin(pin) => {
                let request = pin.request();
                let classification = pin.classification();
                let projection = classification.projection();
                validate_pin_classification(request, projection)?;
                if !matches!(
                    projection.coordinate_space(),
                    VirtualLayoutCoordinateSpace::Logical
                ) {
                    return Err(
                        VirtualLayoutAutomationCompositionError::CoordinateTransformUnavailable,
                    );
                }
                if let Some(range_request) = container_fences.get(&request.container_id())
                    && (!same_item_scope(range_request, request)?
                        || !same_coordinate_space(
                            range_request.coordinate_space(),
                            projection.coordinate_space(),
                        )?)
                {
                    return Err(VirtualLayoutAutomationCompositionError::MalformedClassification);
                }
                if let Some((previous_request, previous_coordinate_space)) =
                    pin_scopes.get(&request.container_id())
                {
                    if !same_pin_scope(
                        previous_request,
                        previous_coordinate_space,
                        request,
                        projection.coordinate_space(),
                    )? {
                        return Err(
                            VirtualLayoutAutomationCompositionError::MalformedClassification,
                        );
                    }
                    match previous_request.key().stable_equals(request.key()) {
                        Some(true) => {}
                        Some(false) => {
                            return Err(
                                VirtualLayoutAutomationCompositionError::MalformedClassification,
                            );
                        }
                        None => {
                            return Err(VirtualLayoutAutomationCompositionError::UnstableEquality);
                        }
                    }
                } else {
                    pin_scopes.insert(
                        request.container_id(),
                        (request.clone(), projection.coordinate_space().clone()),
                    );
                }
                if entries
                    .len()
                    .checked_add(1)
                    .is_none_or(|length| length > raw_input_cap)
                {
                    return Err(VirtualLayoutAutomationCompositionError::HardQueryCapExceeded);
                }
                entries.push(NormalizedEntry {
                    container_id: request.container_id(),
                    projection: projection.clone(),
                    origin: classification.origin(),
                    request: NormalizedRequest::Pin(request.clone()),
                });
            }
        }
    }

    entries.sort_by_key(|entry| (entry.container_id, entry.projection.logical_index()));

    let mut unique = Vec::with_capacity(entries.len());
    for entry in entries {
        if let Some(previous) = unique.last().filter(|previous: &&NormalizedEntry| {
            previous.container_id == entry.container_id
                && previous.projection.logical_index() == entry.projection.logical_index()
        }) {
            match previous
                .projection
                .identity()
                .key()
                .stable_equals(entry.projection.identity().key())
            {
                Some(true) => {
                    if same_entry_evidence(previous, &entry)? {
                        continue;
                    }
                    return Err(VirtualLayoutAutomationCompositionError::ConflictingOverlap);
                }
                Some(false) => {
                    return Err(VirtualLayoutAutomationCompositionError::DuplicateLogicalIndex);
                }
                None => return Err(VirtualLayoutAutomationCompositionError::AmbiguousOverlap),
            }
        }

        for previous in unique.iter() {
            if previous.container_id != entry.container_id {
                continue;
            }
            match previous
                .projection
                .identity()
                .key()
                .stable_equals(entry.projection.identity().key())
            {
                Some(true) => {
                    if previous.projection.logical_index() != entry.projection.logical_index() {
                        return Err(VirtualLayoutAutomationCompositionError::KeyIndexDrift);
                    }
                }
                Some(false) => {}
                None => return Err(VirtualLayoutAutomationCompositionError::UnstableEquality),
            }
        }

        if unique.iter().any(|previous| {
            previous.origin_payload_root() == entry.origin_payload_root()
                && entry.origin_payload_root().is_some()
        }) {
            return Err(VirtualLayoutAutomationCompositionError::DuplicatePayloadRoot);
        }
        unique.push(entry);
    }

    let mut per_container_range_counts = BTreeMap::<NodeId, usize>::new();
    for entry in &unique {
        if matches!(&entry.request, NormalizedRequest::Range(_)) {
            let count = per_container_range_counts
                .entry(entry.container_id)
                .or_default();
            *count = count
                .checked_add(1)
                .ok_or(VirtualLayoutAutomationCompositionError::HardQueryCapExceeded)?;
        }
    }
    for (container_id, count) in per_container_range_counts {
        if let Some(request) = container_fences.get(&container_id)
            && count > request.budget().max_entries()
        {
            return Err(VirtualLayoutAutomationCompositionError::AggregateBudgetExceeded);
        }
    }
    if unique.len() > VIRTUAL_LAYOUT_MAX_QUERY_ENTRIES {
        return Err(VirtualLayoutAutomationCompositionError::HardQueryCapExceeded);
    }

    Ok(unique)
}

fn validate_classification(
    batch: &VirtualLayoutSemanticClassificationBatch,
    projection: &VirtualLayoutSemanticProjection,
    offset: usize,
) -> Result<(), VirtualLayoutAutomationCompositionError> {
    let request = batch.request();
    let range = request.range();
    if projection.authority()
        != crate::gui::layout_core::VirtualLayoutSemanticProjectionAuthority::Unmaterialized
        || projection.identity().container_id() != request.container_id()
        || range.expected_index(offset) != Some(projection.logical_index())
        || !projection.bounds().is_finite()
        || projection.bounds().min.x > projection.bounds().max.x
        || projection.bounds().min.y > projection.bounds().max.y
    {
        return Err(VirtualLayoutAutomationCompositionError::MalformedClassification);
    }
    let Some(projection_range_request) = projection.range_request() else {
        return Err(VirtualLayoutAutomationCompositionError::MalformedClassification);
    };
    if !same_range_request(request, projection_range_request)?
        || !same_item_request(request, projection.request())?
        || !same_coordinate_space(request.coordinate_space(), projection.coordinate_space())?
    {
        return Err(VirtualLayoutAutomationCompositionError::MalformedClassification);
    }
    let identity_key = projection.identity().key();
    let request_key = projection.request().key();
    if identity_key.stable_equals(identity_key) != Some(true)
        || request_key.stable_equals(request_key) != Some(true)
    {
        return Err(VirtualLayoutAutomationCompositionError::UnstableEquality);
    }
    if identity_key.stable_equals(request_key) != Some(true) {
        return Err(VirtualLayoutAutomationCompositionError::MalformedClassification);
    }
    Ok(())
}

fn validate_pin_classification(
    request: &VirtualLayoutSemanticRequest,
    projection: &VirtualLayoutSemanticProjection,
) -> Result<(), VirtualLayoutAutomationCompositionError> {
    if projection.authority()
        != crate::gui::layout_core::VirtualLayoutSemanticProjectionAuthority::Unmaterialized
        || projection.range_request().is_some()
        || projection.identity().container_id() != request.container_id()
        || !projection.bounds().is_finite()
        || projection.bounds().min.x > projection.bounds().max.x
        || projection.bounds().min.y > projection.bounds().max.y
    {
        return Err(VirtualLayoutAutomationCompositionError::MalformedClassification);
    }
    if !same_pin_request(request, projection.request())? {
        return Err(VirtualLayoutAutomationCompositionError::MalformedClassification);
    }
    let identity_key = projection.identity().key();
    let request_key = request.key();
    if identity_key.stable_equals(identity_key) != Some(true)
        || request_key.stable_equals(request_key) != Some(true)
    {
        return Err(VirtualLayoutAutomationCompositionError::UnstableEquality);
    }
    if identity_key.stable_equals(request_key) != Some(true)
        || projection.request().key().stable_equals(request_key) != Some(true)
    {
        return Err(VirtualLayoutAutomationCompositionError::MalformedClassification);
    }
    Ok(())
}

fn same_range_request(
    left: &VirtualLayoutSemanticRangeRequest,
    right: &VirtualLayoutSemanticRangeRequest,
) -> Result<bool, VirtualLayoutAutomationCompositionError> {
    if left.container_id() != right.container_id()
        || left.mount_generation() != right.mount_generation()
        || left.data_revision() != right.data_revision()
        || left.policy_revision() != right.policy_revision()
        || left.measurement_revision() != right.measurement_revision()
        || left.semantic_revision() != right.semantic_revision()
        || left.budget() != right.budget()
        || left.range() != right.range()
    {
        return Ok(false);
    }
    if left
        .policy_identity()
        .stable_equals(right.policy_identity())
        != Some(true)
    {
        return Err(VirtualLayoutAutomationCompositionError::UnstableEquality);
    }
    same_coordinate_space(left.coordinate_space(), right.coordinate_space())
}

fn same_item_request(
    range_request: &VirtualLayoutSemanticRangeRequest,
    item_request: &VirtualLayoutSemanticRequest,
) -> Result<bool, VirtualLayoutAutomationCompositionError> {
    same_item_scope(range_request, item_request)
}

fn same_item_scope(
    range_request: &VirtualLayoutSemanticRangeRequest,
    item_request: &VirtualLayoutSemanticRequest,
) -> Result<bool, VirtualLayoutAutomationCompositionError> {
    if range_request.container_id() != item_request.container_id()
        || range_request.mount_generation() != item_request.mount_generation()
        || range_request.data_revision() != item_request.data_revision()
        || range_request.policy_revision() != item_request.policy_revision()
        || range_request.measurement_revision() != item_request.measurement_revision()
        || range_request.semantic_revision() != item_request.semantic_revision()
    {
        return Ok(false);
    }
    if range_request
        .policy_identity()
        .stable_equals(item_request.policy_identity())
        != Some(true)
    {
        return Err(VirtualLayoutAutomationCompositionError::UnstableEquality);
    }
    Ok(true)
}

fn same_pin_scope(
    left: &VirtualLayoutSemanticRequest,
    left_coordinate_space: &VirtualLayoutCoordinateSpace,
    right: &VirtualLayoutSemanticRequest,
    right_coordinate_space: &VirtualLayoutCoordinateSpace,
) -> Result<bool, VirtualLayoutAutomationCompositionError> {
    if left.container_id() != right.container_id()
        || left.mount_generation() != right.mount_generation()
        || left.data_revision() != right.data_revision()
        || left.policy_revision() != right.policy_revision()
        || left.measurement_revision() != right.measurement_revision()
        || left.semantic_revision() != right.semantic_revision()
    {
        return Ok(false);
    }
    if left
        .policy_identity()
        .stable_equals(right.policy_identity())
        != Some(true)
    {
        return Err(VirtualLayoutAutomationCompositionError::UnstableEquality);
    }
    same_coordinate_space(left_coordinate_space, right_coordinate_space)
}

fn same_pin_request(
    left: &VirtualLayoutSemanticRequest,
    right: &VirtualLayoutSemanticRequest,
) -> Result<bool, VirtualLayoutAutomationCompositionError> {
    if !same_pin_scope(
        left,
        &VirtualLayoutCoordinateSpace::Logical,
        right,
        &VirtualLayoutCoordinateSpace::Logical,
    )? {
        return Ok(false);
    }
    match left.key().stable_equals(right.key()) {
        Some(value) => Ok(value),
        None => Err(VirtualLayoutAutomationCompositionError::UnstableEquality),
    }
}

fn same_coordinate_space(
    left: &VirtualLayoutCoordinateSpace,
    right: &VirtualLayoutCoordinateSpace,
) -> Result<bool, VirtualLayoutAutomationCompositionError> {
    match (left, right) {
        (VirtualLayoutCoordinateSpace::Logical, VirtualLayoutCoordinateSpace::Logical) => Ok(true),
        (
            VirtualLayoutCoordinateSpace::Custom(left),
            VirtualLayoutCoordinateSpace::Custom(right),
        ) => left
            .stable_equals(right)
            .ok_or(VirtualLayoutAutomationCompositionError::UnstableEquality),
        _ => Ok(false),
    }
}

fn same_entry_evidence(
    left: &NormalizedEntry,
    right: &NormalizedEntry,
) -> Result<bool, VirtualLayoutAutomationCompositionError> {
    if left.container_id != right.container_id
        || left.projection.logical_index() != right.projection.logical_index()
        || left.origin != right.origin
        || left.projection.automation_node_id() != right.projection.automation_node_id()
        || !same_rect(left.projection.bounds(), right.projection.bounds())
        || left.projection.semantics() != right.projection.semantics()
    {
        return Ok(false);
    }
    if left
        .projection
        .identity()
        .key()
        .stable_equals(right.projection.identity().key())
        != Some(true)
    {
        return Err(VirtualLayoutAutomationCompositionError::UnstableEquality);
    }
    match (&left.request, &right.request) {
        (NormalizedRequest::Range(left_request), NormalizedRequest::Range(right_request)) => {
            if !same_range_scope(left_request, right_request)?
                || !same_item_request(left_request, left.projection.request())?
                || !same_item_request(right_request, right.projection.request())?
            {
                return Ok(false);
            }
        }
        (NormalizedRequest::Pin(left_request), NormalizedRequest::Pin(right_request)) => {
            if !same_pin_request(left_request, right_request)?
                || !same_pin_scope(
                    left_request,
                    left.projection.coordinate_space(),
                    right_request,
                    right.projection.coordinate_space(),
                )?
            {
                return Ok(false);
            }
        }
        (NormalizedRequest::Range(range_request), NormalizedRequest::Pin(pin_request))
        | (NormalizedRequest::Pin(pin_request), NormalizedRequest::Range(range_request)) => {
            if !same_item_scope(range_request, pin_request)?
                || !same_coordinate_space(
                    range_request.coordinate_space(),
                    left.projection.coordinate_space(),
                )?
                || !same_coordinate_space(
                    range_request.coordinate_space(),
                    right.projection.coordinate_space(),
                )?
            {
                return Ok(false);
            }
        }
    }
    Ok(true)
}

fn same_range_scope(
    left: &VirtualLayoutSemanticRangeRequest,
    right: &VirtualLayoutSemanticRangeRequest,
) -> Result<bool, VirtualLayoutAutomationCompositionError> {
    if left.container_id() != right.container_id()
        || left.mount_generation() != right.mount_generation()
        || left.data_revision() != right.data_revision()
        || left.policy_revision() != right.policy_revision()
        || left.measurement_revision() != right.measurement_revision()
        || left.semantic_revision() != right.semantic_revision()
        || left.budget() != right.budget()
    {
        return Ok(false);
    }
    if left
        .policy_identity()
        .stable_equals(right.policy_identity())
        != Some(true)
    {
        return Err(VirtualLayoutAutomationCompositionError::UnstableEquality);
    }
    same_coordinate_space(left.coordinate_space(), right.coordinate_space())
}

fn same_rect(left: crate::gui::types::Rect, right: crate::gui::types::Rect) -> bool {
    left.min.x.to_bits() == right.min.x.to_bits()
        && left.min.y.to_bits() == right.min.y.to_bits()
        && left.max.x.to_bits() == right.max.x.to_bits()
        && left.max.y.to_bits() == right.max.y.to_bits()
}

fn group_entries(
    entries: Vec<NormalizedEntry>,
) -> Result<BTreeMap<NodeId, Vec<NormalizedEntry>>, VirtualLayoutAutomationCompositionError> {
    let mut grouped = BTreeMap::<NodeId, Vec<NormalizedEntry>>::new();
    for entry in entries {
        grouped.entry(entry.container_id).or_default().push(entry);
    }
    for entries in grouped.values_mut() {
        entries.sort_by_key(|entry| entry.projection.logical_index());
    }
    Ok(grouped)
}

fn collect_ordinary_locations(
    root: &AutomationNodeSnapshot,
) -> HashMap<AutomationNodeId, Vec<NodePath>> {
    let mut locations = HashMap::new();
    let mut path = Vec::new();
    collect_ordinary_locations_into(root, &mut path, &mut locations);
    locations
}

fn collect_ordinary_locations_into(
    node: &AutomationNodeSnapshot,
    path: &mut Vec<usize>,
    locations: &mut HashMap<AutomationNodeId, Vec<NodePath>>,
) {
    locations
        .entry(node.id.clone())
        .or_default()
        .push(NodePath(path.clone()));
    for (index, child) in node.children.iter().enumerate() {
        path.push(index);
        collect_ordinary_locations_into(child, path, locations);
        path.pop();
    }
}

fn resolve_anchors(
    ordinary_locations: &HashMap<AutomationNodeId, Vec<NodePath>>,
    entries_by_container: &BTreeMap<NodeId, Vec<NormalizedEntry>>,
) -> Result<BTreeMap<NodeId, AnchorPlan>, VirtualLayoutAutomationCompositionError> {
    let mut anchors = BTreeMap::new();
    for container_id in entries_by_container.keys().copied() {
        let id = AutomationNodeId::new(container_id.to_string());
        let paths = ordinary_locations
            .get(&id)
            .ok_or(VirtualLayoutAutomationCompositionError::MissingContainerAnchor)?;
        if paths.len() != 1 {
            return Err(VirtualLayoutAutomationCompositionError::DuplicateContainerAnchor);
        }
        anchors.insert(
            container_id,
            AnchorPlan {
                path: paths[0].clone(),
            },
        );
    }

    Ok(anchors)
}

fn reject_nested_anchors(
    anchors: &BTreeMap<NodeId, AnchorPlan>,
) -> Result<(), VirtualLayoutAutomationCompositionError> {
    let anchor_values: Vec<_> = anchors.values().collect();
    for (left_index, left) in anchor_values.iter().enumerate() {
        for right in anchor_values.iter().skip(left_index + 1) {
            if is_strict_prefix(&left.path, &right.path)
                || is_strict_prefix(&right.path, &left.path)
            {
                return Err(VirtualLayoutAutomationCompositionError::NestedContainerAnchor);
            }
        }
    }
    Ok(())
}

fn collect_generated_wrapper_locations(
    ordinary_locations: &HashMap<AutomationNodeId, Vec<NodePath>>,
    entries_by_container: &BTreeMap<NodeId, Vec<NormalizedEntry>>,
) -> HashMap<AutomationNodeId, NodePath> {
    let mut locations = HashMap::new();
    for entry in entries_by_container.values().flatten() {
        let Some(payload_root) = entry.origin_payload_root() else {
            continue;
        };
        let wrapper_id = AutomationNodeId::new(payload_root.to_string());
        if let Some(paths) = ordinary_locations.get(&wrapper_id)
            && paths.len() == 1
        {
            locations.insert(wrapper_id, paths[0].clone());
        }
    }
    locations
}

fn validate_payload_roots(
    root: &AutomationNodeSnapshot,
    ordinary_locations: &HashMap<AutomationNodeId, Vec<NodePath>>,
    anchors: &BTreeMap<NodeId, AnchorPlan>,
    entries_by_container: &BTreeMap<NodeId, Vec<NormalizedEntry>>,
) -> Result<HashMap<AutomationNodeId, NodePath>, VirtualLayoutAutomationCompositionError> {
    let mut locations = HashMap::new();
    for (container_id, entries) in entries_by_container {
        let anchor = anchors
            .get(container_id)
            .ok_or(VirtualLayoutAutomationCompositionError::MissingContainerAnchor)?;
        let anchor_node = node_at_path(root, &anchor.path)
            .ok_or(VirtualLayoutAutomationCompositionError::MissingContainerAnchor)?;
        for entry in entries {
            let Some(payload_root) = entry.origin_payload_root() else {
                continue;
            };
            let wrapper_id = AutomationNodeId::new(payload_root.to_string());
            let direct_count = anchor_node
                .children
                .iter()
                .filter(|child| child.id == wrapper_id)
                .count();
            let all_locations = ordinary_locations.get(&wrapper_id);
            match (direct_count, all_locations) {
                (0, None) => {
                    return Err(VirtualLayoutAutomationCompositionError::MissingPayloadRoot);
                }
                (0, Some(_)) => {
                    return Err(VirtualLayoutAutomationCompositionError::WrongParentPayloadRoot);
                }
                (1, Some(paths)) if paths.len() == 1 => {
                    locations.insert(wrapper_id, paths[0].clone());
                }
                (1, Some(_)) | (_, Some(_)) => {
                    return Err(VirtualLayoutAutomationCompositionError::DuplicatePayloadRoot);
                }
                (_, None) => {
                    return Err(VirtualLayoutAutomationCompositionError::MissingPayloadRoot);
                }
            }
        }
    }
    Ok(locations)
}

fn reject_anchor_below_generated_wrapper(
    anchors: &BTreeMap<NodeId, AnchorPlan>,
    wrapper_locations: &HashMap<AutomationNodeId, NodePath>,
) -> Result<(), VirtualLayoutAutomationCompositionError> {
    for anchor in anchors.values() {
        for wrapper_path in wrapper_locations.values() {
            if anchor.path == *wrapper_path || is_strict_prefix(wrapper_path, &anchor.path) {
                return Err(VirtualLayoutAutomationCompositionError::WrongParentAnchor);
            }
        }
    }
    Ok(())
}

fn reject_provider_id_collisions(
    ordinary_ids: &HashSet<AutomationNodeId>,
    entries_by_container: &BTreeMap<NodeId, Vec<NormalizedEntry>>,
    wrapper_locations: &HashMap<AutomationNodeId, NodePath>,
) -> Result<(), VirtualLayoutAutomationCompositionError> {
    let mut provider_ids = HashSet::new();
    let wrapper_ids: HashSet<_> = wrapper_locations.keys().cloned().collect();
    for entry in entries_by_container.values().flatten() {
        let provider_id = entry.projection.automation_node_id();
        let own_wrapper_id = entry
            .origin_payload_root()
            .map(|payload_root| AutomationNodeId::new(payload_root.to_string()));
        if ordinary_ids.contains(provider_id) && own_wrapper_id.as_ref() != Some(provider_id) {
            return Err(VirtualLayoutAutomationCompositionError::AutomationNodeIdCollision);
        }
        if own_wrapper_id.as_ref().is_none() && ordinary_ids.contains(provider_id) {
            return Err(VirtualLayoutAutomationCompositionError::AutomationNodeIdCollision);
        }
        if own_wrapper_id.as_ref().is_some_and(|wrapper_id| {
            wrapper_ids.contains(provider_id) && wrapper_id != provider_id
        }) {
            return Err(VirtualLayoutAutomationCompositionError::AutomationNodeIdCollision);
        }
        if !provider_ids.insert(provider_id.clone()) {
            return Err(VirtualLayoutAutomationCompositionError::AutomationNodeIdCollision);
        }
    }
    Ok(())
}

fn node_at_path<'a>(
    root: &'a AutomationNodeSnapshot,
    path: &NodePath,
) -> Option<&'a AutomationNodeSnapshot> {
    let mut node = root;
    for index in &path.0 {
        node = node.children.get(*index)?;
    }
    Some(node)
}

fn apply_container_segment(
    node: &mut AutomationNodeSnapshot,
    anchor_id: &AutomationNodeId,
    entries: &[NormalizedEntry],
    unmaterialized_ids: &mut BTreeSet<AutomationNodeId>,
) -> bool {
    if &node.id == anchor_id {
        let materialized_wrapper_ids: HashSet<_> = entries
            .iter()
            .filter_map(|entry| entry.origin_payload_root())
            .map(|payload_root| AutomationNodeId::new(payload_root.to_string()))
            .collect();
        let mut ordinary_children = Vec::with_capacity(node.children.len() + entries.len());
        let mut replaced_wrapper_ids = HashSet::new();
        for child in node.children.drain(..) {
            if let Some(entry) = entries.iter().find(|entry| {
                entry.origin_payload_root().is_some_and(|payload_root| {
                    AutomationNodeId::new(payload_root.to_string()) == child.id
                })
            }) {
                let wrapper_id = child.id.clone();
                replaced_wrapper_ids.insert(wrapper_id);
                ordinary_children.push(provider_snapshot(entry, child.children));
            } else {
                ordinary_children.push(child);
            }
        }

        for entry in entries {
            if entry.origin_payload_root().is_none() {
                let provider_id = entry.projection.automation_node_id().clone();
                unmaterialized_ids.insert(provider_id);
                ordinary_children.push(provider_snapshot(entry, Vec::new()));
            }
        }
        if replaced_wrapper_ids != materialized_wrapper_ids {
            return false;
        }
        node.children = ordinary_children;
        return true;
    }
    for child in &mut node.children {
        if apply_container_segment(child, anchor_id, entries, unmaterialized_ids) {
            return true;
        }
    }
    false
}

fn provider_snapshot(
    entry: &NormalizedEntry,
    children: Vec<AutomationNodeSnapshot>,
) -> AutomationNodeSnapshot {
    let mut snapshot = AutomationNodeSnapshot::from_semantics(
        entry.projection.automation_node_id().clone(),
        AutomationBounds::from_rect(entry.projection.bounds()),
        entry.projection.semantics().clone(),
    );
    snapshot.children = children;
    snapshot
}

fn audit_unique_ids(node: &AutomationNodeSnapshot, ids: &mut HashSet<AutomationNodeId>) -> bool {
    if !ids.insert(node.id.clone()) {
        return false;
    }
    node.children
        .iter()
        .all(|child| audit_unique_ids(child, ids))
}

fn is_strict_prefix(prefix: &NodePath, path: &NodePath) -> bool {
    prefix.0.len() < path.0.len() && path.0.starts_with(&prefix.0)
}

trait NormalizedEntryOrigin {
    fn origin_payload_root(&self) -> Option<NodeId>;
}

impl NormalizedEntryOrigin for NormalizedEntry {
    fn origin_payload_root(&self) -> Option<NodeId> {
        match self.origin {
            VirtualLayoutSemanticClassificationOrigin::Materialized { payload_root, .. } => {
                Some(payload_root)
            }
            VirtualLayoutSemanticClassificationOrigin::Unmaterialized => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::virtual_layout::VirtualLayoutSemanticClassification;
    use super::*;
    use crate::{
        gui::{
            automation::{
                AutomationBounds, AutomationNodeId, AutomationNodeSemantics,
                AutomationNodeSnapshot, AutomationRole, GuiAutomationSnapshot,
            },
            layout_core::{
                VirtualLayoutBudget, VirtualLayoutCoordinateSpace, VirtualLayoutItemKey,
                VirtualLayoutSemanticEntry, VirtualLayoutSemanticRange,
                VirtualLayoutSemanticRangeRequest, VirtualLayoutSlotIdentity,
            },
        },
        layout::NodeId,
    };

    const CONTAINER_ID: NodeId = 10;
    const WRAPPER_ID: NodeId = 11;

    fn semantics(label: &str) -> AutomationNodeSemantics {
        AutomationNodeSemantics::new(AutomationRole::Button).with_label(label)
    }

    fn snapshot_node(id: &str, children: Vec<AutomationNodeSnapshot>) -> AutomationNodeSnapshot {
        let mut node = AutomationNodeSnapshot::from_semantics(
            AutomationNodeId::new(id),
            AutomationBounds {
                x: 0.0,
                y: 0.0,
                width: 100.0,
                height: 20.0,
            },
            AutomationNodeSemantics::new(AutomationRole::Group),
        );
        node.children = children;
        node
    }

    fn ordinary_snapshot() -> GuiAutomationSnapshot {
        GuiAutomationSnapshot {
            schema_version: 2,
            viewport_width: 200,
            viewport_height: 100,
            root: snapshot_node(
                "1",
                vec![snapshot_node(
                    &CONTAINER_ID.to_string(),
                    vec![
                        snapshot_node("2", Vec::new()),
                        snapshot_node(
                            &WRAPPER_ID.to_string(),
                            vec![snapshot_node("12", Vec::new())],
                        ),
                    ],
                )],
            ),
        }
    }

    fn request_for(
        container_id: NodeId,
        start: usize,
        length: usize,
        budget: usize,
        coordinate_space: VirtualLayoutCoordinateSpace,
    ) -> VirtualLayoutSemanticRangeRequest {
        VirtualLayoutSemanticRangeRequest::new(
            container_id,
            crate::layout::VirtualLayoutPolicyIdentity::new("test-policy"),
            1,
            2,
            3,
            4,
            5,
            coordinate_space,
            VirtualLayoutBudget::new(budget),
            VirtualLayoutSemanticRange::new(start, length).expect("valid range"),
        )
    }

    fn request(start: usize, length: usize) -> VirtualLayoutSemanticRangeRequest {
        request_for(
            CONTAINER_ID,
            start,
            length,
            8,
            VirtualLayoutCoordinateSpace::Logical,
        )
    }

    fn projection(
        range_request: &VirtualLayoutSemanticRangeRequest,
        index: usize,
        key: u32,
        id: &str,
        label: &str,
    ) -> VirtualLayoutSemanticProjection {
        let entry = VirtualLayoutSemanticEntry::new(
            VirtualLayoutItemKey::new(key),
            index,
            crate::gui::types::Rect::from_xy_size(1.0, index as f32 * 10.0, 20.0, 8.0),
            semantics(label),
            AutomationNodeId::new(id),
        );
        VirtualLayoutSemanticProjection::from_validated_semantic_range_entry(
            range_request,
            &entry,
            range_request.coordinate_space().clone(),
        )
        .expect("valid test projection")
    }

    fn classification(
        range_request: &VirtualLayoutSemanticRangeRequest,
        index: usize,
        key: u32,
        id: &str,
        label: &str,
        origin: VirtualLayoutSemanticClassificationOrigin,
    ) -> VirtualLayoutSemanticClassification {
        VirtualLayoutSemanticClassification::new(
            projection(range_request, index, key, id, label),
            origin,
        )
    }

    fn batch(
        request: &VirtualLayoutSemanticRangeRequest,
        classifications: Vec<VirtualLayoutSemanticClassification>,
    ) -> VirtualLayoutSemanticClassificationBatch {
        VirtualLayoutSemanticClassificationBatch::new(request.clone(), classifications)
    }

    fn unmaterialized(
        request: &VirtualLayoutSemanticRangeRequest,
        index: usize,
        key: u32,
        id: &str,
        label: &str,
    ) -> VirtualLayoutSemanticClassification {
        classification(
            request,
            index,
            key,
            id,
            label,
            VirtualLayoutSemanticClassificationOrigin::Unmaterialized,
        )
    }

    fn materialized(
        request: &VirtualLayoutSemanticRangeRequest,
        index: usize,
        key: u32,
        id: &str,
        label: &str,
        payload_root: NodeId,
    ) -> VirtualLayoutSemanticClassification {
        classification(
            request,
            index,
            key,
            id,
            label,
            VirtualLayoutSemanticClassificationOrigin::Materialized {
                slot_identity: VirtualLayoutSlotIdentity::from_test_parts(
                    request.container_id(),
                    request.mount_generation(),
                    index,
                    1,
                ),
                payload_root,
            },
        )
    }

    fn container_children(
        snapshot: &GuiAutomationSnapshot,
        container_id: NodeId,
    ) -> &[AutomationNodeSnapshot] {
        fn find<'a>(
            node: &'a AutomationNodeSnapshot,
            id: &AutomationNodeId,
        ) -> Option<&'a AutomationNodeSnapshot> {
            if &node.id == id {
                return Some(node);
            }
            node.children.iter().find_map(|child| find(child, id))
        }
        &find(
            &snapshot.root,
            &AutomationNodeId::new(container_id.to_string()),
        )
        .expect("test container")
        .children
    }

    fn two_container_snapshot() -> GuiAutomationSnapshot {
        let mut snapshot = ordinary_snapshot();
        snapshot.root.children.push(snapshot_node("20", Vec::new()));
        snapshot
    }

    fn node_by_id<'a>(
        node: &'a AutomationNodeSnapshot,
        id: &str,
    ) -> Option<&'a AutomationNodeSnapshot> {
        if node.id == AutomationNodeId::new(id) {
            return Some(node);
        }
        node.children.iter().find_map(|child| node_by_id(child, id))
    }

    #[test]
    fn no_virtual_input_preserves_ordinary_snapshot() {
        let ordinary = ordinary_snapshot();
        let composed = compose_virtual_layout_automation_snapshot(&ordinary, &[])
            .expect("empty composition succeeds");
        assert_eq!(composed.snapshot(), &ordinary);
        assert!(composed.unmaterialized_ids().is_empty());
    }

    #[test]
    fn logical_composition_replaces_materialized_wrapper_and_appends_unmaterialized_leaf() {
        let ordinary = ordinary_snapshot();
        let request = request(0, 2);
        let classifications = batch(
            &request,
            vec![
                materialized(&request, 0, 10, "provider-10", "materialized", WRAPPER_ID),
                unmaterialized(&request, 1, 11, "provider-11", "unmaterialized"),
            ],
        );

        let composed = compose_virtual_layout_automation_snapshot(&ordinary, &[classifications])
            .expect("logical evidence should compose");
        let children = container_children(composed.snapshot(), CONTAINER_ID);
        assert_eq!(
            children
                .iter()
                .map(|child| child.id.0.as_str())
                .collect::<Vec<_>>(),
            vec!["2", "provider-10", "provider-11"]
        );
        assert_eq!(children[1].children[0].id, AutomationNodeId::new("12"));
        assert_eq!(children[1].semantics.label.as_deref(), Some("materialized"));
        assert_eq!(children[1].available_actions, vec!["press"]);
        assert_eq!(
            children[2].semantics.label.as_deref(),
            Some("unmaterialized")
        );
        assert!(node_by_id(&composed.snapshot().root, &WRAPPER_ID.to_string()).is_none());

        let target_snapshot = composed.target_snapshot(77);
        let materialized_target = target_snapshot
            .targets
            .iter()
            .find(|target| target.id == AutomationNodeId::new("provider-10"))
            .expect("materialized provider target");
        assert_eq!(
            materialized_target.authority,
            Some(AutomationTargetAuthority::materialized(77))
        );
        let unmaterialized_target = target_snapshot
            .targets
            .iter()
            .find(|target| target.id == AutomationNodeId::new("provider-11"))
            .expect("unmaterialized provider target");
        assert_eq!(
            unmaterialized_target.authority,
            Some(AutomationTargetAuthority {
                runtime_generation: 77,
                materialized: false,
            })
        );
        let payload_target = target_snapshot
            .targets
            .iter()
            .find(|target| target.id == AutomationNodeId::new("12"))
            .expect("preserved payload target");
        assert_eq!(
            payload_target.authority,
            Some(AutomationTargetAuthority::materialized(77))
        );
    }

    #[test]
    fn materialized_wrapper_replacement_preserves_exact_child_position() {
        let mut ordinary = ordinary_snapshot();
        let container = &mut ordinary.root.children[0];
        let wrapper = container.children.pop().expect("wrapper child");
        container.children.push(wrapper);
        container.children.push(snapshot_node("3", Vec::new()));
        let request = request(0, 1);

        let composed = compose_virtual_layout_automation_snapshot(
            &ordinary,
            &[batch(
                &request,
                vec![materialized(
                    &request,
                    0,
                    13,
                    "provider-position",
                    "position",
                    WRAPPER_ID,
                )],
            )],
        )
        .expect("materialized wrapper should be replaced in place");

        assert_eq!(
            container_children(composed.snapshot(), CONTAINER_ID)
                .iter()
                .map(|child| child.id.0.as_str())
                .collect::<Vec<_>>(),
            vec!["2", "provider-position", "3"]
        );
        assert_eq!(
            container_children(composed.snapshot(), CONTAINER_ID)[1].children[0].id,
            AutomationNodeId::new("12")
        );
    }

    #[test]
    fn reversed_range_input_has_deterministic_union_order() {
        let ordinary = ordinary_snapshot();
        let first_request = request(0, 1);
        let second_request =
            request_for(CONTAINER_ID, 1, 1, 8, VirtualLayoutCoordinateSpace::Logical);
        let first = batch(
            &first_request,
            vec![unmaterialized(
                &first_request,
                0,
                20,
                "provider-20",
                "first",
            )],
        );
        let second = batch(
            &second_request,
            vec![unmaterialized(
                &second_request,
                1,
                21,
                "provider-21",
                "second",
            )],
        );

        let forward =
            compose_virtual_layout_automation_snapshot(&ordinary, &[first.clone(), second.clone()])
                .expect("forward ranges should compose");
        let reversed = compose_virtual_layout_automation_snapshot(&ordinary, &[second, first])
            .expect("reversed ranges should compose");
        assert_eq!(forward, reversed);
        assert_eq!(
            container_children(forward.snapshot(), CONTAINER_ID)
                .iter()
                .map(|child| child.id.0.as_str())
                .collect::<Vec<_>>(),
            vec!["2", "11", "provider-20", "provider-21"]
        );
    }

    #[test]
    fn mismatched_registration_fence_rejects_cross_range_union() {
        let ordinary = ordinary_snapshot();
        let first_request = request(0, 1);
        let second_request = VirtualLayoutSemanticRangeRequest::new(
            CONTAINER_ID,
            crate::layout::VirtualLayoutPolicyIdentity::new("test-policy"),
            1,
            99,
            3,
            4,
            5,
            VirtualLayoutCoordinateSpace::Logical,
            VirtualLayoutBudget::new(8),
            VirtualLayoutSemanticRange::new(1, 1).expect("valid range"),
        );
        let result = compose_virtual_layout_automation_snapshot(
            &ordinary,
            &[
                batch(
                    &first_request,
                    vec![unmaterialized(
                        &first_request,
                        0,
                        22,
                        "provider-fence-zero",
                        "zero",
                    )],
                ),
                batch(
                    &second_request,
                    vec![unmaterialized(
                        &second_request,
                        1,
                        23,
                        "provider-fence-one",
                        "one",
                    )],
                ),
            ],
        );

        assert_eq!(
            result,
            Err(VirtualLayoutAutomationCompositionError::MalformedClassification)
        );
        assert_eq!(ordinary, ordinary_snapshot());
    }

    #[test]
    fn exact_overlap_coalesces_once() {
        let ordinary = ordinary_snapshot();
        let request = request(0, 1);
        let first = batch(
            &request,
            vec![unmaterialized(&request, 0, 30, "provider-30", "same")],
        );
        let second = first.clone();
        let composed = compose_virtual_layout_automation_snapshot(&ordinary, &[first, second])
            .expect("exact overlap should coalesce");
        let children = container_children(composed.snapshot(), CONTAINER_ID);
        assert_eq!(
            children
                .iter()
                .filter(|child| child.id == AutomationNodeId::new("provider-30"))
                .count(),
            1
        );
    }

    #[test]
    fn conflicting_overlap_rejects_atomically() {
        let ordinary = ordinary_snapshot();
        let request = request(0, 1);
        let first = batch(
            &request,
            vec![unmaterialized(&request, 0, 31, "provider-31", "first")],
        );
        let second = batch(
            &request,
            vec![unmaterialized(&request, 0, 31, "provider-31", "different")],
        );
        let result = compose_virtual_layout_automation_snapshot(&ordinary, &[first, second]);
        assert_eq!(
            result,
            Err(VirtualLayoutAutomationCompositionError::ConflictingOverlap)
        );
        assert_eq!(ordinary, ordinary_snapshot());
    }

    #[test]
    fn key_index_and_duplicate_index_failures_are_typed() {
        let ordinary = ordinary_snapshot();
        let first_request = request(0, 1);
        let second_request = request(1, 1);
        let key_drift = compose_virtual_layout_automation_snapshot(
            &ordinary,
            &[
                batch(
                    &first_request,
                    vec![unmaterialized(&first_request, 0, 32, "provider-32", "zero")],
                ),
                batch(
                    &second_request,
                    vec![unmaterialized(&second_request, 1, 32, "provider-32", "one")],
                ),
            ],
        );
        assert_eq!(
            key_drift,
            Err(VirtualLayoutAutomationCompositionError::KeyIndexDrift)
        );

        let duplicate_index = compose_virtual_layout_automation_snapshot(
            &ordinary,
            &[
                batch(
                    &first_request,
                    vec![unmaterialized(&first_request, 0, 33, "provider-33", "one")],
                ),
                batch(
                    &first_request,
                    vec![unmaterialized(&first_request, 0, 34, "provider-34", "two")],
                ),
            ],
        );
        assert_eq!(
            duplicate_index,
            Err(VirtualLayoutAutomationCompositionError::DuplicateLogicalIndex)
        );
        assert_eq!(ordinary, ordinary_snapshot());
    }

    #[test]
    fn payload_root_failures_reject_without_partial_tree() {
        let ordinary = ordinary_snapshot();
        let request = request(0, 1);
        let missing = compose_virtual_layout_automation_snapshot(
            &ordinary,
            &[batch(
                &request,
                vec![materialized(&request, 0, 40, "provider-40", "missing", 999)],
            )],
        );
        assert_eq!(
            missing,
            Err(VirtualLayoutAutomationCompositionError::MissingPayloadRoot)
        );

        let mut duplicate_snapshot = ordinary.clone();
        duplicate_snapshot.root.children[0]
            .children
            .push(snapshot_node(&WRAPPER_ID.to_string(), Vec::new()));
        let duplicate = compose_virtual_layout_automation_snapshot(
            &duplicate_snapshot,
            &[batch(
                &request,
                vec![materialized(
                    &request,
                    0,
                    41,
                    "provider-41",
                    "duplicate",
                    WRAPPER_ID,
                )],
            )],
        );
        assert_eq!(
            duplicate,
            Err(VirtualLayoutAutomationCompositionError::DuplicatePayloadRoot)
        );

        let mut wrong_parent_snapshot = ordinary.clone();
        let wrapper = wrong_parent_snapshot.root.children[0]
            .children
            .pop()
            .expect("wrapper child");
        wrong_parent_snapshot.root.children.push(wrapper);
        let wrong_parent = compose_virtual_layout_automation_snapshot(
            &wrong_parent_snapshot,
            &[batch(
                &request,
                vec![materialized(
                    &request,
                    0,
                    42,
                    "provider-42",
                    "wrong-parent",
                    WRAPPER_ID,
                )],
            )],
        );
        assert_eq!(
            wrong_parent,
            Err(VirtualLayoutAutomationCompositionError::WrongParentPayloadRoot)
        );

        let second_request =
            request_for(CONTAINER_ID, 1, 1, 8, VirtualLayoutCoordinateSpace::Logical);
        let duplicate_root = compose_virtual_layout_automation_snapshot(
            &ordinary,
            &[
                batch(
                    &request,
                    vec![materialized(
                        &request,
                        0,
                        43,
                        "provider-43",
                        "first-root",
                        WRAPPER_ID,
                    )],
                ),
                batch(
                    &second_request,
                    vec![materialized(
                        &second_request,
                        1,
                        44,
                        "provider-44",
                        "second-root",
                        WRAPPER_ID,
                    )],
                ),
            ],
        );
        assert_eq!(
            duplicate_root,
            Err(VirtualLayoutAutomationCompositionError::DuplicatePayloadRoot)
        );
        assert_eq!(ordinary, ordinary_snapshot());
    }

    #[test]
    fn anchor_failures_include_missing_duplicate_nested_and_wrong_parent() {
        let ordinary = ordinary_snapshot();
        let missing_request = request_for(99, 0, 1, 8, VirtualLayoutCoordinateSpace::Logical);
        let missing = compose_virtual_layout_automation_snapshot(
            &ordinary,
            &[batch(
                &missing_request,
                vec![unmaterialized(
                    &missing_request,
                    0,
                    50,
                    "provider-50",
                    "missing",
                )],
            )],
        );
        assert_eq!(
            missing,
            Err(VirtualLayoutAutomationCompositionError::MissingContainerAnchor)
        );

        let mut duplicate_snapshot = ordinary.clone();
        duplicate_snapshot
            .root
            .children
            .push(snapshot_node(&CONTAINER_ID.to_string(), Vec::new()));
        let duplicate = compose_virtual_layout_automation_snapshot(
            &duplicate_snapshot,
            &[batch(
                &request(0, 1),
                vec![unmaterialized(
                    &request(0, 1),
                    0,
                    51,
                    "provider-51",
                    "duplicate",
                )],
            )],
        );
        assert_eq!(
            duplicate,
            Err(VirtualLayoutAutomationCompositionError::DuplicateContainerAnchor)
        );

        let mut nested_snapshot = ordinary.clone();
        let nested_container = nested_snapshot.root.children.remove(0);
        nested_snapshot
            .root
            .children
            .push(snapshot_node("20", vec![nested_container]));
        let nested_request = request_for(20, 0, 1, 8, VirtualLayoutCoordinateSpace::Logical);
        let nested = compose_virtual_layout_automation_snapshot(
            &nested_snapshot,
            &[
                batch(
                    &request(0, 1),
                    vec![unmaterialized(
                        &request(0, 1),
                        0,
                        52,
                        "provider-52",
                        "nested-a",
                    )],
                ),
                batch(
                    &nested_request,
                    vec![unmaterialized(
                        &nested_request,
                        0,
                        53,
                        "provider-53",
                        "nested-b",
                    )],
                ),
            ],
        );
        assert_eq!(
            nested,
            Err(VirtualLayoutAutomationCompositionError::NestedContainerAnchor)
        );

        let mut wrong_parent_snapshot = two_container_snapshot();
        let container = wrong_parent_snapshot.root.children.remove(0);
        let wrapper = snapshot_node("50", vec![container]);
        wrong_parent_snapshot.root.children[0]
            .children
            .push(wrapper);
        let second_container_request =
            request_for(20, 0, 1, 8, VirtualLayoutCoordinateSpace::Logical);
        let wrong_parent = compose_virtual_layout_automation_snapshot(
            &wrong_parent_snapshot,
            &[
                batch(
                    &second_container_request,
                    vec![materialized(
                        &second_container_request,
                        0,
                        54,
                        "provider-54",
                        "wrapper-parent",
                        50,
                    )],
                ),
                batch(
                    &request(0, 1),
                    vec![unmaterialized(
                        &request(0, 1),
                        0,
                        55,
                        "provider-55",
                        "anchor-parent",
                    )],
                ),
            ],
        );
        assert_eq!(
            wrong_parent,
            Err(VirtualLayoutAutomationCompositionError::WrongParentAnchor)
        );
    }

    #[test]
    fn custom_coordinate_space_is_rejected_before_insertion() {
        let ordinary = ordinary_snapshot();
        let request = request_for(
            CONTAINER_ID,
            0,
            1,
            8,
            VirtualLayoutCoordinateSpace::custom(crate::layout::VirtualLayoutPolicyIdentity::new(
                "custom-space",
            )),
        );
        let result = compose_virtual_layout_automation_snapshot(
            &ordinary,
            &[batch(
                &request,
                vec![unmaterialized(&request, 0, 60, "provider-60", "custom")],
            )],
        );
        assert_eq!(
            result,
            Err(VirtualLayoutAutomationCompositionError::CoordinateTransformUnavailable)
        );
        assert_eq!(ordinary, ordinary_snapshot());
    }

    #[test]
    fn aggregate_budget_and_hard_cap_are_deterministic() {
        let ordinary = ordinary_snapshot();
        let first_request =
            request_for(CONTAINER_ID, 0, 2, 2, VirtualLayoutCoordinateSpace::Logical);
        let second_request =
            request_for(CONTAINER_ID, 2, 2, 2, VirtualLayoutCoordinateSpace::Logical);
        let over_budget = compose_virtual_layout_automation_snapshot(
            &ordinary,
            &[
                batch(
                    &first_request,
                    vec![
                        unmaterialized(&first_request, 0, 70, "provider-70", "zero"),
                        unmaterialized(&first_request, 1, 71, "provider-71", "one"),
                    ],
                ),
                batch(
                    &second_request,
                    vec![
                        unmaterialized(&second_request, 2, 72, "provider-72", "two"),
                        unmaterialized(&second_request, 3, 73, "provider-73", "three"),
                    ],
                ),
            ],
        );
        assert_eq!(
            over_budget,
            Err(VirtualLayoutAutomationCompositionError::AggregateBudgetExceeded)
        );

        let hard_cap = VIRTUAL_LAYOUT_MAX_QUERY_ENTRIES;
        let mut batches = Vec::with_capacity(hard_cap + 1);
        for index in 0..=hard_cap {
            let request = request_for(
                CONTAINER_ID,
                index,
                1,
                hard_cap + 1,
                VirtualLayoutCoordinateSpace::Logical,
            );
            batches.push(batch(
                &request,
                vec![unmaterialized(
                    &request,
                    index,
                    1_000 + index as u32,
                    &format!("provider-hard-{index}"),
                    "hard-cap",
                )],
            ));
        }
        let hard_cap_result = compose_virtual_layout_automation_snapshot(&ordinary, &batches);
        assert_eq!(
            hard_cap_result,
            Err(VirtualLayoutAutomationCompositionError::HardQueryCapExceeded)
        );
        assert_eq!(ordinary, ordinary_snapshot());
    }

    #[test]
    fn global_id_collision_rejects_but_exact_wrapper_displacement_is_allowed() {
        let ordinary = ordinary_snapshot();
        let request = request(0, 1);
        let collision = compose_virtual_layout_automation_snapshot(
            &ordinary,
            &[batch(
                &request,
                vec![unmaterialized(&request, 0, 80, "2", "ordinary-collision")],
            )],
        );
        assert_eq!(
            collision,
            Err(VirtualLayoutAutomationCompositionError::AutomationNodeIdCollision)
        );

        let displacement = compose_virtual_layout_automation_snapshot(
            &ordinary,
            &[batch(
                &request,
                vec![materialized(
                    &request,
                    0,
                    81,
                    &WRAPPER_ID.to_string(),
                    "same-generated-id",
                    WRAPPER_ID,
                )],
            )],
        )
        .expect("only the exact replaced wrapper may be displaced");
        assert_eq!(
            container_children(displacement.snapshot(), CONTAINER_ID)
                .iter()
                .filter(|child| child.id == AutomationNodeId::new(WRAPPER_ID.to_string()))
                .count(),
            1
        );

        let descendant_collision = compose_virtual_layout_automation_snapshot(
            &ordinary,
            &[batch(
                &request,
                vec![unmaterialized(
                    &request,
                    0,
                    82,
                    "12",
                    "descendant-collision",
                )],
            )],
        );
        assert_eq!(
            descendant_collision,
            Err(VirtualLayoutAutomationCompositionError::AutomationNodeIdCollision)
        );

        let second_request =
            request_for(CONTAINER_ID, 1, 1, 8, VirtualLayoutCoordinateSpace::Logical);
        let range_collision = compose_virtual_layout_automation_snapshot(
            &ordinary,
            &[
                batch(
                    &request,
                    vec![unmaterialized(
                        &request,
                        0,
                        83,
                        "range-collision",
                        "first-range",
                    )],
                ),
                batch(
                    &second_request,
                    vec![unmaterialized(
                        &second_request,
                        1,
                        84,
                        "range-collision",
                        "second-range",
                    )],
                ),
            ],
        );
        assert_eq!(
            range_collision,
            Err(VirtualLayoutAutomationCompositionError::AutomationNodeIdCollision)
        );
    }

    #[test]
    fn final_uniqueness_audit_rejects_preexisting_duplicate_descendants_atomically() {
        let mut ordinary = ordinary_snapshot();
        ordinary.root.children[0]
            .children
            .push(snapshot_node("2", Vec::new()));
        let before = ordinary.clone();
        let request = request(0, 1);

        let result = compose_virtual_layout_automation_snapshot(
            &ordinary,
            &[batch(
                &request,
                vec![unmaterialized(
                    &request,
                    0,
                    85,
                    "provider-duplicate-ordinary",
                    "duplicate-ordinary",
                )],
            )],
        );

        assert_eq!(
            result,
            Err(VirtualLayoutAutomationCompositionError::AutomationNodeIdCollision)
        );
        assert_eq!(ordinary, before);
    }

    #[derive(Clone)]
    struct UnstableIdentity(std::rc::Rc<std::cell::Cell<bool>>);

    impl PartialEq for UnstableIdentity {
        fn eq(&self, _other: &Self) -> bool {
            let result = self.0.get();
            self.0.set(!result);
            result
        }
    }

    impl Eq for UnstableIdentity {}

    #[test]
    fn unstable_equality_is_rejected_before_tree_mutation() {
        let ordinary = ordinary_snapshot();
        let request = VirtualLayoutSemanticRangeRequest::new(
            CONTAINER_ID,
            crate::layout::VirtualLayoutPolicyIdentity::new(UnstableIdentity(std::rc::Rc::new(
                std::cell::Cell::new(true),
            ))),
            1,
            2,
            3,
            4,
            5,
            VirtualLayoutCoordinateSpace::Logical,
            VirtualLayoutBudget::new(8),
            VirtualLayoutSemanticRange::new(0, 1).expect("valid range"),
        );
        let result = compose_virtual_layout_automation_snapshot(
            &ordinary,
            &[batch(
                &request,
                vec![unmaterialized(&request, 0, 90, "provider-90", "unstable")],
            )],
        );
        assert_eq!(
            result,
            Err(VirtualLayoutAutomationCompositionError::UnstableEquality)
        );
        assert_eq!(ordinary, ordinary_snapshot());
    }
}
