//! Application-owned state for one effect-backed shared resource.
//!
//! Resource values, errors, and presentation progress live here rather than in
//! the shared task broker. The broker supplies only exact operation fences.

use crate::{
    application::{SharedResourceCompletion, SharedResourceOperation, SharedResourceTasks},
    runtime::ResourceKey,
};
use std::sync::{Arc, Weak};

#[cfg(test)]
mod tests;

/// Presentation phase for an application-owned resource.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum ResourcePhase {
    /// No current value or request.
    #[default]
    Idle,
    /// A request is active without retained ready content.
    Pending,
    /// A current ready value is available.
    Ready,
    /// A request is active while prior ready content remains available.
    Refreshing,
    /// The current request failed.
    Failed,
    /// A request was cancelled or became stale without ready content.
    Cancelled,
}

/// Whether starting a new operation retains prior ready content.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum ResourceRefreshPolicy {
    /// Keep a prior ready value while refreshing and after a refresh failure.
    #[default]
    RetainReady,
    /// Clear a prior ready value before the new operation begins.
    DiscardReady,
}

/// Validated bounded progress suitable for generic resource presentation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ResourceProgress(ResourceProgressKind);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ResourceProgressKind {
    /// The operation has progress but no finite total.
    Indeterminate,
    /// A validated finite progress range.
    Determinate { completed: u64, total: u64 },
}

/// Invalid determinate progress input.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ResourceProgressError {
    /// A determinate range must have a positive total.
    ZeroTotal,
    /// Completed work cannot exceed the declared total.
    CompletedExceedsTotal,
}

impl ResourceProgress {
    /// Build validated finite progress.
    pub fn determinate(completed: u64, total: u64) -> Result<Self, ResourceProgressError> {
        if total == 0 {
            return Err(ResourceProgressError::ZeroTotal);
        }
        if completed > total {
            return Err(ResourceProgressError::CompletedExceedsTotal);
        }
        Ok(Self(ResourceProgressKind::Determinate { completed, total }))
    }

    /// Build explicit indeterminate progress.
    pub const fn indeterminate() -> Self {
        Self(ResourceProgressKind::Indeterminate)
    }

    /// Whether the operation has no finite total.
    pub const fn is_indeterminate(self) -> bool {
        matches!(self.0, ResourceProgressKind::Indeterminate)
    }

    /// Completed and total work for finite progress.
    pub const fn determinate_parts(self) -> Option<(u64, u64)> {
        match self.0 {
            ResourceProgressKind::Indeterminate => None,
            ResourceProgressKind::Determinate { completed, total } => Some((completed, total)),
        }
    }
}

/// Reducer failures that leave the current resource state unchanged.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ResourceStateError {
    /// An operation belongs to a different resource key.
    ForeignOperation,
    /// An operation is no longer current in its shared broker.
    StaleOperation,
    /// The resource revision cannot advance without wrapping.
    RevisionExhausted,
    /// The resource generation cannot advance without wrapping.
    GenerationExhausted,
}

struct ResourceIdentity;

struct ResourcePredecessor<T, E> {
    phase: ResourcePhase,
    value: Option<Arc<T>>,
    error: Option<Arc<E>>,
    progress: Option<ResourceProgress>,
    progress_sequence: Option<u64>,
    policy: ResourceRefreshPolicy,
    operation: Option<SharedResourceOperation>,
}

/// Opaque reducer intent to retry one exact resource presentation state.
#[derive(Clone)]
pub struct ResourceRetryIntent {
    key: ResourceKey,
    revision: u64,
    generation: u64,
    identity: Weak<ResourceIdentity>,
}

/// Opaque reducer intent to cancel one exact running resource operation.
#[derive(Clone)]
pub struct ResourceCancelIntent {
    key: ResourceKey,
    revision: u64,
    generation: u64,
    identity: Weak<ResourceIdentity>,
    operation: SharedResourceOperation,
}

