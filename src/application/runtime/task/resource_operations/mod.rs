//! Private latest-operation registry layered over resource interests.
//!
//! This registry retains no resource values or errors. It only owns bounded
//! latest-task fences so UI reducers can reject stale operation completions.

use super::resource_interests::{
    ResourceInterestAdmissionError, ResourceInterestLedger, ResourceInterestMetadata,
};
use super::{LatestTask, LatestTaskTransaction, LatestTaskTransactionSettlement, TaskTicket};
use crate::{application::CancellationToken, runtime::ResourceKey};
use std::{
    collections::HashMap,
    sync::{
        Arc, Mutex, Weak,
        atomic::{AtomicU8, Ordering},
    },
};

#[cfg(test)]
mod tests;

const MAX_SLOTS: usize = 256;

#[derive(Clone)]
pub(crate) struct ResourceOperationRegistry {
    state: Arc<Mutex<State>>,
    // This marker identifies one broker without allowing a completion to retain
    // its mutable state.
    identity: Arc<RegistryIdentity>,
}

#[derive(Debug)]
struct RegistryIdentity;

impl ResourceOperationRegistry {
    pub(crate) fn with_ledger(ledger: ResourceInterestLedger) -> Self {
        Self {
            state: Arc::new(Mutex::new(State::new(ledger, MAX_SLOTS))),
            identity: Arc::new(RegistryIdentity),
        }
    }

    #[cfg(test)]
    pub(crate) fn with_test_limit(ledger: ResourceInterestLedger, limit: usize) -> Self {
        assert!(limit > 0);
        Self {
            state: Arc::new(Mutex::new(State::new(ledger, limit))),
            identity: Arc::new(RegistryIdentity),
        }
    }

    /// Reserve work for a live ledger demand. A joined, ready, or backoff
    /// result never publishes a new task; callers only start work for
    /// `Reserved`.
    pub(crate) fn reserve(
        &self,
        key: ResourceKey,
        mode: ResourceOperationReplaceMode,
    ) -> Result<ResourceOperationReserve, ResourceOperationAdmissionError> {
        let mut state = lock(&self.state);
        state.prune();
        if state.closed {
            return Err(ResourceOperationAdmissionError::Closed);
        }
        let Some(demand_generation) = state.ledger.demand_generation(&key) else {
            return Err(ResourceOperationAdmissionError::NoLiveInterest);
        };
        if state.slots.get(&key).is_some_and(|slot| {
            slot.demand_generation != demand_generation
                && slot.keep_ready
                && slot.pending.is_none()
                && matches!(slot.phase, Phase::Ready)
                && mode == ResourceOperationReplaceMode::Join
        }) {
            // Retained readiness has no completion value in this broker, but it
            // is reusable by a new live demand. Advance the fence so every old
            // completion remains stale.
            let epoch = state.allocate_epoch()?;
            let Some(slot) = state.slots.get_mut(&key) else {
                return Err(ResourceOperationAdmissionError::NoLiveInterest);
            };
            slot.epoch = epoch;
            slot.demand_generation = demand_generation;
            return Ok(ResourceOperationReserve::Ready);
        }
        if let Some(slot) = state.slots.get(&key) {
            if slot.demand_generation == demand_generation {
                if slot.pending.is_some() {
                    return match mode {
                        ResourceOperationReplaceMode::Join => Ok(ResourceOperationReserve::Joined),
                        ResourceOperationReplaceMode::Replace => {
                            Err(ResourceOperationAdmissionError::PendingAdmission)
                        }
                    };
                }
                if mode == ResourceOperationReplaceMode::Join {
                    match slot.phase {
                        Phase::Running { .. } => return Ok(ResourceOperationReserve::Joined),
                        Phase::Ready => return Ok(ResourceOperationReserve::Ready),
                        Phase::Backoff { .. } => return Ok(ResourceOperationReserve::Backoff),
                        Phase::Idle => {}
                    }
                }
            }
        } else if state.slots.len() >= state.limit {
            return Err(ResourceOperationAdmissionError::Capacity);
        }

        let epoch = state.allocate_epoch()?;
        let keep_ready = state.ledger.keeps_ready(&key);
        let slot = state.slots.entry(key.clone()).or_default();
        slot.keep_ready = keep_ready;
        // A final release cancels any stale transaction before a later demand
        // can reuse the retained ready slot. Its eventual settlement cannot
        // match this replacement.
        if slot.demand_generation != demand_generation {
            slot.pending = None;
            slot.settlement = None;
        }
        let previous = PreviousSlot {
            phase: slot.phase,
            epoch: slot.epoch,
            demand_generation: slot.demand_generation,
            cancellation: slot.cancellation.clone(),
            cancel_requested: slot.cancel_requested,
            settlement: slot.settlement.clone(),
        };
        // The replacement owns its own token. The predecessor's token stays
        // with its rollback snapshot until this transaction settles.
        slot.cancellation = None;
        slot.cancel_requested = false;
        let transaction = slot.latest.begin_replacement();
        let ticket = transaction.replacement();
        let effect_id = slot.latest.effect_id();
        let settlement = Arc::new(OperationSettlement::pending());
        slot.epoch = epoch;
        slot.demand_generation = demand_generation;
        slot.phase = Phase::Running { ticket, effect_id };
        slot.settlement = Some(Arc::clone(&settlement));
        slot.pending = Some(PendingOperation {
            ticket,
            epoch,
            demand_generation,
            previous,
        });
        let state_weak = Arc::downgrade(&self.state);
        let hook_key = key.clone();
        let hook_settlement = Arc::clone(&settlement);
        let transaction = transaction.with_settlement_hook(Arc::new(move |settlement| {
            hook_settlement.set(settlement);
            settle_pending(
                &state_weak,
                &hook_key,
                ticket,
                epoch,
                demand_generation,
                settlement,
            );
        }));
        let current = ResourceOperationCurrent {
            key: key.clone(),
            ticket: Some(ticket),
            effect_id: Some(effect_id),
            operation_epoch: epoch,
            demand_generation,
            identity: Arc::downgrade(&self.identity),
            settlement,
        };
        Ok(ResourceOperationReserve::Reserved(
            ResourceOperationReservation {
                key,
                ticket,
                effect_id,
                operation_epoch: epoch,
                demand_generation,
                transaction,
                current: current_probe(
                    Arc::downgrade(&self.state),
                    current.key.clone(),
                    ticket,
                    epoch,
                    demand_generation,
                ),
                identity: current.identity.clone(),
                settlement: Arc::clone(&current.settlement),
                state: Arc::downgrade(&self.state),
            },
        ))
    }

