use super::source::{OverlayEvidence, SourceCompatibility, SourceIdentity, SourceMetadata};
use super::widget::WidgetCapabilityEvidence;
use super::{UiSurface, WidgetDispatchResult, WidgetPath};
use crate::application::DeclarativeEffectOwner;
use crate::widgets::{WidgetId, WidgetRevision};
use std::collections::{HashMap, HashSet};
use std::panic::{AssertUnwindSafe, catch_unwind};

/// Read-only evidence for one retained-widget replacement boundary.
///
/// The plan owns only identity, compatibility, revision, and validity
/// evidence. It deliberately does not retain widget references, output
/// values, host messages, or mapper callbacks.
#[derive(Clone, PartialEq, Eq)]
struct WidgetReplacementEvidence {
    widget_id: WidgetId,
    compatibility_kind: &'static str,
    revision: WidgetRevision,
    valid: bool,
}

impl WidgetReplacementEvidence {
    fn from_widget<Message>(widget: &super::SurfaceWidget<Message>) -> Self {
        let evidence = widget.revision_evidence();
        Self {
            widget_id: widget.id(),
            compatibility_kind: evidence.compatibility_kind,
            revision: evidence.revision.clone(),
            valid: evidence.valid,
        }
    }
}

/// One owned read-only replacement witness retained until commit.
struct WidgetReplacementPlanEntry {
    widget_id: WidgetId,
    previous_path: Option<WidgetPath>,
    previous_evidence: Option<WidgetReplacementEvidence>,
    successor_path: Option<WidgetPath>,
    successor_evidence: Option<WidgetReplacementEvidence>,
    previous_unique: bool,
    successor_unique: bool,
}

/// A crate-private, consuming replacement plan.
///
/// This type intentionally does not implement `Clone`. Dropping it is an
/// inert discard of read-only evidence; irreversible widget callbacks and
/// message mapping occur only through [`UiSurface::commit_widget_replacements`].
pub(in crate::runtime) struct WidgetReplacementPlan {
    entries: Vec<WidgetReplacementPlanEntry>,
}

impl WidgetReplacementPlan {
    pub(in crate::runtime) fn empty() -> Self {
        Self {
            entries: Vec::new(),
        }
    }
}

/// A replacement plan whose complete read-only evidence has been validated.
///
/// This token intentionally does not implement `Clone`.  Its only consumer is
/// the callback commit method, so a caller cannot validate once and then reuse
/// the same replacement evidence for a second callback batch.
pub(in crate::runtime) struct ValidatedWidgetReplacementPlan {
    entries: Vec<WidgetReplacementPlanEntry>,
}

/// Reason a complete replacement plan was vetoed before its first callback.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::runtime) enum WidgetReplacementPlanVeto {
    /// Stored identity/path/revision or uniqueness evidence no longer matches
    /// the surfaces supplied to commit.
    StaleEvidence,
}

/// Results produced by consuming one validated replacement plan.
pub(in crate::runtime) struct WidgetReplacementCommitResult<Message> {
    pub(in crate::runtime) terminal_messages: Vec<Message>,
    pub(in crate::runtime) retired_widget_ids: Vec<WidgetId>,
    pub(in crate::runtime) veto: Option<WidgetReplacementPlanVeto>,
}

/// Results produced after a validated replacement plan has entered its
/// irreversible callback sequence.
pub(in crate::runtime) struct ValidatedWidgetReplacementCommitResult<Message> {
    pub(in crate::runtime) terminal_messages: Vec<Message>,
    pub(in crate::runtime) retired_widget_ids: Vec<WidgetId>,
}