/// Immutable owned data for a projected resource branch.
///
/// Cloning a snapshot clones only its `Arc` payloads and never requires `T` or
/// `E` to implement `Clone`.
pub struct ResourceSnapshot<T, E> {
    key: ResourceKey,
    revision: u64,
    generation: u64,
    phase: ResourcePhase,
    value: Option<Arc<T>>,
    error: Option<Arc<E>>,
    progress: Option<ResourceProgress>,
    operation: Option<SharedResourceOperation>,
    identity: Weak<ResourceIdentity>,
}

impl<T, E> Clone for ResourceSnapshot<T, E> {
    fn clone(&self) -> Self {
        Self {
            key: self.key.clone(),
            revision: self.revision,
            generation: self.generation,
            phase: self.phase,
            value: self.value.as_ref().map(Arc::clone),
            error: self.error.as_ref().map(Arc::clone),
            progress: self.progress,
            operation: self.operation.clone(),
            identity: self.identity.clone(),
        }
    }
}

impl<T, E> ResourceSnapshot<T, E> {
    /// Stable application resource identity.
    pub fn key(&self) -> &ResourceKey {
        &self.key
    }

    /// Monotonic reducer revision.
    pub const fn revision(&self) -> u64 {
        self.revision
    }

    /// Monotonic local generation for started, superseded, or rekeyed work.
    pub const fn generation(&self) -> u64 {
        self.generation
    }

    /// Effective pure presentation phase.
    pub const fn phase(&self) -> ResourcePhase {
        self.phase
    }

    /// Current or retained ready value.
    pub fn value(&self) -> Option<&Arc<T>> {
        self.value.as_ref()
    }

    /// Current typed failure detail.
    pub fn error(&self) -> Option<&Arc<E>> {
        self.error.as_ref()
    }

    /// Current validated progress.
    pub const fn progress(&self) -> Option<ResourceProgress> {
        self.progress
    }

    /// Build an exact retry intent for a phase with no running operation.
    pub fn retry_intent(&self) -> Option<ResourceRetryIntent> {
        matches!(
            self.phase,
            ResourcePhase::Idle | ResourcePhase::Failed | ResourcePhase::Cancelled
        )
        .then(|| ResourceRetryIntent {
            key: self.key.clone(),
            revision: self.revision,
            generation: self.generation,
            identity: self.identity.clone(),
        })
    }

    /// Build an exact cancellation intent for the current running operation.
    pub fn cancel_intent(&self) -> Option<ResourceCancelIntent> {
        matches!(
            self.phase,
            ResourcePhase::Pending | ResourcePhase::Refreshing
        )
        .then(|| self.operation.as_ref())
        .flatten()
        .map(|operation| ResourceCancelIntent {
            key: self.key.clone(),
            revision: self.revision,
            generation: self.generation,
            identity: self.identity.clone(),
            operation: operation.clone(),
        })
    }
}

/// Application-owned resource state paired with exact shared-operation fences.
pub struct Resource<T, E> {
    identity: Arc<ResourceIdentity>,
    key: ResourceKey,
    revision: u64,
    generation: u64,
    phase: ResourcePhase,
    value: Option<Arc<T>>,
    error: Option<Arc<E>>,
    progress: Option<ResourceProgress>,
    progress_sequence: Option<u64>,
    policy: ResourceRefreshPolicy,
    operation: Option<SharedResourceOperation>,
    predecessor: Option<ResourcePredecessor<T, E>>,
}

impl<T, E> Resource<T, E> {
    /// Build an idle resource with no retained value or operation.
    pub fn new(key: impl Into<ResourceKey>) -> Self {
        Self {
            identity: Arc::new(ResourceIdentity),
            key: key.into(),
            revision: 0,
            generation: 0,
            phase: ResourcePhase::Idle,
            value: None,
            error: None,
            progress: None,
            progress_sequence: None,
            policy: ResourceRefreshPolicy::RetainReady,
            operation: None,
            predecessor: None,
        }
    }

    /// Stable resource identity.
    pub fn key(&self) -> &ResourceKey {
        &self.key
    }

    /// Monotonic reducer revision.
    pub const fn revision(&self) -> u64 {
        self.revision
    }

