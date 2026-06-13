use crate::application::{CancellationToken, TaskCompletion, TaskTicket};

use super::{BusinessWorkContext, request::BusinessRequest};

/// Builder for one latest business request.
pub struct BusinessLatestRequest<'context, Message> {
    pub(super) request: BusinessRequest<'context, Message>,
    pub(super) ticket: TaskTicket,
}

impl<Message> BusinessLatestRequest<'_, Message> {
    /// Return the task ticket assigned to this request.
    pub fn ticket(&self) -> TaskTicket {
        self.ticket
    }
}

impl<'context, Message> BusinessLatestRequest<'context, Message> {
    /// Make this latest request cooperatively cancellable.
    pub fn cancellable(self) -> CancellableBusinessLatestRequest<'context, Message> {
        CancellableBusinessLatestRequest {
            request: self.request,
            token: CancellationToken::new(),
            ticket: self.ticket,
        }
    }

    /// Run latest work and tag the output with its task ticket.
    pub fn run<Output>(
        self,
        work: impl FnOnce(BusinessWorkContext) -> Output + Send + 'static,
        map: impl FnOnce(TaskCompletion<Output>) -> Message + Send + 'static,
    ) where
        Output: Send + 'static,
    {
        let ticket = self.ticket;
        self.request.run(
            move |context| TaskCompletion {
                ticket,
                output: work(context),
            },
            map,
        );
    }
}

/// Cancellable builder for one latest business request.
pub struct CancellableBusinessLatestRequest<'context, Message> {
    pub(super) request: BusinessRequest<'context, Message>,
    pub(super) token: CancellationToken,
    pub(super) ticket: TaskTicket,
}

impl<Message> CancellableBusinessLatestRequest<'_, Message> {
    /// Return the task ticket assigned to this request.
    pub fn ticket(&self) -> TaskTicket {
        self.ticket
    }

    /// Return a clone of the cancellation token owned by this request.
    pub fn token(&self) -> CancellationToken {
        self.token.clone()
    }
}

impl<'context, Message> CancellableBusinessLatestRequest<'context, Message> {
    /// Run cancellable latest work and return its cancellation token.
    pub fn run<Output>(
        self,
        work: impl FnOnce(BusinessWorkContext) -> Output + Send + 'static,
        map: impl FnOnce(TaskCompletion<Output>) -> Message + Send + 'static,
    ) -> CancellationToken
    where
        Output: Send + 'static,
    {
        let token = self.token.clone();
        let ticket = self.ticket;
        self.request.run_with_optional_cancellation(
            Some(self.token),
            move |context| TaskCompletion {
                ticket,
                output: work(context),
            },
            map,
        );
        token
    }
}