    /// Snapshot only a live running operation. The returned currentness probe
    /// holds weak state and never starts, retains, or revives work.
    pub(crate) fn currentness_probe(
        &self,
        current: &ResourceOperationCurrent,
    ) -> Arc<dyn Fn() -> bool + Send + Sync + 'static> {
        let Some(ticket) = current.ticket else {
            return Arc::new(|| false);
        };
        current_probe(
            Arc::downgrade(&self.state),
            current.key.clone(),
            ticket,
            current.operation_epoch,
            current.demand_generation,
        )
    }

    pub(crate) fn current_operation(&self, key: &ResourceKey) -> Option<ResourceOperationCurrent> {
        let mut state = lock(&self.state);
        state.prune();
        let demand_generation = state.ledger.demand_generation(key)?;
        let slot = state.slots.get(key)?;
        let Phase::Running { ticket, effect_id } = slot.phase else {
            return None;
        };
        let settlement = slot.settlement.clone()?;
        (slot.demand_generation == demand_generation
            && !slot.is_cancelled()
            && slot.latest.is_active(ticket))
        .then(|| ResourceOperationCurrent {
            key: key.clone(),
            ticket: Some(ticket),
            effect_id: Some(effect_id),
            operation_epoch: slot.epoch,
            demand_generation,
            identity: Arc::downgrade(&self.identity),
            settlement,
        })
    }

    /// Accept a UI-reducer completion only after the host effect fences have
    /// accepted its transaction. This does not invoke application callbacks.
    pub(crate) fn finish_ready(&self, current: ResourceOperationCurrent) -> bool {
        self.finish(current, true)
    }

    pub(crate) fn finish_idle(&self, current: ResourceOperationCurrent) -> bool {
        self.finish(current, false)
    }

    fn finish(&self, current: ResourceOperationCurrent, ready: bool) -> bool {
        if !current.belongs_to(&self.identity) {
            return false;
        }
        let Some(ticket) = current.ticket else {
            return false;
        };
        let mut state = lock(&self.state);
        state.prune();
        if state.ledger.demand_generation(&current.key) != Some(current.demand_generation) {
            return false;
        }
        let Some(slot) = state.slots.get_mut(&current.key) else {
            return false;
        };
        if !matches!(slot.phase, Phase::Running { ticket: active, effect_id }
            if active == ticket && current.effect_id == Some(effect_id))
            || slot.epoch != current.operation_epoch
            || slot.demand_generation != current.demand_generation
            || slot.pending.is_some()
            || !slot.latest.is_active(ticket)
        {
            return false;
        }
        if !slot.latest.finish(ticket) {
            return false;
        }
        slot.phase = if ready { Phase::Ready } else { Phase::Idle };
        slot.settlement = None;
        slot.cancellation = None;
        slot.cancel_requested = false;
        true
    }

    /// Schedule at most one logical retry for the exact failed operation.
    pub(crate) fn schedule_retry(&self, current: ResourceOperationCurrent, deadline: u64) -> bool {
        if !current.belongs_to(&self.identity) {
            return false;
        }
        let Some(ticket) = current.ticket else {
            return false;
        };
        let mut state = lock(&self.state);
        state.prune();
        if state.ledger.demand_generation(&current.key) != Some(current.demand_generation) {
            return false;
        }
        let Some(slot) = state.slots.get_mut(&current.key) else {
            return false;
        };
        if !matches!(slot.phase, Phase::Running { ticket: active, effect_id }
            if active == ticket && current.effect_id == Some(effect_id))
            || slot.epoch != current.operation_epoch
            || slot.demand_generation != current.demand_generation
            || slot.pending.is_some()
            || !slot.latest.is_active(ticket)
        {
            return false;
        }
        if !slot.latest.finish(ticket) {
            return false;
        }
        slot.phase = Phase::Backoff { deadline };
        slot.settlement = None;
        slot.cancellation = None;
        slot.cancel_requested = false;
        true
    }

    /// Consume one due retry while holding the registry lock. The subsequent
    /// join cannot launch duplicate work: a concurrent explicit reservation
    /// becomes the one joined by this retry.
    pub(crate) fn take_retry(
        &self,
        key: &ResourceKey,
        now: u64,
    ) -> Option<ResourceOperationReserve> {
        {
            let mut state = lock(&self.state);
            state.prune();
            let demand_generation = state.ledger.demand_generation(key)?;
            let slot = state.slots.get_mut(key)?;
            if slot.demand_generation != demand_generation
                || slot.pending.is_some()
                || !matches!(slot.phase, Phase::Backoff { deadline } if deadline <= now)
            {
                return None;
            }
            slot.phase = Phase::Idle;
        }
        self.reserve(key.clone(), ResourceOperationReplaceMode::Join)
            .ok()
    }

    /// Preserve ready bookkeeping for an explicitly retained key. Ledger
    /// metadata is updated first so retained operation slots consume the same
    /// bounded resource budget as other retained keys.
    pub(crate) fn set_keep_ready(
        &self,
        key: ResourceKey,
        keep_ready: bool,
    ) -> Result<(), ResourceOperationAdmissionError> {
        let ledger = {
            let state = lock(&self.state);
            if state.closed {
                return Err(ResourceOperationAdmissionError::Closed);
            }
            state.ledger.clone()
        };
        ledger
            .set_metadata(key.clone(), ResourceInterestMetadata { keep_ready })
            .map_err(ResourceOperationAdmissionError::Interest)?;
        let mut state = lock(&self.state);
        if state.closed {
            return Err(ResourceOperationAdmissionError::Closed);
        }
        if let Some(slot) = state.slots.get_mut(&key) {
            slot.keep_ready = keep_ready;
        }
        state.prune();
        Ok(())
    }

    /// Cancel the current operation for this key. An unsettled replacement
    /// remains pending until its transaction settles, so cancellation cannot
    /// open another predecessor chain.
    pub(crate) fn cancel(&self, key: &ResourceKey) -> bool {
        let mut state = lock(&self.state);
        let Some(slot) = state.slots.get_mut(key) else {
            return false;
        };
        cancel_slot(slot);
        true
    }

    /// Cancel one exact snapshot under the operation mutex. A stale or foreign
    /// snapshot cannot cancel a newer operation that reused the same key.
    pub(crate) fn cancel_current(&self, current: &ResourceOperationCurrent) -> bool {
        if !current.belongs_to(&self.identity) {
            return false;
        }
        let Some(ticket) = current.ticket else {
            return false;
        };
        let mut state = lock(&self.state);
        state.prune();
        if state.ledger.demand_generation(&current.key) != Some(current.demand_generation) {
            return false;
        }
        let Some(slot) = state.slots.get_mut(&current.key) else {
            return false;
        };
        if slot.epoch != current.operation_epoch
            || slot.demand_generation != current.demand_generation
            || !matches!(slot.phase, Phase::Running { ticket: active, effect_id }
                if active == ticket && current.effect_id == Some(effect_id))
            || !slot.latest.is_active(ticket)
        {
            return false;
        }
        cancel_slot(slot);
        true
    }

    pub(crate) fn shutdown(&self) {
        let mut state = lock(&self.state);
        state.closed = true;
        for slot in state.slots.values_mut() {
            slot.latest.cancel();
            slot.pending = None;
            slot.settlement = None;
        }
        state.slots.clear();
    }

    #[cfg(test)]
    pub(crate) fn slot_count(&self) -> usize {
        self.cleanup();
        lock(&self.state).slots.len()
    }

    #[cfg(test)]
    fn rollback_entry_count(&self, key: &ResourceKey) -> usize {
        lock(&self.state)
            .slots
            .get(key)
            .map_or(0, |slot| slot.latest.rollback_entry_count())
    }

    #[cfg(test)]
    fn slot_fence(&self, key: &ResourceKey) -> Option<(u64, u64)> {
        lock(&self.state)
            .slots
            .get(key)
            .map(|slot| (slot.epoch, slot.demand_generation))
    }

    #[cfg(test)]
    fn backoff_deadline(&self, key: &ResourceKey) -> Option<u64> {
        lock(&self.state)
            .slots
            .get(key)
            .and_then(|slot| match slot.phase {
                Phase::Backoff { deadline } => Some(deadline),
                Phase::Idle | Phase::Running { .. } | Phase::Ready => None,
            })
    }

    #[cfg(test)]
    fn cleanup(&self) {
        lock(&self.state).prune();
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ResourceOperationReplaceMode {
    Join,
    Replace,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ResourceOperationAdmissionError {
    Closed,
    Capacity,
    EpochExhausted,
    NoLiveInterest,
    PendingAdmission,
    Interest(ResourceInterestAdmissionError),
}

pub(crate) enum ResourceOperationReserve {
    Joined,
    Ready,
    Backoff,
    Reserved(ResourceOperationReservation),
}

pub(crate) struct ResourceOperationReservation {
    key: ResourceKey,
    ticket: TaskTicket,
    effect_id: u64,
    operation_epoch: u64,
    demand_generation: u64,
    transaction: LatestTaskTransaction,
    current: Arc<dyn Fn() -> bool + Send + Sync + 'static>,
    identity: Weak<RegistryIdentity>,
    settlement: Arc<OperationSettlement>,
    state: Weak<Mutex<State>>,
}

impl ResourceOperationReservation {
    pub(crate) fn current(&self) -> ResourceOperationCurrent {
        ResourceOperationCurrent {
            key: self.key.clone(),
            ticket: Some(self.ticket),
            effect_id: Some(self.effect_id),
            operation_epoch: self.operation_epoch,
            demand_generation: self.demand_generation,
            identity: self.identity.clone(),
            settlement: Arc::clone(&self.settlement),
        }
    }
    pub(crate) fn ticket(&self) -> TaskTicket {
        self.ticket
    }
    pub(crate) fn effect_id(&self) -> u64 {
        self.effect_id
    }
    #[cfg(test)]
    pub(crate) fn transaction(&self) -> &LatestTaskTransaction {
        &self.transaction
    }
    pub(crate) fn into_transaction(self) -> LatestTaskTransaction {
        self.transaction
    }
    /// Bind the host worker token before the reservation transaction is
    /// consumed. A cancelled token immediately makes its completion stale.
    pub(crate) fn attach_cancellation(&self, token: CancellationToken) {
        let Some(state) = self.state.upgrade() else {
            return;
        };
        let mut state = lock(&state);
        let Some(slot) = state.slots.get_mut(&self.key) else {
            return;
        };
        if slot.epoch != self.operation_epoch
            || slot.demand_generation != self.demand_generation
            || !matches!(slot.phase, Phase::Running { ticket, effect_id }
                if ticket == self.ticket && effect_id == self.effect_id)
        {
            return;
        }
        if slot.cancel_requested {
            token.cancel();
        }
        slot.cancellation = Some(token);
    }

    pub(crate) fn currentness_probe(&self) -> Arc<dyn Fn() -> bool + Send + Sync + 'static> {
        Arc::clone(&self.current)
    }
}

#[derive(Clone, Debug)]
pub(crate) struct ResourceOperationCurrent {
    key: ResourceKey,
    ticket: Option<TaskTicket>,
    effect_id: Option<u64>,
    operation_epoch: u64,
    demand_generation: u64,
    identity: Weak<RegistryIdentity>,
    settlement: Arc<OperationSettlement>,
}

impl ResourceOperationCurrent {
    pub(crate) fn key(&self) -> &ResourceKey {
        &self.key
    }
    pub(crate) fn operation_epoch(&self) -> u64 {
        self.operation_epoch
    }
    #[cfg(test)]
    pub(crate) fn demand_generation(&self) -> u64 {
        self.demand_generation
    }

    pub(super) fn matches(&self, other: &Self) -> bool {
        self.key == other.key
            && self.ticket == other.ticket
            && self.effect_id == other.effect_id
            && self.operation_epoch == other.operation_epoch
            && self.demand_generation == other.demand_generation
            && self.identity.upgrade().is_some_and(|identity| {
                other
                    .identity
                    .upgrade()
                    .is_some_and(|other| Arc::ptr_eq(&identity, &other))
            })
    }

    pub(super) fn was_rejected(&self) -> bool {
        self.settlement.was_rejected()
    }

    fn belongs_to(&self, identity: &Arc<RegistryIdentity>) -> bool {
        self.identity
            .upgrade()
            .is_some_and(|current| Arc::ptr_eq(&current, identity))
    }
}

struct State {
    ledger: ResourceInterestLedger,
    slots: HashMap<ResourceKey, Slot>,
    next_epoch: u64,
    exhausted: bool,
    closed: bool,
    limit: usize,
}

#[derive(Default)]
struct Slot {
    latest: LatestTask,
    epoch: u64,
    demand_generation: u64,
    phase: Phase,
    pending: Option<PendingOperation>,
    keep_ready: bool,
    cancellation: Option<CancellationToken>,
    cancel_requested: bool,
    settlement: Option<Arc<OperationSettlement>>,
}

impl Slot {
    fn is_cancelled(&self) -> bool {
        self.cancel_requested
            || self
                .cancellation
                .as_ref()
                .is_some_and(CancellationToken::is_cancelled)
    }

    fn reset_cancelled_if_settled(&mut self) {
        if self.pending.is_none() && self.is_cancelled() {
            self.latest.cancel();
            self.phase = Phase::Idle;
            self.settlement = None;
            self.cancellation = None;
            self.cancel_requested = false;
        }
    }
}

#[derive(Clone)]
struct PendingOperation {
    ticket: TaskTicket,
    epoch: u64,
    demand_generation: u64,
    previous: PreviousSlot,
}

#[derive(Clone)]
struct PreviousSlot {
    phase: Phase,
    epoch: u64,
    demand_generation: u64,
    cancellation: Option<CancellationToken>,
    cancel_requested: bool,
    settlement: Option<Arc<OperationSettlement>>,
}

#[derive(Clone, Copy, Debug, Default)]
enum Phase {
    #[default]
    Idle,
    Running {
        ticket: TaskTicket,
        effect_id: u64,
    },
    Ready,
    Backoff {
        deadline: u64,
    },
}

fn cancel_slot(slot: &mut Slot) {
    slot.cancel_requested = true;
    if let Some(token) = &slot.cancellation {
        token.cancel();
    }
    if let Some(pending) = &mut slot.pending {
        // Explicit cancellation applies to both replacement and predecessor,
        // so rejection cannot revive the predecessor later.
        pending.previous.cancel_requested = true;
        if let Some(token) = &pending.previous.cancellation {
            token.cancel();
        }
    }
    slot.reset_cancelled_if_settled();
}

impl State {
    fn new(ledger: ResourceInterestLedger, limit: usize) -> Self {
        Self {
            ledger,
            slots: HashMap::new(),
            next_epoch: 1,
            exhausted: false,
            closed: false,
            limit,
        }
    }

    fn allocate_epoch(&mut self) -> Result<u64, ResourceOperationAdmissionError> {
        if self.exhausted {
            return Err(ResourceOperationAdmissionError::EpochExhausted);
        }
        let epoch = self.next_epoch.max(1);
        match epoch.checked_add(1) {
            Some(next) => self.next_epoch = next,
            None => self.exhausted = true,
        }
        Ok(epoch)
    }

    fn prune(&mut self) {
        let ledger = self.ledger.clone();
        self.slots.retain(|key, slot| {
            let live = ledger.demand_generation(key) == Some(slot.demand_generation);
            if !live {
                slot.latest.cancel();
                slot.pending = None;
                slot.settlement = None;
                slot.cancellation = None;
                slot.cancel_requested = false;
                if !matches!(slot.phase, Phase::Ready) {
                    slot.phase = Phase::Idle;
                }
            } else {
                slot.reset_cancelled_if_settled();
            }
            live || (slot.keep_ready
                && matches!(slot.phase, Phase::Ready)
                && slot.pending.is_none())
        });
    }
}

fn current_probe(
    state: Weak<Mutex<State>>,
    key: ResourceKey,
    ticket: TaskTicket,
    epoch: u64,
    demand_generation: u64,
) -> Arc<dyn Fn() -> bool + Send + Sync + 'static> {
    Arc::new(move || {
        state.upgrade().is_some_and(|state| {
            let state = lock(&state);
            state.ledger.demand_generation(&key) == Some(demand_generation)
                && state.slots.get(&key).is_some_and(|slot| {
                    slot.epoch == epoch
                        && slot.demand_generation == demand_generation
                        && !slot.is_cancelled()
                        && slot.latest.is_active(ticket)
                })
        })
    })
}

