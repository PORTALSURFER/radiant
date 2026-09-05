//! Application-owned projection receipt continuity.
//!
//! A request owns one lowering transaction. The runtime bridge consumes its
//! stage decision, and only a later observed surface generation acknowledges
//! the staged receipt as committed.

use super::super::environment::ApplicationEnvironment;
use super::super::view_node::reconciliation::{ApplicationProjectionReceipt, ReceiptComparison};
use crate::gui::types::Rect;
use crate::runtime::{
    ExactChangedRoot, SurfaceRefreshRequest, SurfaceUpdateProviderAuthority, WindowEnvironment,
};
use std::cell::Cell;
use std::num::NonZeroU64;
use std::rc::Rc;
use std::sync::{
    OnceLock,
    atomic::{AtomicU64, Ordering},
};

static PROCESS_OWNER_ALLOCATOR: OnceLock<OwnerAllocator> = OnceLock::new();

fn process_owner_allocator() -> &'static OwnerAllocator {
    PROCESS_OWNER_ALLOCATOR.get_or_init(OwnerAllocator::new)
}

struct OwnerAllocator {
    next: AtomicU64,
}

impl OwnerAllocator {
    const fn new() -> Self {
        Self {
            next: AtomicU64::new(1),
        }
    }

    fn allocate(&self) -> Option<NonZeroU64> {
        let mut current = self.next.load(Ordering::Acquire);
        loop {
            if current == 0 {
                return None;
            }
            let next = current.checked_add(1).unwrap_or_default();
            match self.next.compare_exchange_weak(
                current,
                next,
                Ordering::AcqRel,
                Ordering::Acquire,
            ) {
                Ok(_) => return NonZeroU64::new(current),
                Err(observed) => current = observed,
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ProjectionFences {
    pub(crate) viewport: Rect,
    pub(crate) window: WindowEnvironment,
    pub(crate) application: Option<ApplicationEnvironment>,
}

impl ProjectionFences {
    fn from_request(request: &ProducerRequest) -> Self {
        Self {
            viewport: request.viewport,
            window: request.window,
            application: request.application.clone(),
        }
    }

    fn matches(&self, other: &Self) -> bool {
        self.viewport.min.x.to_bits() == other.viewport.min.x.to_bits()
            && self.viewport.min.y.to_bits() == other.viewport.min.y.to_bits()
            && self.viewport.max.x.to_bits() == other.viewport.max.x.to_bits()
            && self.viewport.max.y.to_bits() == other.viewport.max.y.to_bits()
            && self.window == other.window
            && self.application == other.application
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ProducerRequest {
    pub(crate) runtime_identity: u64,
    pub(crate) request_revision: u64,
    pub(crate) active_surface_generation: u64,
    pub(crate) expected_provider_authority: Option<SurfaceUpdateProviderAuthority>,
    pub(crate) viewport: Rect,
    pub(crate) window: WindowEnvironment,
    pub(crate) application: Option<ApplicationEnvironment>,
}

impl ProducerRequest {
    pub(crate) fn from_runtime(
        request: SurfaceRefreshRequest,
        application: Option<ApplicationEnvironment>,
    ) -> Self {
        Self {
            runtime_identity: request.runtime_identity,
            request_revision: request.request_revision,
            active_surface_generation: request.active_surface_generation,
            expected_provider_authority: request.expected_provider_authority,
            viewport: request.viewport,
            window: request.window_environment,
            application,
        }
    }

    fn valid_numbers(&self) -> bool {
        self.runtime_identity != 0
            && self.request_revision != 0
            && self.request_revision != u64::MAX
            && self.active_surface_generation != 0
            && self.active_surface_generation != u64::MAX
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Continuity {
    Trusted,
    NeedsFull,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RunState {
    Idle,
    InProgress,
    Poisoned,
}

struct PendingProjection {
    runtime_identity: u64,
    base_generation: u64,
    successor_generation: u64,
    request_revision: u64,
    fences: ProjectionFences,
    receipt: Rc<ApplicationProjectionReceipt>,
}

#[allow(clippy::large_enum_variant)]
enum Phase {
    Fresh,
    StartupUnbound {
        committed: Rc<ApplicationProjectionReceipt>,
        continuity: Continuity,
    },
    Bound {
        runtime_identity: u64,
        committed_generation: u64,
        committed: Rc<ApplicationProjectionReceipt>,
        committed_fences: ProjectionFences,
        continuity: Continuity,
        pending: Option<PendingProjection>,
        last_completed_request_revision: Option<u64>,
    },
    Poisoned,
}

struct ValidatedRequestTicket {
    request: ProducerRequest,
    baseline: Rc<ApplicationProjectionReceipt>,
    observed_base_generation: u64,
    successor_generation: u64,
    fences: ProjectionFences,
    authority: Option<SurfaceUpdateProviderAuthority>,
    exact_eligible: bool,
    stageable: bool,
}

pub(crate) struct Candidate<Payload = Rc<ApplicationProjectionReceipt>> {
    pub(crate) payload: Payload,
    receipt: Rc<ApplicationProjectionReceipt>,
    comparison: ReceiptComparison,
    request_echo: Option<ProducerRequest>,
}

impl<Payload> Candidate<Payload> {
    pub(crate) fn new(
        payload: Payload,
        receipt: ApplicationProjectionReceipt,
        comparison: ReceiptComparison,
    ) -> Self {
        Self {
            payload,
            receipt: Rc::new(receipt),
            comparison,
            request_echo: None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum StageDecision {
    Full,
    Exact {
        provider_authority: SurfaceUpdateProviderAuthority,
        changed_roots: Vec<ExactChangedRoot>,
    },
}

pub(crate) struct ApplicationProjectionProducer {
    owner: Option<NonZeroU64>,
    authority_revision: Option<NonZeroU64>,
    phase: Phase,
    run_state: Cell<RunState>,
}

impl ApplicationProjectionProducer {
    pub(crate) fn new() -> Self {
        let owner = process_owner_allocator().allocate();
        let authority_revision = owner.and(NonZeroU64::new(1));
        Self {
            owner,
            authority_revision,
            phase: Phase::Fresh,
            run_state: Cell::new(RunState::Idle),
        }
    }

    #[cfg(test)]
    fn with_owner(owner: Option<NonZeroU64>) -> Self {
        Self {
            authority_revision: owner.and(NonZeroU64::new(1)),
            owner,
            phase: Phase::Fresh,
            run_state: Cell::new(RunState::Idle),
        }
    }

    pub(crate) fn current_authority(&self) -> Option<SurfaceUpdateProviderAuthority> {
        if self.run_state.get() == RunState::Poisoned
            || matches!(self.phase, Phase::Fresh | Phase::Poisoned)
        {
            return None;
        }
        Some(SurfaceUpdateProviderAuthority {
            owner: self.owner?.get(),
            checked_revision: self.authority_revision?.get(),
        })
    }

    pub(crate) fn commit_startup(&mut self, receipt: Rc<ApplicationProjectionReceipt>) -> bool {
        if !matches!(self.phase, Phase::Fresh) || self.run_state.get() != RunState::Idle {
            return false;
        }
        self.phase = Phase::StartupUnbound {
            committed: receipt,
            continuity: Continuity::Trusted,
        };
        true
    }

    pub(crate) fn invalidate_external_projection(&mut self) {
        match &mut self.phase {
            Phase::StartupUnbound { continuity, .. } => {
                *continuity = Continuity::NeedsFull;
                self.bump_authority();
            }
            Phase::Bound {
                continuity,
                pending,
                ..
            } => {
                *continuity = Continuity::NeedsFull;
                *pending = None;
                self.bump_authority();
            }
            Phase::Fresh | Phase::Poisoned => {}
        }
    }

    pub(crate) fn begin_request(&mut self, request: ProducerRequest) -> RequestTransaction<'_> {
        if self.run_state.get() != RunState::Idle {
            self.poison();
            return RequestTransaction::new(
                &mut self.phase,
                &mut self.authority_revision,
                &self.run_state,
                None,
            );
        }
        let ticket = self.prepare_ticket(request);
        self.run_state.set(RunState::InProgress);
        RequestTransaction::new(
            &mut self.phase,
            &mut self.authority_revision,
            &self.run_state,
            ticket,
        )
    }

    fn prepare_ticket(&mut self, request: ProducerRequest) -> Option<ValidatedRequestTicket> {
        let authority = self.authority_value()?;
        let phase = std::mem::replace(&mut self.phase, Phase::Poisoned);
        match phase {
            Phase::StartupUnbound {
                committed,
                continuity,
            } => {
                if !request.valid_numbers()
                    || request.expected_provider_authority != Some(authority)
                {
                    self.phase = Phase::StartupUnbound {
                        committed,
                        continuity: Continuity::NeedsFull,
                    };
                    if request.request_revision == u64::MAX
                        || request.active_surface_generation == u64::MAX
                    {
                        self.disable_authority();
                    } else {
                        self.bump_authority();
                    }
                    return None;
                }
                let baseline = Rc::clone(&committed);
                let fences = ProjectionFences::from_request(&request);
                let Some(successor_generation) = request.active_surface_generation.checked_add(1)
                else {
                    self.phase = Phase::StartupUnbound {
                        committed,
                        continuity,
                    };
                    self.disable_authority();
                    return None;
                };
                if successor_generation == u64::MAX {
                    self.phase = Phase::StartupUnbound {
                        committed,
                        continuity,
                    };
                    self.disable_authority();
                    return None;
                }
                let continuity_value = continuity;
                self.phase = Phase::Bound {
                    runtime_identity: request.runtime_identity,
                    committed_generation: request.active_surface_generation,
                    committed,
                    committed_fences: fences.clone(),
                    continuity: continuity_value,
                    pending: None,
                    last_completed_request_revision: None,
                };
                let base_generation = request.active_surface_generation;
                Some(ValidatedRequestTicket {
                    request,
                    baseline,
                    observed_base_generation: base_generation,
                    successor_generation,
                    exact_eligible: continuity_value == Continuity::Trusted,
                    fences,
                    authority: Some(authority),
                    stageable: true,
                })
            }
            Phase::Bound {
                runtime_identity,
                mut committed_generation,
                mut committed,
                mut committed_fences,
                mut continuity,
                pending,
                mut last_completed_request_revision,
            } => {
                let numbers_valid = request.valid_numbers();
                let same_runtime = request.runtime_identity == runtime_identity;
                let authority_valid = request.expected_provider_authority == Some(authority);
                let revision_valid = last_completed_request_revision
                    .is_none_or(|last| request.request_revision > last);
                let mut trusted_request =
                    numbers_valid && same_runtime && authority_valid && revision_valid;
                let mut runtime_identity = runtime_identity;

                let mut pending = pending;
                if let Some(previous) = pending {
                    if numbers_valid
                        && same_runtime
                        && request.runtime_identity == previous.runtime_identity
                        && authority_valid
                        && revision_valid
                        && request.active_surface_generation == previous.successor_generation
                        && request.request_revision > previous.request_revision
                    {
                        committed_generation = previous.successor_generation;
                        committed = previous.receipt;
                        committed_fences = previous.fences;
                        continuity = Continuity::Trusted;
                        pending = None;
                    } else if numbers_valid
                        && same_runtime
                        && request.runtime_identity == previous.runtime_identity
                        && authority_valid
                        && revision_valid
                        && request.active_surface_generation == previous.base_generation
                    {
                        // The runtime held the old surface; the unpublished
                        // candidate cannot replace the committed receipt.
                        pending = None;
                    } else {
                        continuity = Continuity::NeedsFull;
                        if numbers_valid && (!same_runtime || revision_valid) {
                            if !same_runtime {
                                last_completed_request_revision = None;
                            }
                            runtime_identity = request.runtime_identity;
                            committed_generation = request.active_surface_generation;
                        }
                        self.bump_authority();
                        trusted_request = false;
                        pending = None;
                    }
                } else if !numbers_valid
                    || !same_runtime
                    || !authority_valid
                    || !revision_valid
                    || request.active_surface_generation != committed_generation
                {
                    continuity = Continuity::NeedsFull;
                    if numbers_valid && (!same_runtime || revision_valid) {
                        if !same_runtime {
                            last_completed_request_revision = None;
                        }
                        runtime_identity = request.runtime_identity;
                        committed_generation = request.active_surface_generation;
                    }
                    self.bump_authority();
                    trusted_request = false;
                }

                if !numbers_valid {
                    if request.request_revision == u64::MAX
                        || request.active_surface_generation == u64::MAX
                    {
                        self.disable_authority();
                    }
                    self.phase = Phase::Bound {
                        runtime_identity,
                        committed_generation,
                        committed,
                        committed_fences,
                        continuity,
                        pending: None,
                        last_completed_request_revision,
                    };
                    return None;
                }
                let Some(successor_generation) = request.active_surface_generation.checked_add(1)
                else {
                    self.phase = Phase::Bound {
                        runtime_identity,
                        committed_generation,
                        committed,
                        committed_fences,
                        continuity,
                        pending: None,
                        last_completed_request_revision,
                    };
                    self.disable_authority();
                    return None;
                };
                if successor_generation == u64::MAX {
                    self.phase = Phase::Bound {
                        runtime_identity,
                        committed_generation,
                        committed,
                        committed_fences,
                        continuity,
                        pending: None,
                        last_completed_request_revision,
                    };
                    self.disable_authority();
                    return None;
                }
                let fences = ProjectionFences::from_request(&request);
                let exact_eligible = trusted_request
                    && continuity == Continuity::Trusted
                    && fences.matches(&committed_fences)
                    && request.expected_provider_authority == self.authority_value();
                let base_generation = request.active_surface_generation;
                let ticket = ValidatedRequestTicket {
                    request: request.clone(),
                    baseline: Rc::clone(&committed),
                    observed_base_generation: base_generation,
                    successor_generation,
                    fences,
                    authority: self.authority_value(),
                    exact_eligible,
                    stageable: (!same_runtime || revision_valid)
                        && self.authority_value().is_some(),
                };
                self.phase = Phase::Bound {
                    runtime_identity,
                    committed_generation,
                    committed,
                    committed_fences,
                    continuity,
                    pending,
                    last_completed_request_revision,
                };
                Some(ticket)
            }
            phase @ (Phase::Fresh | Phase::Poisoned) => {
                self.phase = phase;
                None
            }
        }
    }

    fn authority_value(&self) -> Option<SurfaceUpdateProviderAuthority> {
        Some(SurfaceUpdateProviderAuthority {
            owner: self.owner?.get(),
            checked_revision: self.authority_revision?.get(),
        })
    }

    fn bump_authority(&mut self) {
        let Some(current) = self.authority_revision else {
            return;
        };
        let Some(next) = current.get().checked_add(1).and_then(NonZeroU64::new) else {
            self.authority_revision = None;
            return;
        };
        self.authority_revision = Some(next);
    }

    fn disable_authority(&mut self) {
        self.authority_revision = None;
    }

    fn poison(&mut self) {
        self.phase = Phase::Poisoned;
        self.run_state.set(RunState::Poisoned);
        self.authority_revision = None;
    }

    #[cfg(test)]
    fn committed_generation(&self) -> Option<u64> {
        match &self.phase {
            Phase::Bound {
                committed_generation,
                ..
            } => Some(*committed_generation),
            _ => None,
        }
    }

    #[cfg(test)]
    fn pending_summary(&self) -> Option<(u64, u64, u64)> {
        match &self.phase {
            Phase::Bound {
                pending: Some(pending),
                ..
            } => Some((
                pending.base_generation,
                pending.successor_generation,
                pending.request_revision,
            )),
            _ => None,
        }
    }

    #[cfg(test)]
    fn last_completed_request_revision(&self) -> Option<u64> {
        match &self.phase {
            Phase::Bound {
                last_completed_request_revision,
                ..
            } => *last_completed_request_revision,
            _ => None,
        }
    }
}

pub(crate) struct RequestTransaction<'a> {
    guard: RequestGuard<'a>,
    ticket: Option<ValidatedRequestTicket>,
}

struct RequestGuard<'a> {
    phase: &'a mut Phase,
    authority_revision: &'a mut Option<NonZeroU64>,
    run_state: &'a Cell<RunState>,
    complete: bool,
}

impl<'a> RequestGuard<'a> {
    fn new(
        phase: &'a mut Phase,
        authority_revision: &'a mut Option<NonZeroU64>,
        run_state: &'a Cell<RunState>,
    ) -> Self {
        Self {
            phase,
            authority_revision,
            run_state,
            complete: false,
        }
    }

    fn bump_authority(&mut self) {
        let Some(current) = *self.authority_revision else {
            return;
        };
        *self.authority_revision = current.get().checked_add(1).and_then(NonZeroU64::new);
    }

    fn finish(&mut self) {
        self.complete = true;
        if self.run_state.get() == RunState::InProgress {
            self.run_state.set(RunState::Idle);
        }
    }
}

impl Drop for RequestGuard<'_> {
    fn drop(&mut self) {
        if !self.complete {
            *self.phase = Phase::Poisoned;
            self.run_state.set(RunState::Poisoned);
        }
    }
}

impl<'a> RequestTransaction<'a> {
    fn new(
        phase: &'a mut Phase,
        authority_revision: &'a mut Option<NonZeroU64>,
        run_state: &'a Cell<RunState>,
        ticket: Option<ValidatedRequestTicket>,
    ) -> Self {
        Self {
            guard: RequestGuard::new(phase, authority_revision, run_state),
            ticket,
        }
    }

    pub(crate) fn project<Payload>(
        mut self,
        lower_once: impl FnOnce(Option<&ApplicationProjectionReceipt>) -> Candidate<Payload>,
    ) -> ProjectedTransaction<'a, Payload> {
        let baseline = self.ticket.as_ref().map(|ticket| ticket.baseline.as_ref());
        let candidate = lower_once(baseline);
        ProjectedTransaction {
            guard: self.guard,
            ticket: self.ticket.take(),
            candidate,
        }
    }
}

pub(crate) struct ProjectedTransaction<'a, Payload> {
    guard: RequestGuard<'a>,
    ticket: Option<ValidatedRequestTicket>,
    candidate: Candidate<Payload>,
}

impl<Payload> ProjectedTransaction<'_, Payload> {
    pub(crate) fn stage(mut self) -> (Candidate<Payload>, StageDecision) {
        let decision = self.finish(true);
        let candidate = self.candidate;
        self.guard.finish();
        (candidate, decision)
    }

    #[cfg(test)]
    pub(crate) fn abort(mut self) -> (Candidate<Payload>, StageDecision) {
        let decision = self.finish(false);
        let candidate = self.candidate;
        self.guard.finish();
        (candidate, decision)
    }

    fn finish(&mut self, stage: bool) -> StageDecision {
        let Some(ticket) = self.ticket.take() else {
            return StageDecision::Full;
        };
        self.candidate.request_echo = Some(ticket.request.clone());
        if stage && ticket.stageable {
            let exact_roots = match &self.candidate.comparison {
                ReceiptComparison::Exact(roots) if ticket.exact_eligible && !roots.is_empty() => {
                    Some(roots.clone())
                }
                _ => None,
            };
            if let Phase::Bound {
                pending,
                last_completed_request_revision,
                ..
            } = self.guard.phase
            {
                *pending = Some(PendingProjection {
                    runtime_identity: ticket.request.runtime_identity,
                    base_generation: ticket.observed_base_generation,
                    successor_generation: ticket.successor_generation,
                    request_revision: ticket.request.request_revision,
                    fences: ticket.fences,
                    receipt: Rc::clone(&self.candidate.receipt),
                });
                *last_completed_request_revision = Some(
                    last_completed_request_revision
                        .map_or(ticket.request.request_revision, |last| {
                            last.max(ticket.request.request_revision)
                        }),
                );
            }
            exact_roots.map_or(StageDecision::Full, |changed_roots| {
                ticket
                    .authority
                    .map_or(StageDecision::Full, |provider_authority| {
                        StageDecision::Exact {
                            provider_authority,
                            changed_roots,
                        }
                    })
            })
        } else {
            if let Phase::Bound {
                pending,
                continuity,
                last_completed_request_revision,
                ..
            } = self.guard.phase
            {
                *pending = None;
                *continuity = Continuity::NeedsFull;
                *last_completed_request_revision = Some(
                    last_completed_request_revision
                        .map_or(ticket.request.request_revision, |last| {
                            last.max(ticket.request.request_revision)
                        }),
                );
                if !stage {
                    self.guard.bump_authority();
                }
            }
            StageDecision::Full
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::LocaleId;
    use crate::runtime::WindowColorScheme;
    use crate::theme::DpiScale;
    use std::panic::AssertUnwindSafe;

    #[allow(clippy::arc_with_non_send_sync)]
    fn receipt() -> Rc<ApplicationProjectionReceipt> {
        Rc::new(ApplicationProjectionReceipt {
            nodes: Box::new([]),
            supported: true,
            emitted_records: 0,
            comparison_count: 0,
        })
    }

    fn request(
        authority: Option<SurfaceUpdateProviderAuthority>,
        revision: u64,
        generation: u64,
    ) -> ProducerRequest {
        ProducerRequest {
            runtime_identity: 41,
            request_revision: revision,
            active_surface_generation: generation,
            expected_provider_authority: authority,
            viewport: Rect::default(),
            window: WindowEnvironment::default(),
            application: None,
        }
    }

    fn exact() -> ReceiptComparison {
        ReceiptComparison::Exact(vec![ExactChangedRoot {
            node_id: 7,
            child_path: vec![0],
        }])
    }

    fn candidate(comparison: ReceiptComparison) -> Candidate {
        let receipt = receipt();
        Candidate {
            payload: Rc::clone(&receipt),
            receipt,
            comparison,
            request_echo: None,
        }
    }

    fn make_producer(owner: Option<NonZeroU64>) -> ApplicationProjectionProducer {
        ApplicationProjectionProducer::with_owner(owner)
    }

    fn startup_bound(
        producer: &mut ApplicationProjectionProducer,
    ) -> SurfaceUpdateProviderAuthority {
        assert!(producer.commit_startup(receipt()));
        producer.current_authority().unwrap()
    }

    fn pull(
        producer: &mut ApplicationProjectionProducer,
        request: ProducerRequest,
        comparison: ReceiptComparison,
    ) -> StageDecision {
        producer
            .begin_request(request)
            .project(|_| candidate(comparison))
            .stage()
            .1
    }

    #[test]
    fn same_request_binds_startup_projects_once_and_stages() {
        let mut producer = make_producer(NonZeroU64::new(7));
        let authority = startup_bound(&mut producer);
        let q1 = request(Some(authority), 1, 1);
        let mut projections = 0;
        let (candidate, decision) = producer
            .begin_request(q1.clone())
            .project(|_| {
                projections += 1;
                candidate(exact())
            })
            .stage();
        assert_eq!(projections, 1);
        let echoed = candidate.request_echo.as_ref().expect("request echo");
        assert_eq!(echoed.runtime_identity, q1.runtime_identity);
        assert_eq!(echoed.request_revision, q1.request_revision);
        assert_eq!(
            echoed.active_surface_generation,
            q1.active_surface_generation
        );
        assert_eq!(echoed.viewport, q1.viewport);
        assert_eq!(echoed.window, q1.window);
        assert_eq!(echoed.application, q1.application);
        assert_eq!(
            echoed.expected_provider_authority,
            q1.expected_provider_authority
        );
        assert!(matches!(
            decision,
            StageDecision::Exact {
                provider_authority,
                ..
            } if provider_authority == authority
        ));
        assert_eq!(producer.current_authority(), Some(authority));
        assert_eq!(producer.pending_summary(), Some((1, 2, 1)));
        assert_eq!(producer.committed_generation(), Some(1));

        let q2 = request(Some(authority), 2, 2);
        let decision = pull(&mut producer, q2, exact());
        assert!(matches!(
            decision,
            StageDecision::Exact {
                provider_authority,
                ..
            } if provider_authority == authority
        ));
        assert_eq!(producer.committed_generation(), Some(2));
    }

    #[test]
    fn distinct_receipts_are_retained_and_supplied_as_the_next_baseline() {
        struct OneShotPayload(u8);

        let mut producer = make_producer(NonZeroU64::new(7));
        let s0 = receipt();
        assert!(producer.commit_startup(Rc::clone(&s0)));
        let authority = producer.current_authority().unwrap();
        let s1 = receipt();
        let mut saw_s0 = false;
        let (candidate, _) = producer
            .begin_request(request(Some(authority), 1, 1))
            .project(|baseline| {
                saw_s0 = baseline.is_some_and(|baseline| std::ptr::eq(baseline, s0.as_ref()));
                Candidate {
                    payload: OneShotPayload(1),
                    receipt: Rc::clone(&s1),
                    comparison: exact(),
                    request_echo: None,
                }
            })
            .stage();
        assert!(saw_s0);
        assert_eq!(candidate.payload.0, 1);
        assert!(Rc::ptr_eq(&candidate.receipt, &s1));

        let s2 = receipt();
        let mut saw_old_after_hold = false;
        let (candidate, _) = producer
            .begin_request(request(Some(authority), 2, 1))
            .project(|baseline| {
                saw_old_after_hold =
                    baseline.is_some_and(|baseline| std::ptr::eq(baseline, s0.as_ref()));
                Candidate {
                    payload: 2_u64,
                    receipt: Rc::clone(&s2),
                    comparison: exact(),
                    request_echo: None,
                }
            })
            .stage();
        assert!(saw_old_after_hold);
        assert!(Rc::ptr_eq(&candidate.receipt, &s2));

        let s3 = receipt();
        let mut saw_s2 = false;
        let (candidate, _) = producer
            .begin_request(request(Some(authority), 3, 2))
            .project(|baseline| {
                saw_s2 = baseline.is_some_and(|baseline| std::ptr::eq(baseline, s2.as_ref()));
                Candidate {
                    payload: 3_u64,
                    receipt: Rc::clone(&s3),
                    comparison: ReceiptComparison::Full,
                    request_echo: None,
                }
            })
            .stage();
        assert!(saw_s2);
        assert!(Rc::ptr_eq(&candidate.receipt, &s3));
    }

    #[test]
    fn held_exact_is_discarded_and_retried_against_old_receipt() {
        let mut producer = make_producer(NonZeroU64::new(7));
        let authority = startup_bound(&mut producer);
        assert!(matches!(
            pull(&mut producer, request(Some(authority), 1, 1), exact()),
            StageDecision::Exact { .. }
        ));
        let decision = pull(&mut producer, request(Some(authority), 2, 1), exact());
        assert!(matches!(decision, StageDecision::Exact { .. }));
        assert_eq!(producer.committed_generation(), Some(1));
    }

    #[test]
    fn published_full_promotes_then_allows_exact_recovery() {
        let mut producer = make_producer(NonZeroU64::new(7));
        let authority = startup_bound(&mut producer);
        assert_eq!(
            pull(
                &mut producer,
                request(Some(authority), 1, 1),
                ReceiptComparison::Full
            ),
            StageDecision::Full
        );
        assert!(matches!(
            pull(&mut producer, request(Some(authority), 2, 2), exact()),
            StageDecision::Exact { .. }
        ));
        assert_eq!(producer.committed_generation(), Some(2));
    }

    #[test]
    fn stale_same_runtime_full_keeps_request_high_water_and_cannot_promote() {
        let mut producer = make_producer(NonZeroU64::new(7));
        let authority = startup_bound(&mut producer);
        assert!(matches!(
            pull(&mut producer, request(Some(authority), 1, 1), exact()),
            StageDecision::Exact { .. }
        ));
        assert!(matches!(
            pull(&mut producer, request(Some(authority), 2, 2), exact()),
            StageDecision::Exact { .. }
        ));
        assert_eq!(producer.last_completed_request_revision(), Some(2));

        let stale = request(Some(authority), 1, 2);
        assert_eq!(
            pull(&mut producer, stale, ReceiptComparison::Full),
            StageDecision::Full
        );
        assert_eq!(producer.last_completed_request_revision(), Some(2));

        let authority = producer.current_authority().unwrap();
        assert_eq!(
            pull(&mut producer, request(Some(authority), 2, 3), exact()),
            StageDecision::Full
        );
        assert_eq!(producer.last_completed_request_revision(), Some(2));
        assert_eq!(producer.committed_generation(), Some(2));
    }

    #[test]
    fn runtime_rebind_resets_request_high_water_for_new_runtime() {
        let mut producer = make_producer(NonZeroU64::new(7));
        let authority = startup_bound(&mut producer);
        assert!(matches!(
            pull(&mut producer, request(Some(authority), 1, 1), exact()),
            StageDecision::Exact { .. }
        ));
        assert!(matches!(
            pull(&mut producer, request(Some(authority), 2, 2), exact()),
            StageDecision::Exact { .. }
        ));
        assert_eq!(producer.last_completed_request_revision(), Some(2));

        let runtime_two_request = ProducerRequest {
            runtime_identity: 99,
            ..request(Some(authority), 1, 4)
        };
        assert_eq!(
            pull(&mut producer, runtime_two_request, ReceiptComparison::Full),
            StageDecision::Full
        );
        assert_eq!(producer.last_completed_request_revision(), Some(1));
        let runtime_two_authority = producer.current_authority().unwrap();
        assert_ne!(runtime_two_authority, authority);

        assert!(matches!(
            pull(
                &mut producer,
                ProducerRequest {
                    runtime_identity: 99,
                    ..request(Some(runtime_two_authority), 2, 5)
                },
                exact(),
            ),
            StageDecision::Exact { .. }
        ));
        assert_eq!(producer.committed_generation(), Some(5));
        assert_eq!(producer.last_completed_request_revision(), Some(2));
    }

    #[test]
    fn authority_revision_max_bump_is_permanently_exhausted() {
        let mut producer = make_producer(NonZeroU64::new(7));
        let initial_authority = startup_bound(&mut producer);
        let _ = pull(
            &mut producer,
            request(Some(initial_authority), 1, 1),
            exact(),
        );
        producer.authority_revision = NonZeroU64::new(u64::MAX);
        let authority = producer.current_authority().unwrap();
        assert_eq!(authority.checked_revision, u64::MAX);

        assert_eq!(
            pull(
                &mut producer,
                ProducerRequest {
                    runtime_identity: 99,
                    ..request(Some(authority), 1, 1)
                },
                ReceiptComparison::Full,
            ),
            StageDecision::Full
        );
        assert!(producer.current_authority().is_none());
        assert_eq!(producer.authority_revision, None);
    }

    #[test]
    fn viewport_window_and_application_fences_force_full_then_recover() {
        let mut producer = make_producer(NonZeroU64::new(7));
        let authority = startup_bound(&mut producer);
        assert!(matches!(
            pull(&mut producer, request(Some(authority), 1, 1), exact()),
            StageDecision::Exact { .. }
        ));

        let mut viewport = request(Some(authority), 2, 2);
        viewport.viewport = Rect::from_size(100.0, 40.0);
        assert_eq!(
            pull(&mut producer, viewport.clone(), exact()),
            StageDecision::Full
        );
        let authority = producer.current_authority().unwrap();
        let mut window = request(Some(authority), 3, 3);
        window.window = WindowEnvironment::new(
            DpiScale::new(2.0),
            Some(WindowColorScheme::Dark),
            true,
            true,
        );
        assert_eq!(
            pull(&mut producer, window.clone(), exact()),
            StageDecision::Full
        );
        let authority = producer.current_authority().unwrap();
        let mut application = request(Some(authority), 4, 4);
        application.application = Some(ApplicationEnvironment::new(
            LocaleId::new("fr").expect("test locale"),
        ));
        assert_eq!(
            pull(&mut producer, application, exact()),
            StageDecision::Full
        );
    }

    #[test]
    fn generation_jump_rebinds_untrusted_base_and_recovers_after_full() {
        let mut producer = make_producer(NonZeroU64::new(7));
        let authority = startup_bound(&mut producer);
        assert!(matches!(
            pull(&mut producer, request(Some(authority), 1, 1), exact()),
            StageDecision::Exact { .. }
        ));
        let jump = request(Some(authority), 2, 4);
        let decision = pull(&mut producer, jump, exact());
        assert_eq!(decision, StageDecision::Full);
        let authority = producer.current_authority().unwrap();
        assert!(matches!(
            pull(&mut producer, request(Some(authority), 3, 5), exact()),
            StageDecision::Exact { .. }
        ));
    }

    #[test]
    fn external_invalidation_applies_to_startup_and_bound() {
        let mut producer = make_producer(NonZeroU64::new(7));
        assert!(producer.commit_startup(receipt()));
        producer.invalidate_external_projection();
        assert!(producer.current_authority().is_some());
        let authority = producer.current_authority().unwrap();
        assert_eq!(
            pull(&mut producer, request(Some(authority), 1, 1), exact()),
            StageDecision::Full
        );

        producer.invalidate_external_projection();
        let authority = producer.current_authority().unwrap();
        assert_eq!(
            pull(&mut producer, request(Some(authority), 2, 1), exact()),
            StageDecision::Full
        );
    }

    #[test]
    fn wrong_runtime_stale_revision_and_authority_produce_full_without_authority() {
        let mut producer = make_producer(NonZeroU64::new(7));
        let authority = startup_bound(&mut producer);
        assert!(matches!(
            pull(&mut producer, request(Some(authority), 1, 1), exact()),
            StageDecision::Exact { provider_authority, .. }
                if provider_authority == authority
        ));
        let wrong = request(Some(authority), 2, 2);
        let wrong = ProducerRequest {
            runtime_identity: 99,
            ..wrong
        };
        assert_eq!(pull(&mut producer, wrong, exact()), StageDecision::Full);
        assert!(producer.current_authority().is_some());
        let authority = producer.current_authority().unwrap();
        let stale = request(Some(authority), 2, 5);
        assert_eq!(pull(&mut producer, stale, exact()), StageDecision::Full);
        let mismatch = request(
            Some(SurfaceUpdateProviderAuthority {
                owner: 999,
                checked_revision: 1,
            }),
            3,
            5,
        );
        assert_eq!(pull(&mut producer, mismatch, exact()), StageDecision::Full);
    }

    #[test]
    fn max_owner_and_none_owner_are_permanent_full_modes() {
        let allocator = OwnerAllocator {
            next: AtomicU64::new(u64::MAX),
        };
        assert_eq!(allocator.allocate(), NonZeroU64::new(u64::MAX));
        assert_eq!(allocator.allocate(), None);
        assert_eq!(allocator.allocate(), None);

        let mut producer = make_producer(None);
        assert!(producer.commit_startup(receipt()));
        assert!(producer.current_authority().is_none());
        let decision = pull(&mut producer, request(None, 1, 1), exact());
        assert_eq!(decision, StageDecision::Full);
        assert!(producer.current_authority().is_none());
    }

    #[test]
    fn reserved_request_and_generation_disable_authority_without_pending() {
        let mut producer = make_producer(NonZeroU64::new(7));
        let authority = startup_bound(&mut producer);
        assert_eq!(
            pull(
                &mut producer,
                request(Some(authority), u64::MAX, 1),
                exact()
            ),
            StageDecision::Full
        );
        assert!(producer.current_authority().is_none());

        let mut producer = make_producer(NonZeroU64::new(7));
        let authority = startup_bound(&mut producer);
        assert_eq!(
            pull(
                &mut producer,
                request(Some(authority), 1, u64::MAX - 1),
                exact()
            ),
            StageDecision::Full
        );
        assert!(producer.current_authority().is_none());
    }

    #[test]
    fn dropped_transaction_poison_and_non_idle_entry_stays_full() {
        let mut producer = make_producer(NonZeroU64::new(7));
        let authority = startup_bound(&mut producer);
        let transaction = producer.begin_request(request(Some(authority), 1, 1));
        drop(transaction);
        assert!(producer.current_authority().is_none());

        let mut producer = make_producer(NonZeroU64::new(7));
        let authority = startup_bound(&mut producer);
        producer.run_state.set(RunState::InProgress);
        let decision = producer
            .begin_request(request(Some(authority), 1, 1))
            .project(|_| candidate(exact()))
            .abort()
            .1;
        assert_eq!(decision, StageDecision::Full);
        assert!(producer.current_authority().is_none());
    }

    #[test]
    fn abort_clears_pending_and_bumps_authority_without_exact() {
        let mut producer = make_producer(NonZeroU64::new(7));
        let authority = startup_bound(&mut producer);
        let (_candidate, decision) = producer
            .begin_request(request(Some(authority), 1, 1))
            .project(|_| candidate(exact()))
            .abort();
        assert_eq!(decision, StageDecision::Full);
        assert_eq!(
            producer.current_authority(),
            Some(SurfaceUpdateProviderAuthority {
                owner: 7,
                checked_revision: 2,
            })
        );
        assert_eq!(producer.pending_summary(), None);
    }

    #[test]
    fn projection_panic_unwinds_through_drop_guard() {
        let mut producer = make_producer(NonZeroU64::new(7));
        let authority = startup_bound(&mut producer);
        let result = std::panic::catch_unwind(AssertUnwindSafe(|| {
            let _ = producer
                .begin_request(request(Some(authority), 1, 1))
                .project::<()>(|_| panic!("projection panic"));
        }));
        assert!(result.is_err());
        assert!(producer.current_authority().is_none());
    }
}