pub(in crate::runtime) struct WidgetStateSyncEvidence<'a> {
    pub(in crate::runtime::surface) stateful_widget_order: &'a [WidgetId],
    pub(in crate::runtime::surface) current_paths: &'a HashMap<WidgetId, WidgetPath>,
    pub(in crate::runtime::surface) previous_paths: &'a HashMap<WidgetId, WidgetPath>,
    pub(in crate::runtime::surface) previous_widget_order: &'a [WidgetId],
    pub(in crate::runtime::surface) current_widget_order: &'a [WidgetId],
    pub(in crate::runtime::surface) retired_widget_ids: &'a [WidgetId],
    pub(in crate::runtime::surface) policy: WidgetStateSyncPolicy,
}

/// Typed veto for the private candidate-only retained-state synchronization
/// boundary.
///
/// All non-panic variants are discovered by the complete preflight before the
/// first successor callback. A panic is caught around the candidate-owned
/// batch; the caller drops the candidate and therefore never publishes partial
/// successor state.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::runtime) enum PreparedWidgetStateSyncVeto {
    Unsupported,
    Ambiguous,
    InvalidIdentity,
    InvalidPath,
    InvalidRevision,
    Incompatible,
    Panicked,
}

/// Complete identity/path evidence for one candidate-only state-sync batch.
#[derive(Clone, Copy)]
pub(in crate::runtime) struct PreparedWidgetStateSyncEvidence<'a> {
    pub(in crate::runtime) stateful_widget_order: &'a [WidgetId],
    pub(in crate::runtime) current_paths: &'a HashMap<WidgetId, WidgetPath>,
    pub(in crate::runtime) previous_paths: &'a HashMap<WidgetId, WidgetPath>,
    pub(in crate::runtime) previous_widget_order: &'a [WidgetId],
    pub(in crate::runtime) current_widget_order: &'a [WidgetId],
    pub(in crate::runtime) policy: WidgetStateSyncPolicy,
}

/// Immutable witness captured for every selected predecessor/successor leaf
/// before the first synchronization callback.  The live fields are queried
/// from the erased widget object; the cached fields come from the
/// `SurfaceWidget` boundary record. Keeping both prevents runtime-owned state
/// mutation from masquerading as unchanged declarative evidence.
pub(in crate::runtime) struct PreparedWidgetStateSyncWitness {
    pub(in crate::runtime) entries: Vec<PreparedWidgetStateSyncLeafWitness>,
}