    /// Monotonic application generation.
    pub const fn generation(&self) -> u64 {
        self.generation
    }

    /// Stored phase before pure stale-operation presentation adjustment.
    pub const fn phase(&self) -> ResourcePhase {
        self.phase
    }

    /// Retained ready value, if any.
    pub fn value(&self) -> Option<&Arc<T>> {
        self.value.as_ref()
    }

    /// Current typed error, if any.
    pub fn error(&self) -> Option<&Arc<E>> {
        self.error.as_ref()
    }

    /// Current validated progress, if any.
    pub const fn progress(&self) -> Option<ResourceProgress> {
        self.progress
    }

    /// Start or attach one exact current operation.
    ///
    /// Repeating the same exact operation is idempotent. A different current
    /// operation advances both local fences without wrapping.
    pub fn begin(
        &mut self,
        operation: SharedResourceOperation,
        policy: ResourceRefreshPolicy,
    ) -> Result<bool, ResourceStateError> {
        self.restore_predecessor();
        if operation.key() != &self.key {
            return Err(ResourceStateError::ForeignOperation);
        }
        if !operation.is_current() {
            return Err(ResourceStateError::StaleOperation);
        }
        if self
            .operation
            .as_ref()
            .is_some_and(|current| current.same_operation(&operation))
        {
            return Ok(false);
        }
        let predecessor = Some(ResourcePredecessor {
            phase: self.phase,
            value: self.value.clone(),
            error: self.error.clone(),
            progress: self.progress,
            progress_sequence: self.progress_sequence,
            policy: self.policy,
            operation: self.operation.clone(),
        });
        self.advance_fences()?;
        if policy == ResourceRefreshPolicy::DiscardReady {
            self.value = None;
        }
        self.phase = if self.value.is_some() {
            ResourcePhase::Refreshing
        } else {
            ResourcePhase::Pending
        };
        self.error = None;
        self.progress = None;
        self.progress_sequence = None;
        self.policy = policy;
        self.operation = Some(operation);
        self.predecessor = predecessor;
        Ok(true)
    }

    /// Apply a fenced successful or failed worker completion in the reducer.
    pub fn apply_completion(
        &mut self,
        tasks: &SharedResourceTasks,
        completion: SharedResourceCompletion<Result<T, E>>,
    ) -> bool {
        self.apply_completion_inner(tasks, completion, None)
    }

    /// Apply a completion and, on failure, schedule one explicit broker retry.
    ///
    /// This records the typed failure locally but never starts a timer or worker.
    /// A caller later takes the due retry and explicitly begins its next operation.
    pub fn apply_completion_with_retry(
        &mut self,
        tasks: &SharedResourceTasks,
        completion: SharedResourceCompletion<Result<T, E>>,
        deadline: u64,
    ) -> bool {
        self.apply_completion_inner(tasks, completion, Some(deadline))
    }

    fn apply_completion_inner(
        &mut self,
        tasks: &SharedResourceTasks,
        completion: SharedResourceCompletion<Result<T, E>>,
        retry_deadline: Option<u64>,
    ) -> bool {
        self.restore_predecessor();
        let Some(operation) = self.operation.as_ref() else {
            return false;
        };
        if !operation.matches_completion(&completion) || !operation.is_current() {
            return false;
        }
        if !self.can_bump_revision() {
            return false;
        }
        if completion.output.is_err()
            && retry_deadline.is_some_and(|deadline| !tasks.schedule_retry(&completion, deadline))
        {
            return false;
        }
        match completion.output {
            Ok(value) => {
                let completion = SharedResourceCompletion {
                    output: value,
                    current: completion.current,
                };
                let Some(value) = tasks.finish_ready(completion) else {
                    return false;
                };
                self.value = Some(Arc::new(value));
                self.error = None;
                self.progress = None;
                self.progress_sequence = None;
                self.operation = None;
                self.phase = ResourcePhase::Ready;
            }
            Err(error) => {
                let error = if retry_deadline.is_some() {
                    error
                } else {
                    let completion = SharedResourceCompletion {
                        output: error,
                        current: completion.current,
                    };
                    let Some(error) = tasks.finish_failed(completion) else {
                        return false;
                    };
                    error
                };
                if self.policy == ResourceRefreshPolicy::DiscardReady {
                    self.value = None;
                }
                self.error = Some(Arc::new(error));
                self.progress = None;
                self.progress_sequence = None;
                self.operation = None;
                self.phase = ResourcePhase::Failed;
            }
        }
        self.bump_revision();
        self.predecessor = None;
        true
    }

