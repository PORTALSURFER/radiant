use crate::application::{CancellationToken, KeyedTaskCompletion, TaskTicket};

use super::{BusinessWorkContext, request::BusinessRequest};

/// Builder for one keyed-latest business request.
pub struct BusinessKeyedLatestRequest<'context, Message, Key> {
    pub(super) request: BusinessRequest<'context, Message>,
    pub(super) ticket: TaskTicket,
    pub(super) key: Key,
}

impl<Message, Key> BusinessKeyedLatestRequest<'_, Message, Key> {
    /// Return the task ticket assigned to this request.
    pub fn ticket(&self) -> TaskTicket {
        self.ticket
    }

    /// Return the host-owned key for this request.
    pub fn key(&self) -> &Key {
        &self.key
    }
}

impl<'context, Message, Key> BusinessKeyedLatestRequest<'context, Message, Key>
where
    Key: Clone + Send + 'static,
{
    /// Make this keyed latest request cooperatively cancellable.
    pub fn cancellable(self) -> CancellableBusinessKeyedLatestRequest<'context, Message, Key> {
        CancellableBusinessKeyedLatestRequest {
            request: self.request,
            token: CancellationToken::new(),
            ticket: self.ticket,
            key: self.key,
        }
    }

    /// Run keyed latest work and tag the output with its key and task ticket.
    pub fn run<Output>(
        self,
        work: impl FnOnce(BusinessWorkContext) -> Output + Send + 'static,
        map: impl FnOnce(KeyedTaskCompletion<Key, Output>) -> Message + Send + 'static,
    ) where
        Output: Send + 'static,
    {
        let key = self.key;
        let ticket = self.ticket;
        self.request.run(
            move |context| KeyedTaskCompletion {
                key,
                ticket,
                output: work(context),
            },
            map,
        );
    }
}

/// Cancellable builder for one keyed-latest business request.
pub struct CancellableBusinessKeyedLatestRequest<'context, Message, Key> {
    pub(super) request: BusinessRequest<'context, Message>,
    pub(super) token: CancellationToken,
    pub(super) ticket: TaskTicket,
    pub(super) key: Key,
}

impl<Message, Key> CancellableBusinessKeyedLatestRequest<'_, Message, Key> {
    /// Return the task ticket assigned to this request.
    pub fn ticket(&self) -> TaskTicket {
        self.ticket
    }

    /// Return a clone of the cancellation token owned by this request.
    pub fn token(&self) -> CancellationToken {
        self.token.clone()
    }

    /// Return the host-owned key for this request.
    pub fn key(&self) -> &Key {
        &self.key
    }
}

impl<'context, Message, Key> CancellableBusinessKeyedLatestRequest<'context, Message, Key>
where
    Key: Clone + Send + 'static,
{
    /// Run cancellable keyed latest work and return its cancellation token.
    pub fn run<Output>(
        self,
        work: impl FnOnce(BusinessWorkContext) -> Output + Send + 'static,
        map: impl FnOnce(KeyedTaskCompletion<Key, Output>) -> Message + Send + 'static,
    ) -> CancellationToken
    where
        Output: Send + 'static,
    {
        let token = self.token.clone();
        let key = self.key;
        let ticket = self.ticket;
        self.request.run_with_optional_cancellation(
            Some(self.token),
            move |context| KeyedTaskCompletion {
                key,
                ticket,
                output: work(context),
            },
            map,
        );
        token
    }
}