pub(in crate::runtime) struct PreparedWidgetStateSyncLeafWitness {
    pub(in crate::runtime) widget_id: WidgetId,
    pub(in crate::runtime) previous_path: WidgetPath,
    pub(in crate::runtime) current_path: WidgetPath,
    previous_cached_revision: WidgetRevision,
    previous_live_revision: WidgetRevision,
    current_cached_revision: WidgetRevision,
    current_live_revision: WidgetRevision,
    previous_cached_capabilities: WidgetCapabilityEvidence,
    previous_live_capabilities: WidgetCapabilityEvidence,
    current_cached_capabilities: WidgetCapabilityEvidence,
    current_live_capabilities: WidgetCapabilityEvidence,
    previous_support: bool,
    current_support: bool,
    previous_membership: [bool; 7],
    current_membership: [bool; 7],
    previous_cached_id: WidgetId,
    previous_live_id: WidgetId,
    current_cached_id: WidgetId,
    current_live_id: WidgetId,
    previous_sources: Vec<Option<PreparedSourceMetadataWitness>>,
    current_sources: Vec<Option<PreparedSourceMetadataWitness>>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct PreparedSourceMetadataWitness {
    identity: SourceIdentity,
    compatibility: SourceCompatibility,
    keyed_nodes: Vec<(
        SourceIdentity,
        SourceCompatibility,
        Option<DeclarativeEffectOwner>,
    )>,
    overlays: Vec<OverlayEvidence>,
}

impl PreparedSourceMetadataWitness {
    fn capture(metadata: &SourceMetadata) -> Self {
        Self {
            identity: metadata.identity,
            compatibility: metadata.compatibility,
            keyed_nodes: metadata
                .topology
                .keyed_nodes
                .iter()
                .map(|node| (node.identity(), node.compatibility(), node.effect_owner()))
                .collect(),
            overlays: metadata.topology.overlays.clone(),
        }
    }
}

fn freeze_source_path(
    sources: &[Option<std::rc::Rc<SourceMetadata>>],
) -> Vec<Option<PreparedSourceMetadataWitness>> {
    sources
        .iter()
        .map(|source| {
            source
                .as_deref()
                .map(PreparedSourceMetadataWitness::capture)
        })
        .collect()
}

impl PreparedWidgetStateSyncLeafWitness {
    pub(super) fn capture<Message>(
        widget_id: WidgetId,
        previous_path: WidgetPath,
        current_path: WidgetPath,
        previous: &super::SurfaceWidget<Message>,
        current: &super::SurfaceWidget<Message>,
        previous_sources: Vec<Option<std::rc::Rc<SourceMetadata>>>,
        current_sources: Vec<Option<std::rc::Rc<SourceMetadata>>>,
    ) -> Self {
        let previous_cached = previous.revision_evidence();
        let current_cached = current.revision_evidence();
        Self {
            widget_id,
            previous_path,
            current_path,
            previous_cached_revision: previous_cached.revision.clone(),
            previous_live_revision: previous.live_revision(),
            current_cached_revision: current_cached.revision.clone(),
            current_live_revision: current.live_revision(),
            previous_cached_capabilities: previous_cached.capabilities.clone(),
            previous_live_capabilities: previous.live_capability_evidence(),
            current_cached_capabilities: current_cached.capabilities.clone(),
            current_live_capabilities: current.live_capability_evidence(),
            previous_support: previous.supports_prepared_state_synchronization(),
            current_support: current.supports_prepared_state_synchronization(),
            previous_membership: previous.prepared_state_membership(),
            current_membership: current.prepared_state_membership(),
            previous_cached_id: previous_cached.id,
            previous_live_id: previous.id(),
            current_cached_id: current_cached.id,
            current_live_id: current.id(),
            previous_sources: freeze_source_path(&previous_sources),
            current_sources: freeze_source_path(&current_sources),
        }
    }

    fn current_is_unchanged<Message>(
        &self,
        current: &super::SurfaceWidget<Message>,
        current_sources: &[Option<std::rc::Rc<SourceMetadata>>],
    ) -> bool {
        let cached = current.revision_evidence();
        cached.valid
            && current.cached_revision_is_exact()
            && cached.revision == self.current_cached_revision
            && current.live_revision() == self.current_live_revision
            && cached.revision == current.live_revision()
            && cached.capabilities == self.current_cached_capabilities
            && current.live_capability_evidence() == self.current_live_capabilities
            && cached.capabilities == current.live_capability_evidence()
            && current.supports_prepared_state_synchronization() == self.current_support
            && current.prepared_state_membership() == self.current_membership
            && cached.id == self.current_cached_id
            && current.id() == self.current_live_id
            && cached.id == current.id()
            && freeze_source_path(current_sources) == self.current_sources
    }

