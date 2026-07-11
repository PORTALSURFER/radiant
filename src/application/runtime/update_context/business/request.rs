mod cancellable;
mod dispatch;
mod stream_guard;

pub use cancellable::CancellableBusinessRequest;

use std::hash::Hash;

use crate::{
    application::runtime::ResourceTasks,
    application::{CancellationToken, KeyedLatestTasks, LatestTask},
    runtime::{ResourceKey, ResourceSlot, TaskPriority},
};

use super::{
    keyed_latest::BusinessKeyedLatestRequest, latest::BusinessLatestRequest,
    resource::BusinessResourceRequest,
};
use crate::application::runtime::update_context::UiUpdateContext;

/// Builder for one named business request.
pub struct BusinessRequest<'context, Message> {
    pub(super) context: &'context mut UiUpdateContext<Message>,
    pub(super) name: &'static str,
    pub(super) priority: TaskPriority,
}

impl<'context, Message> BusinessRequest<'context, Message> {
    /// Make this request cooperatively cancellable.
    pub fn cancellable(self) -> CancellableBusinessRequest<'context, Message> {
        CancellableBusinessRequest {
            request: self,
            token: CancellationToken::new(),
        }
    }

    /// Start replace-latest work for one host-owned task slot.
    pub fn latest(self, latest: &mut LatestTask) -> BusinessLatestRequest<'context, Message> {
        BusinessLatestRequest {
            request: self,
            ticket: latest.begin(),
        }
    }

    /// Start replace-latest work for one key in a host-owned task registry.
    pub fn latest_for<Key>(
        self,
        latest: &mut KeyedLatestTasks<Key>,
        key: Key,
    ) -> BusinessKeyedLatestRequest<'context, Message, Key>
    where
        Key: Clone + Eq + Hash,
    {
        BusinessKeyedLatestRequest {
            request: self,
            ticket: latest.begin(key.clone()),
            key,
        }
    }

    /// Start replace-latest work for one generic resource key.
    pub fn latest_for_resource(
        self,
        resources: &mut ResourceTasks,
        key: impl Into<ResourceKey>,
    ) -> BusinessKeyedLatestRequest<'context, Message, ResourceKey> {
        let ticket = resources.begin_latest(key.into());
        let key = ticket.key().clone();
        BusinessKeyedLatestRequest {
            request: self,
            ticket: ticket.ticket(),
            key,
        }
    }

    /// Start exclusive work for one generic resource key.
    ///
    /// Returns `None` when the same key already has an active exclusive task.
    pub fn exclusive_for(
        self,
        resources: &mut ResourceTasks,
        key: impl Into<ResourceKey>,
    ) -> Option<BusinessKeyedLatestRequest<'context, Message, ResourceKey>> {
        let ticket = resources.begin_exclusive(key.into())?;
        let key = ticket.key().clone();
        Some(BusinessKeyedLatestRequest {
            request: self,
            ticket: ticket.ticket(),
            key,
        })
    }

    /// Start a resource load for one host-owned resource slot.
    pub fn resource<Output>(
        self,
        slot: &mut ResourceSlot<Output>,
    ) -> BusinessResourceRequest<'context, Message, Output> {
        BusinessResourceRequest {
            request: self,
            resource: slot.begin_load(),
            output: std::marker::PhantomData,
        }
    }
}
