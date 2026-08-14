//! Shared private lifecycle state for runtime-owned effects.

use super::declarative_owner::DeclarativeOwnerToken;
use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicU64, Ordering},
};

static NEXT_RUNTIME_OWNER_ID: AtomicU64 = AtomicU64::new(1);
static NEXT_AUXILIARY_OWNER_GENERATION: AtomicU64 = AtomicU64::new(1);

/// One runtime's lifecycle.  The owner is intentionally private: it is only
/// used to fence controller worker, timer, and platform registrations.
#[derive(Clone)]
pub(super) struct RuntimeOwner {
    id: u64,
    open: Arc<AtomicBool>,
}

impl Default for RuntimeOwner {
    fn default() -> Self {
        Self::new()
    }
}

impl RuntimeOwner {
    pub(super) fn new() -> Self {
        Self {
            id: NEXT_RUNTIME_OWNER_ID.fetch_add(1, Ordering::Relaxed),
            open: Arc::new(AtomicBool::new(true)),
        }
    }

    pub(super) fn cancel(&self) {
        self.open.store(false, Ordering::Release);
    }

    pub(super) fn is_open(&self) -> bool {
        self.open.load(Ordering::Acquire)
    }

    pub(super) const fn id(&self) -> u64 {
        self.id
    }

    pub(super) fn is_same(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

/// One parent-runtime-owned auxiliary-window generation.
///
/// The stable key is retained for controller lookup, while `generation` is an
/// opaque identity fence for one live or cached native child.  Retiring a
/// generation closes every clone held by a worker registration or native event
/// result without affecting another key or a later same-key generation.
#[derive(Clone)]
pub(crate) struct AuxiliaryWindowOwner {
    key: Arc<str>,
    generation: u64,
    open: Arc<AtomicBool>,
}

impl AuxiliaryWindowOwner {
    pub(crate) fn new(key: &str) -> Self {
        Self {
            key: Arc::from(key),
            generation: NEXT_AUXILIARY_OWNER_GENERATION.fetch_add(1, Ordering::Relaxed),
            open: Arc::new(AtomicBool::new(true)),
        }
    }

    pub(crate) fn key(&self) -> &str {
        &self.key
    }

    pub(crate) fn retire(&self) {
        self.open.store(false, Ordering::Release);
    }

    pub(crate) fn is_open(&self) -> bool {
        self.open.load(Ordering::Acquire)
    }

    pub(crate) fn is_same_generation(&self, other: &Self) -> bool {
        self.generation == other.generation && self.key == other.key
    }
}

/// Internal provenance carried only by controller dispatch and worker
/// registrations.  Public command and bridge APIs remain unchanged.
#[derive(Clone)]
pub(super) enum EffectOrigin {
    Application,
    Auxiliary(AuxiliaryWindowOwner),
    Declarative(DeclarativeOwnerToken),
}

impl EffectOrigin {
    pub(super) fn is_live(&self) -> bool {
        match self {
            Self::Application => true,
            Self::Auxiliary(owner) => owner.is_open(),
            Self::Declarative(owner) => owner.is_live(),
        }
    }

    pub(super) fn cancellation_probe(&self) -> Option<CancellationProbe> {
        match self {
            Self::Application => None,
            Self::Auxiliary(owner) => {
                let owner = owner.clone();
                Some(Arc::new(move || !owner.is_open()))
            }
            Self::Declarative(owner) => {
                let owner = owner.clone();
                Some(Arc::new(move || !owner.is_live()))
            }
        }
    }

    pub(super) fn declarative_generation(&self) -> Option<u64> {
        match self {
            Self::Declarative(owner) => Some(owner.generation()),
            Self::Application | Self::Auxiliary(_) => None,
        }
    }
}

impl PartialEq for EffectOrigin {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Application, Self::Application) => true,
            (Self::Auxiliary(first), Self::Auxiliary(second)) => first.is_same_generation(second),
            (Self::Declarative(first), Self::Declarative(second)) => first == second,
            _ => false,
        }
    }
}

impl Eq for EffectOrigin {}

pub(super) type CancellationProbe = Arc<dyn Fn() -> bool + Send + Sync + 'static>;

/// Descriptor retained by one lane registration and checked before mapping.
/// `slot` records the logical latest/replacement slot while `key` identifies
/// the concrete registration in its lane.
#[derive(Clone)]
pub(super) struct LifecycleDescriptor {
    owner: RuntimeOwner,
    key: u64,
    slot: Option<u64>,
    generation: u64,
    cancellation: Option<CancellationProbe>,
}

impl LifecycleDescriptor {
    pub(super) fn new(
        owner: RuntimeOwner,
        key: u64,
        slot: Option<u64>,
        generation: u64,
        cancellation: Option<CancellationProbe>,
    ) -> Self {
        Self {
            owner,
            key,
            slot,
            generation,
            cancellation,
        }
    }

    pub(super) fn admits(
        &self,
        owner: &RuntimeOwner,
        key: u64,
        generation: u64,
        slot_current: bool,
    ) -> bool {
        self.owner.is_same(owner)
            && owner.is_open()
            && self.key == key
            && self.generation == generation
            && slot_current
            && !self.cancellation.as_ref().is_some_and(|probe| probe())
    }

    pub(super) fn slot(&self) -> Option<u64> {
        self.slot
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicUsize;

    #[test]
    fn admission_requires_owner_key_generation_and_live_policy() {
        let owner = RuntimeOwner::new();
        let cancelled = Arc::new(AtomicBool::new(false));
        let calls = Arc::new(AtomicUsize::new(0));
        let probe = {
            let cancelled = Arc::clone(&cancelled);
            let calls = Arc::clone(&calls);
            Arc::new(move || {
                calls.fetch_add(1, Ordering::Relaxed);
                cancelled.load(Ordering::Acquire)
            }) as CancellationProbe
        };
        let descriptor = LifecycleDescriptor::new(owner.clone(), 7, Some(3), 2, Some(probe));
        assert!(descriptor.admits(&owner, 7, 2, true));
        assert!(!descriptor.admits(&owner, 7, 1, true));
        assert!(!descriptor.admits(&owner, 7, 2, false));
        cancelled.store(true, Ordering::Release);
        assert!(!descriptor.admits(&owner, 7, 2, true));
        owner.cancel();
        assert!(!descriptor.admits(&owner, 7, 2, true));
        assert_eq!(descriptor.slot(), Some(3));
        assert!(calls.load(Ordering::Relaxed) >= 2);
    }
}