    /// Apply one strictly newer progress update for the installed operation.
    pub fn apply_progress(
        &mut self,
        operation: &SharedResourceOperation,
        sequence: u64,
        progress: ResourceProgress,
    ) -> bool {
        self.restore_predecessor();
        let Some(installed) = self.operation.as_ref() else {
            return false;
        };
        if !installed.same_operation(operation)
            || !installed.is_current()
            || !self.can_bump_revision()
            || self
                .progress_sequence
                .is_some_and(|previous| sequence <= previous)
        {
            return false;
        }
        self.progress = Some(progress);
        self.progress_sequence = Some(sequence);
        self.bump_revision();
        true
    }

    /// Cancel the exact installed operation while preserving valid ready data.
    pub fn cancel(&mut self, tasks: &SharedResourceTasks) -> bool {
        self.restore_predecessor();
        let Some(operation) = self.operation.as_ref() else {
            return false;
        };
        if !operation.is_current()
            || !self.can_bump_revision()
            || !tasks.cancel_operation(operation)
        {
            return false;
        }
        self.operation = None;
        self.error = None;
        self.progress = None;
        self.progress_sequence = None;
        self.phase = if self.value.is_some() {
            ResourcePhase::Ready
        } else {
            ResourcePhase::Cancelled
        };
        self.bump_revision();
        self.predecessor = None;
        true
    }

    /// Consume an exact retry intent without admitting or starting work.
    ///
    /// The caller explicitly constructs a worker effect and calls [`Self::begin`]
    /// after this succeeds. Reusing an old or duplicate intent is rejected.
    pub fn take_retry(&mut self, intent: &ResourceRetryIntent) -> bool {
        self.restore_predecessor();
        if intent.key != self.key
            || intent.revision != self.revision
            || intent.generation != self.generation
            || !self.matches_identity(&intent.identity)
            || !matches!(
                self.snapshot().phase(),
                ResourcePhase::Idle | ResourcePhase::Failed | ResourcePhase::Cancelled
            )
            || !self.can_bump_revision()
        {
            return false;
        }
        self.bump_revision();
        true
    }

    /// Consume an exact cancellation intent and cancel only its installed operation.
    pub fn cancel_intent(
        &mut self,
        tasks: &SharedResourceTasks,
        intent: &ResourceCancelIntent,
    ) -> bool {
        self.restore_predecessor();
        let Some(operation) = self.operation.as_ref() else {
            return false;
        };
        if intent.key != self.key
            || intent.revision != self.revision
            || intent.generation != self.generation
            || !self.matches_identity(&intent.identity)
            || !operation.same_operation(&intent.operation)
            || !matches!(
                self.snapshot().phase(),
                ResourcePhase::Pending | ResourcePhase::Refreshing
            )
            || !self.can_bump_revision()
            || !tasks.cancel_operation(operation)
        {
            return false;
        }
        self.operation = None;
        self.error = None;
        self.progress = None;
        self.progress_sequence = None;
        self.phase = if self.value.is_some() {
            ResourcePhase::Ready
        } else {
            ResourcePhase::Cancelled
        };
        self.bump_revision();
        self.predecessor = None;
        true
    }

    /// Change resource identity and detach its local presentation state.
    ///
    /// Rekeying does not cancel the old shared operation because other
    /// projected consumers can still own its demand.
    pub fn rekey(&mut self, key: impl Into<ResourceKey>) -> Result<bool, ResourceStateError> {
        let key = key.into();
        if key == self.key {
            return Ok(false);
        }
        self.advance_fences()?;
        self.key = key;
        self.phase = ResourcePhase::Idle;
        self.value = None;
        self.error = None;
        self.progress = None;
        self.progress_sequence = None;
        self.operation = None;
        self.predecessor = None;
        Ok(true)
    }