fn settle_pending(
    state: &Weak<Mutex<State>>,
    key: &ResourceKey,
    ticket: TaskTicket,
    epoch: u64,
    demand_generation: u64,
    settlement: LatestTaskTransactionSettlement,
) {
    let Some(state) = state.upgrade() else {
        return;
    };
    let mut state = lock(&state);
    let demand_live = state.ledger.demand_generation(key) == Some(demand_generation);
    let Some(slot) = state.slots.get_mut(key) else {
        return;
    };
    let Some(pending) = slot.pending.clone() else {
        return;
    };
    if pending.ticket != ticket
        || pending.epoch != epoch
        || pending.demand_generation != demand_generation
    {
        return;
    }
    slot.pending = None;
    if settlement == LatestTaskTransactionSettlement::Rejected {
        if demand_live && pending.previous.demand_generation == demand_generation {
            slot.phase = pending.previous.phase;
            slot.epoch = pending.previous.epoch;
            slot.demand_generation = pending.previous.demand_generation;
            slot.cancellation = pending.previous.cancellation;
            slot.cancel_requested = pending.previous.cancel_requested;
            slot.settlement = pending.previous.settlement;
        } else {
            slot.phase = Phase::Idle;
            slot.epoch = 0;
            slot.demand_generation = 0;
            slot.cancellation = None;
            slot.cancel_requested = false;
            slot.settlement = None;
        }
        // An explicit cancellation or a predecessor token that became
        // cancelled while replacement was pending must not revive running work.
        slot.reset_cancelled_if_settled();
    }
    // The settlement callback runs after LatestTask has released its mutex.
    slot.latest.clear_resolved_replacement(ticket);
}

/// Operation-local settlement evidence survives slot replacement so application
/// state can distinguish a rejected begin from an older predecessor.
#[derive(Debug)]
struct OperationSettlement {
    state: AtomicU8,
}

impl OperationSettlement {
    const PENDING: u8 = 0;
    const ACCEPTED: u8 = 1;
    const REJECTED: u8 = 2;

    fn pending() -> Self {
        Self {
            state: AtomicU8::new(Self::PENDING),
        }
    }

    fn set(&self, settlement: LatestTaskTransactionSettlement) {
        let state = match settlement {
            LatestTaskTransactionSettlement::Accepted => Self::ACCEPTED,
            LatestTaskTransactionSettlement::Rejected => Self::REJECTED,
        };
        self.state.store(state, Ordering::Release);
    }

    fn was_rejected(&self) -> bool {
        self.state.load(Ordering::Acquire) == Self::REJECTED
    }
}

fn lock<T>(state: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    state
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}
