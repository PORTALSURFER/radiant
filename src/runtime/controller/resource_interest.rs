//! Runtime retirement of application-owned shared-resource interests.

#[cfg(test)]
mod tests;

use super::{SurfaceRuntime, owner::EffectOrigin};
use crate::application::runtime::task::resource_interests::{
    ResourceInterestLeaseWeak, ResourceInterestLiveness,
};
use std::sync::atomic::Ordering;

const MAX_RUNTIME_RESOURCE_INTERESTS: usize = 1024;

#[derive(Default)]
pub(super) struct ResourceInterestRegistry {
    entries: Vec<RegisteredInterest>,
}

struct RegisteredInterest {
    origin: EffectOrigin,
    lease: ResourceInterestLeaseWeak,
    live: ResourceInterestLiveness,
}

impl ResourceInterestRegistry {
    fn is_full(&mut self) -> bool {
        self.entries.retain(|entry| entry.lease.is_live());
        self.entries.len() >= MAX_RUNTIME_RESOURCE_INTERESTS
    }
    /// Retain only retirement authority, never application lease ownership.
    pub(super) fn register(
        &mut self,
        origin: EffectOrigin,
        lease: ResourceInterestLeaseWeak,
        live: ResourceInterestLiveness,
    ) -> bool {
        self.entries.retain(|entry| entry.lease.is_live());
        if self.entries.iter().any(|entry| entry.lease.is_same(&lease)) {
            return true;
        }
        if !origin.is_live()
            || !lease.is_live()
            || self.entries.len() >= MAX_RUNTIME_RESOURCE_INTERESTS
        {
            live.store(false, Ordering::Release);
            lease.release();
            return false;
        }
        self.entries.push(RegisteredInterest {
            origin,
            lease,
            live,
        });
        true
    }

    pub(super) fn retire_origin(&mut self, origin: &EffectOrigin) {
        self.entries.retain(|entry| {
            if &entry.origin == origin {
                entry.live.store(false, Ordering::Release);
                entry.lease.release();
                false
            } else {
                entry.lease.is_live()
            }
        });
    }

    pub(super) fn shutdown(&mut self) {
        for entry in self.entries.drain(..) {
            entry.live.store(false, Ordering::Release);
            entry.lease.release();
        }
    }
}

impl Drop for ResourceInterestRegistry {
    fn drop(&mut self) {
        self.shutdown();
    }
}

impl<Bridge, Message> SurfaceRuntime<Bridge, Message>
where
    Bridge: crate::runtime::RuntimeBridge<Message>,
{
    pub(super) fn admit_resource_interest(
        &mut self,
        effect: &crate::runtime::command::ResourceInterestEffect<Message>,
    ) -> Result<crate::application::ResourceInterest, crate::application::ResourceInterestError>
    {
        use crate::application::ResourceInterestError;
        use crate::runtime::EffectOwner;
        use std::sync::{Arc, atomic::AtomicBool};

        if !self.lifecycle_accepts_work() {
            return Err(ResourceInterestError::Closed);
        }
        let origin = match effect.owner {
            EffectOwner::Application => EffectOrigin::Application,
            EffectOwner::Declarative(handle) => self
                .declarative_owner_origin_for_handle(handle)
                .ok_or(ResourceInterestError::OwnerUnavailable)?,
        };
        // Do not bind a fresh broker when this runtime cannot retain its first
        // retirement guard. Existing brokers may still deduplicate at capacity.
        if self.resource_interests.is_full()
            && !effect.tasks.ledger.is_bound_to(
                crate::application::runtime::task::resource_interests::ResourceInterestRuntimeId::new(self.effect_owner.id())
            )
        {
            return Err(ResourceInterestError::RuntimeCapacity);
        }
        let live = Arc::new(AtomicBool::new(true));
        let interest = effect.tasks.admit_interest(
            self.effect_owner.id(),
            origin.declarative_generation().unwrap_or(0),
            effect.interest_id,
            effect.key.clone(),
            effect.kind,
            live.clone(),
        )?;
        if !self
            .resource_interests
            .register(origin, interest.lease.downgrade(), live)
        {
            return Err(ResourceInterestError::RuntimeCapacity);
        }
        Ok(interest)
    }
}