    pub(super) fn previous_is_unchanged<Message>(
        &self,
        previous: &super::SurfaceWidget<Message>,
        previous_sources: &[Option<std::rc::Rc<SourceMetadata>>],
    ) -> bool {
        let cached = previous.revision_evidence();
        cached.valid
            && previous.cached_revision_is_exact()
            && cached.revision == self.previous_cached_revision
            && previous.live_revision() == self.previous_live_revision
            && cached.revision == previous.live_revision()
            && cached.capabilities == self.previous_cached_capabilities
            && previous.live_capability_evidence() == self.previous_live_capabilities
            && cached.capabilities == previous.live_capability_evidence()
            && previous.supports_prepared_state_synchronization() == self.previous_support
            && previous.prepared_state_membership() == self.previous_membership
            && cached.id == self.previous_cached_id
            && previous.id() == self.previous_live_id
            && cached.id == previous.id()
            && freeze_source_path(previous_sources) == self.previous_sources
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(in crate::runtime) struct WidgetStateSyncPolicy {
    exclusive_pointer_capture: Option<WidgetId>,
    retained_hover_owner: Option<WidgetId>,
}

impl WidgetStateSyncPolicy {
    pub(in crate::runtime) fn exclusive_pointer_capture(widget_id: WidgetId) -> Self {
        Self {
            exclusive_pointer_capture: Some(widget_id),
            retained_hover_owner: Some(widget_id),
        }
    }

    pub(in crate::runtime) fn retained_hover_owner(widget_id: Option<WidgetId>) -> Self {
        Self {
            exclusive_pointer_capture: None,
            retained_hover_owner: widget_id,
        }
    }

    pub(in crate::runtime) fn clears_retained_hover_for(self, widget_id: WidgetId) -> bool {
        if let Some(captured) = self.exclusive_pointer_capture {
            return captured != widget_id;
        }
        self.retained_hover_owner != Some(widget_id)
    }
}

impl<Message> UiSurface<Message> {
    pub(in crate::runtime) fn preflight_prepared_widget_state_sync(
        &self,
        previous: &Self,
        evidence: PreparedWidgetStateSyncEvidence<'_>,
    ) -> Result<PreparedWidgetStateSyncWitness, PreparedWidgetStateSyncVeto> {
        let result = catch_unwind(AssertUnwindSafe(|| {
            self.root
                .preflight_prepared_widget_state_sync(&evidence, &previous.root)
        }));
        match result {
            Ok(result) => result,
            Err(_) => Err(PreparedWidgetStateSyncVeto::Panicked),
        }
    }

    pub(in crate::runtime) fn synchronize_prepared_widget_state(
        &mut self,
        previous: &Self,
        evidence: PreparedWidgetStateSyncEvidence<'_>,
        witness: &PreparedWidgetStateSyncWitness,
    ) -> Result<(), PreparedWidgetStateSyncVeto> {
        let result = catch_unwind(AssertUnwindSafe(|| {
            self.root
                .synchronize_prepared_widget_state(&evidence, witness, &previous.root)
        }));
        match result {
            Ok(result) => result,
            Err(_) => Err(PreparedWidgetStateSyncVeto::Panicked),
        }
    }

    /// Revalidate candidate-only state-sync evidence after the callback batch
    /// without invoking any widget callback.
    pub(in crate::runtime) fn prepared_widget_state_sync_is_current(
        &self,
        witness: &PreparedWidgetStateSyncWitness,
    ) -> Result<(), PreparedWidgetStateSyncVeto> {
        let result = catch_unwind(AssertUnwindSafe(|| {
            for entry in &witness.entries {
                let current = self
                    .root
                    .find_widget_at_path(entry.current_path.as_slice())
                    .filter(|widget| widget.id() == entry.widget_id)
                    .ok_or(PreparedWidgetStateSyncVeto::InvalidIdentity)?;
                let current_sources = self
                    .root
                    .source_metadata_path_at(entry.current_path.as_slice())
                    .ok_or(PreparedWidgetStateSyncVeto::InvalidPath)?;
                if !entry.current_is_unchanged(current, &current_sources) {
                    return Err(PreparedWidgetStateSyncVeto::InvalidRevision);
                }
            }
            Ok(())
        }));
        match result {
            Ok(result) => result,
            Err(_) => Err(PreparedWidgetStateSyncVeto::Panicked),
        }
    }

    pub(in crate::runtime) fn plan_widget_replacements(
        &self,
        successor: &Self,
        previous_stateful_widget_order: &[WidgetId],
        previous_widget_order: &[WidgetId],
        current_widget_order: &[WidgetId],
        current_paths: &HashMap<WidgetId, WidgetPath>,
        previous_paths: &HashMap<WidgetId, WidgetPath>,
    ) -> WidgetReplacementPlan {
        self.plan_widget_replacements_for_ids(
            successor,
            previous_stateful_widget_order,
            previous_widget_order,
            current_widget_order,
            current_paths,
            previous_paths,
        )
    }

    pub(in crate::runtime) fn plan_widget_replacements_for_ids(
        &self,
        successor: &Self,
        stateful_widget_order: &[WidgetId],
        previous_widget_order: &[WidgetId],
        current_widget_order: &[WidgetId],
        current_paths: &HashMap<WidgetId, WidgetPath>,
        previous_paths: &HashMap<WidgetId, WidgetPath>,
    ) -> WidgetReplacementPlan {
        let mut entries = Vec::with_capacity(stateful_widget_order.len());
        let mut visited = HashSet::with_capacity(stateful_widget_order.len());

        for widget_id in stateful_widget_order {
            if !visited.insert(*widget_id) {
                continue;
            }

            let previous_path = previous_paths.get(widget_id).cloned();
            let previous_evidence = previous_path
                .as_ref()
                .and_then(|path| self.replacement_evidence_at_path(path));
            let successor_path = current_paths.get(widget_id).cloned();
            let successor_evidence = successor_path
                .as_ref()
                .and_then(|path| successor.replacement_evidence_at_path(path));
            entries.push(WidgetReplacementPlanEntry {
                widget_id: *widget_id,
                previous_path,
                previous_evidence,
                successor_path,
                successor_evidence,
                previous_unique: has_unique_widget_id(previous_widget_order, *widget_id),
                successor_unique: has_unique_widget_id(current_widget_order, *widget_id),
            });
        }

        WidgetReplacementPlan { entries }
    }

    pub(in crate::runtime) fn validate_selected_widget_replacement_plan(
        &self,
        successor: &Self,
        plan: WidgetReplacementPlan,
        previous_paths: &HashMap<WidgetId, WidgetPath>,
        current_paths: &HashMap<WidgetId, WidgetPath>,
    ) -> Result<ValidatedWidgetReplacementPlan, WidgetReplacementPlanVeto> {
        if !plan.entries.iter().all(|entry| {
            entry.previous_unique
                && entry.successor_unique
                && path_evidence_matches(
                    previous_paths.get(&entry.widget_id),
                    entry.previous_path.as_ref(),
                )
                && path_evidence_matches(
                    current_paths.get(&entry.widget_id),
                    entry.successor_path.as_ref(),
                )
                && self.replacement_evidence_matches(
                    entry.previous_path.as_ref(),
                    entry.previous_evidence.as_ref(),
                )
                && successor.replacement_evidence_matches(
                    entry.successor_path.as_ref(),
                    entry.successor_evidence.as_ref(),
                )
        }) {
            return Err(WidgetReplacementPlanVeto::StaleEvidence);
        }
        Ok(ValidatedWidgetReplacementPlan {
            entries: plan.entries,
        })
    }

    pub(in crate::runtime) fn commit_widget_replacements(
        &mut self,
        successor: &Self,
        plan: WidgetReplacementPlan,
        previous_widget_order: &[WidgetId],
        current_widget_order: &[WidgetId],
        previous_paths: &HashMap<WidgetId, WidgetPath>,
        current_paths: &HashMap<WidgetId, WidgetPath>,
    ) -> WidgetReplacementCommitResult<Message> {
        let validated_plan = match self.validate_widget_replacement_plan(
            successor,
            plan,
            previous_widget_order,
            current_widget_order,
            previous_paths,
            current_paths,
        ) {
            Ok(plan) => plan,
            Err(veto) => {
                return WidgetReplacementCommitResult {
                    terminal_messages: Vec::new(),
                    retired_widget_ids: Vec::new(),
                    veto: Some(veto),
                };
            }
        };

        let committed = self.commit_validated_widget_replacements(successor, validated_plan);
        WidgetReplacementCommitResult {
            terminal_messages: committed.terminal_messages,
            retired_widget_ids: committed.retired_widget_ids,
            veto: None,
        }
    }

    /// Validate one complete replacement plan without invoking any widget or
    /// mapper callback.
    pub(in crate::runtime) fn validate_widget_replacement_plan(
        &self,
        successor: &Self,
        plan: WidgetReplacementPlan,
        previous_widget_order: &[WidgetId],
        current_widget_order: &[WidgetId],
        previous_paths: &HashMap<WidgetId, WidgetPath>,
        current_paths: &HashMap<WidgetId, WidgetPath>,
    ) -> Result<ValidatedWidgetReplacementPlan, WidgetReplacementPlanVeto> {
        if !self.replacement_plan_is_current(
            successor,
            &plan,
            previous_widget_order,
            current_widget_order,
            previous_paths,
            current_paths,
        ) {
            return Err(WidgetReplacementPlanVeto::StaleEvidence);
        }

        Ok(ValidatedWidgetReplacementPlan {
            entries: plan.entries,
        })
    }

    /// Consume a validated plan and enter its callback-only commit sequence.
    ///
    /// The validated token proves that no replacement-plan veto remains.  This
    /// method therefore performs no evidence checks and cannot return a veto.
    pub(in crate::runtime) fn commit_validated_widget_replacements(
        &mut self,
        successor: &Self,
        plan: ValidatedWidgetReplacementPlan,
    ) -> ValidatedWidgetReplacementCommitResult<Message> {
        self.commit_widget_replacement_entries(successor, plan.entries)
    }

    /// Commit the current combined replacement evidence without a retained
    /// plan. This is the conservative fallback after a plan-wide veto; it
    /// preserves the original immediate callback, mapper, and retired-ID
    /// behavior without projecting either surface again.
    pub(in crate::runtime) fn commit_widget_replacements_immediately(
        &mut self,
        successor: &Self,
        previous_stateful_widget_order: &[WidgetId],
        previous_widget_order: &[WidgetId],
        current_widget_order: &[WidgetId],
        current_paths: &HashMap<WidgetId, WidgetPath>,
        previous_paths: &HashMap<WidgetId, WidgetPath>,
    ) -> WidgetReplacementCommitResult<Message> {
        let plan = self.plan_widget_replacements(
            successor,
            previous_stateful_widget_order,
            previous_widget_order,
            current_widget_order,
            current_paths,
            previous_paths,
        );
        let committed = self.commit_widget_replacement_entries(successor, plan.entries);
        WidgetReplacementCommitResult {
            terminal_messages: committed.terminal_messages,
            retired_widget_ids: committed.retired_widget_ids,
            veto: None,
        }
    }

    fn commit_widget_replacement_entries(
        &mut self,
        successor: &Self,
        entries: Vec<WidgetReplacementPlanEntry>,
    ) -> ValidatedWidgetReplacementCommitResult<Message> {
        let mut terminal_messages = Vec::with_capacity(entries.len());
        let mut retired_widget_ids = Vec::with_capacity(entries.len());
        for entry in entries {
            let successor_widget = self
                .exact_successor_for_plan_entry(successor, &entry)
                .map(|widget| widget.widget_object());
            let (called, result) = match entry.previous_path.as_ref() {
                Some(previous_path) => {
                    let committed = self.root.commit_widget_replacement_at_path(
                        entry.widget_id,
                        previous_path.as_slice(),
                        successor_widget,
                    );
                    if committed.0 {
                        committed
                    } else {
                        self.root
                            .commit_widget_replacement(entry.widget_id, successor_widget)
                    }
                }
                None => self
                    .root
                    .commit_widget_replacement(entry.widget_id, successor_widget),
            };
            if !called {
                continue;
            }

            if !matches!(result, WidgetDispatchResult::NoOutput) {
                retired_widget_ids.push(entry.widget_id);
            }
            if let WidgetDispatchResult::Message(message) = result {
                terminal_messages.push(message);
            }
        }

        ValidatedWidgetReplacementCommitResult {
            terminal_messages,
            retired_widget_ids,
        }
    }

    fn replacement_plan_is_current(
        &self,
        successor: &Self,
        plan: &WidgetReplacementPlan,
        previous_widget_order: &[WidgetId],
        current_widget_order: &[WidgetId],
        previous_paths: &HashMap<WidgetId, WidgetPath>,
        current_paths: &HashMap<WidgetId, WidgetPath>,
    ) -> bool {
        plan.entries.iter().all(|entry| {
            entry.previous_unique == has_unique_widget_id(previous_widget_order, entry.widget_id)
                && entry.successor_unique
                    == has_unique_widget_id(current_widget_order, entry.widget_id)
                && path_evidence_matches(
                    previous_paths.get(&entry.widget_id),
                    entry.previous_path.as_ref(),
                )
                && path_evidence_matches(
                    current_paths.get(&entry.widget_id),
                    entry.successor_path.as_ref(),
                )
                && self.replacement_evidence_matches(
                    entry.previous_path.as_ref(),
                    entry.previous_evidence.as_ref(),
                )
                && successor.replacement_evidence_matches(
                    entry.successor_path.as_ref(),
                    entry.successor_evidence.as_ref(),
                )
        })
    }

    fn replacement_evidence_matches(
        &self,
        path: Option<&WidgetPath>,
        expected: Option<&WidgetReplacementEvidence>,
    ) -> bool {
        match (path, expected) {
            (None, None) => true,
            (Some(path), Some(expected)) => self
                .replacement_evidence_at_path(path)
                .is_some_and(|actual| actual == *expected),
            (Some(path), None) => self.replacement_evidence_at_path(path).is_none(),
            (None, Some(_)) => false,
        }
    }

    fn exact_successor_for_plan_entry<'a>(
        &self,
        successor: &'a Self,
        entry: &WidgetReplacementPlanEntry,
    ) -> Option<&'a super::SurfaceWidget<Message>> {
        if !entry.previous_unique || !entry.successor_unique {
            return None;
        }
        let previous = entry.previous_evidence.as_ref()?;
        if !previous.valid || previous.widget_id != entry.widget_id {
            return None;
        }
        let current = entry.successor_evidence.as_ref()?;
        if !current.valid
            || current.widget_id != entry.widget_id
            || current.compatibility_kind != previous.compatibility_kind
        {
            return None;
        }
        let successor_path = entry.successor_path.as_ref()?;
        successor.find_widget_at_path(entry.widget_id, successor_path)
    }

