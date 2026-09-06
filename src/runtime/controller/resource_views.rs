//! Strong leases owned by the accepted resource-view projection.
use super::{SurfaceRuntime, owner::EffectOrigin};
use crate::{
    application::{ResourceInterest, resource_view::demand::ResourceViewDemand},
    runtime::{RuntimeBridge, surface::ResourceViewIdentity},
};
use std::{
    collections::HashMap,
    rc::Rc,
    sync::{Arc, atomic::AtomicBool},
};

/// Bounded interest admission diagnostics for the current accepted resource views.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ResourceViewInterestStatus {
    /// Accepted live projected consumers.
    pub active: usize,
    /// Consumers rejected by a broker or runtime bound.
    pub rejected: usize,
    /// The projected source was ambiguous or exceeded traversal bounds.
    pub source_invalid: bool,
}

pub(super) struct ResourceViewInterests {
    entries: HashMap<ResourceViewIdentity, AcceptedInterest>,
    next_owner: Option<u64>,
    status: ResourceViewInterestStatus,
}
struct AcceptedInterest {
    demand: Rc<ResourceViewDemand>,
    interest: ResourceInterest,
}
impl Default for ResourceViewInterests {
    fn default() -> Self {
        Self {
            entries: HashMap::new(),
            next_owner: Some(1),
            status: Default::default(),
        }
    }
}
impl ResourceViewInterests {
    pub(super) fn clear(&mut self) {
        self.entries.clear();
        self.status = Default::default();
    }
}

impl<Bridge: RuntimeBridge<Message>, Message> SurfaceRuntime<Bridge, Message> {
    /// Read interest diagnostics without starting work or acquiring demand.
    pub fn resource_view_interest_status(&self) -> ResourceViewInterestStatus {
        ResourceViewInterestStatus {
            active: self
                .resource_views
                .entries
                .values()
                .filter(|entry| entry.interest.is_live())
                .count(),
            ..self.resource_views.status
        }
    }

    pub(super) fn install_resource_view_interests(&mut self) {
        if !self.lifecycle_accepts_work() {
            return;
        }
        let Some(mut projected) = self.surface.resource_view_demands() else {
            self.resource_views.clear();
            self.resource_views.status.source_invalid = true;
            return;
        };
        let desired: HashMap<_, _> = projected
            .iter()
            .map(|item| (&item.identity, &item.demand))
            .collect();
        self.resource_views.entries.retain(|identity, entry| {
            desired.get(identity).is_some_and(|demand| {
                entry.interest.is_live()
                    && entry.demand.key == demand.key
                    && entry.demand.tasks.same_broker(&demand.tasks)
            })
        });
        drop(desired);
        // Stable projection order breaks ties; visible admission precedes speculative demand.
        projected.sort_by_key(|item| match item.demand.kind {
            crate::application::ResourceInterestKind::Visible => 0,
            crate::application::ResourceInterestKind::Persistent => 1,
            crate::application::ResourceInterestKind::Prefetch => 2,
        });
        self.resource_views.status = Default::default();
        for item in projected {
            let identity = item.identity;
            let demand = item.demand;
            if let Some(entry) = self.resource_views.entries.get_mut(&identity) {
                entry.interest.set_kind(demand.kind);
                entry.demand = demand;
                continue;
            }
            // Check the aggregate bound before binding a new broker.
            if self.resource_interests.is_full() {
                self.resource_views.status.rejected += 1;
                continue;
            }
            let Some(owner) = self.resource_views.next_owner else {
                self.resource_views.status.rejected += 1;
                continue;
            };
            self.resource_views.next_owner = owner.checked_add(1);
            let live = Arc::new(AtomicBool::new(true));
            let result = demand.tasks.admit_view_interest(
                self.effect_owner.id(),
                owner,
                demand.interest_id,
                demand.key.clone(),
                demand.kind,
                live.clone(),
            );
            let Ok(interest) = result else {
                self.resource_views.status.rejected += 1;
                continue;
            };
            if !self.resource_interests.register(
                EffectOrigin::Application,
                interest.lease.downgrade(),
                live,
            ) {
                self.resource_views.status.rejected += 1;
                continue;
            }
            self.resource_views
                .entries
                .insert(identity, AcceptedInterest { demand, interest });
        }
        self.resource_views.status.active = self.resource_views.entries.len();
    }
}
