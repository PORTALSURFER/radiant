use super::{UiSurface, WidgetDispatchResult, WidgetPath};
use crate::widgets::{WidgetId, WidgetRevision};
use std::collections::{HashMap, HashSet};

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

pub(in crate::runtime) struct WidgetStateSyncEvidence<'a> {
    pub(in crate::runtime::surface) stateful_widget_order: &'a [WidgetId],
    pub(in crate::runtime::surface) current_paths: &'a HashMap<WidgetId, WidgetPath>,
    pub(in crate::runtime::surface) previous_paths: &'a HashMap<WidgetId, WidgetPath>,
    pub(in crate::runtime::surface) previous_widget_order: &'a [WidgetId],
    pub(in crate::runtime::surface) current_widget_order: &'a [WidgetId],
    pub(in crate::runtime::surface) retired_widget_ids: &'a [WidgetId],
    pub(in crate::runtime::surface) policy: WidgetStateSyncPolicy,
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
    pub(in crate::runtime) fn plan_widget_replacements(
        &self,
        successor: &Self,
        previous_stateful_widget_order: &[WidgetId],
        previous_widget_order: &[WidgetId],
        current_widget_order: &[WidgetId],
        current_paths: &HashMap<WidgetId, WidgetPath>,
        previous_paths: &HashMap<WidgetId, WidgetPath>,
    ) -> WidgetReplacementPlan {
        let mut entries = Vec::with_capacity(previous_stateful_widget_order.len());
        let mut visited = HashSet::with_capacity(previous_stateful_widget_order.len());

        for widget_id in previous_stateful_widget_order {
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

    pub(in crate::runtime) fn commit_widget_replacements(
        &mut self,
        successor: &Self,
        plan: WidgetReplacementPlan,
        previous_widget_order: &[WidgetId],
        current_widget_order: &[WidgetId],
        previous_paths: &HashMap<WidgetId, WidgetPath>,
        current_paths: &HashMap<WidgetId, WidgetPath>,
    ) -> WidgetReplacementCommitResult<Message> {
        if !self.replacement_plan_is_current(
            successor,
            &plan,
            previous_widget_order,
            current_widget_order,
            previous_paths,
            current_paths,
        ) {
            return WidgetReplacementCommitResult {
                terminal_messages: Vec::new(),
                retired_widget_ids: Vec::new(),
                veto: Some(WidgetReplacementPlanVeto::StaleEvidence),
            };
        }

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
        self.commit_widget_replacement_entries(successor, plan.entries)
    }

    fn commit_widget_replacement_entries(
        &mut self,
        successor: &Self,
        entries: Vec<WidgetReplacementPlanEntry>,
    ) -> WidgetReplacementCommitResult<Message> {
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

        WidgetReplacementCommitResult {
            terminal_messages,
            retired_widget_ids,
            veto: None,
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