    fn replacement_evidence_at_path(&self, path: &WidgetPath) -> Option<WidgetReplacementEvidence> {
        self.root
            .find_widget_at_path(path.as_slice())
            .map(WidgetReplacementEvidence::from_widget)
    }

    pub(in crate::runtime) fn widget_compatibility_at_path(
        &self,
        path: &[usize],
    ) -> Option<(&'static str, bool)> {
        self.root.widget_compatibility_at_path(path)
    }

    #[allow(clippy::too_many_arguments)]
    pub(in crate::runtime) fn synchronize_widget_state_from_paths_with_evidence(
        &mut self,
        previous: &Self,
        stateful_widget_order: &[WidgetId],
        current_paths: &HashMap<WidgetId, WidgetPath>,
        previous_paths: &HashMap<WidgetId, WidgetPath>,
        previous_widget_order: &[WidgetId],
        current_widget_order: &[WidgetId],
        retired_widget_ids: &[WidgetId],
        policy: WidgetStateSyncPolicy,
    ) {
        self.root.synchronize_widget_state_from_paths_with_evidence(
            WidgetStateSyncEvidence {
                stateful_widget_order,
                current_paths,
                previous_paths,
                previous_widget_order,
                current_widget_order,
                retired_widget_ids,
                policy,
            },
            &previous.root,
        );
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

fn path_evidence_matches(actual: Option<&WidgetPath>, expected: Option<&WidgetPath>) -> bool {
    actual == expected
}