    /// Build an owned pure presentation snapshot without starting work.
    pub fn snapshot(&self) -> ResourceSnapshot<T, E> {
        let primary_current = self
            .operation
            .as_ref()
            .is_some_and(SharedResourceOperation::is_current);
        let predecessor = (!primary_current)
            .then(|| {
                self.predecessor.as_ref().filter(|state| {
                    self.operation
                        .as_ref()
                        .is_some_and(SharedResourceOperation::was_rejected)
                        || state
                            .operation
                            .as_ref()
                            .is_some_and(SharedResourceOperation::is_current)
                })
            })
            .flatten();
        let (stored_phase, value, error, progress, operation_current, operation) =
            if let Some(state) = predecessor {
                (
                    state.phase,
                    &state.value,
                    &state.error,
                    state.progress,
                    state
                        .operation
                        .as_ref()
                        .is_some_and(SharedResourceOperation::is_current),
                    state.operation.as_ref(),
                )
            } else {
                (
                    self.phase,
                    &self.value,
                    &self.error,
                    self.progress,
                    primary_current,
                    self.operation.as_ref(),
                )
            };
        let phase = match (stored_phase, operation_current, value.is_some()) {
            (ResourcePhase::Pending | ResourcePhase::Refreshing, false, true) => {
                ResourcePhase::Ready
            }
            (ResourcePhase::Pending | ResourcePhase::Refreshing, false, false) => {
                ResourcePhase::Cancelled
            }
            (phase, _, _) => phase,
        };
        ResourceSnapshot {
            key: self.key.clone(),
            revision: self.revision,
            generation: self.generation,
            phase,
            value: value.as_ref().map(Arc::clone),
            error: error.as_ref().map(Arc::clone),
            progress: operation_current.then_some(progress).flatten(),
            operation: operation_current.then(|| operation.cloned()).flatten(),
            identity: Arc::downgrade(&self.identity),
        }
    }

    fn advance_fences(&mut self) -> Result<(), ResourceStateError> {
        let revision = self
            .revision
            .checked_add(1)
            .ok_or(ResourceStateError::RevisionExhausted)?;
        let generation = self
            .generation
            .checked_add(1)
            .ok_or(ResourceStateError::GenerationExhausted)?;
        self.revision = revision;
        self.generation = generation;
        Ok(())
    }

    fn can_bump_revision(&self) -> bool {
        self.revision != u64::MAX
    }

    fn restore_predecessor(&mut self) {
        if self
            .operation
            .as_ref()
            .is_some_and(SharedResourceOperation::is_current)
        {
            return;
        }
        let Some(predecessor) = self.predecessor.take() else {
            return;
        };
        let replacement_rejected = self
            .operation
            .as_ref()
            .is_some_and(SharedResourceOperation::was_rejected);
        if !predecessor
            .operation
            .as_ref()
            .is_some_and(SharedResourceOperation::is_current)
            && !replacement_rejected
        {
            self.predecessor = Some(predecessor);
            return;
        }
        self.phase = predecessor.phase;
        self.value = predecessor.value;
        self.error = predecessor.error;
        self.progress = predecessor.progress;
        self.progress_sequence = predecessor.progress_sequence;
        self.policy = predecessor.policy;
        self.operation = predecessor.operation;
    }

    fn matches_identity(&self, identity: &Weak<ResourceIdentity>) -> bool {
        identity
            .upgrade()
            .is_some_and(|identity| Arc::ptr_eq(&self.identity, &identity))
    }

    fn bump_revision(&mut self) {
        self.revision = self
            .revision
            .checked_add(1)
            .expect("checked before resource state mutation");
    }
}

impl<T, E> Default for Resource<T, E> {
    fn default() -> Self {
        Self::new("default")
    }
}
