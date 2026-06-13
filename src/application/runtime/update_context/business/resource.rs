use std::marker::PhantomData;

use crate::{
    application::CancellationToken,
    runtime::{ResourceCompletion, ResourceCompletionParts},
};

use super::{BusinessWorkContext, request::BusinessRequest};

/// Builder for one resource business request.
pub struct BusinessResourceRequest<'context, Message, Output> {
    pub(super) request: BusinessRequest<'context, Message>,
    pub(super) resource: crate::runtime::ResourceRequest,
    pub(super) output: PhantomData<Output>,
}

impl<'context, Message, Output> BusinessResourceRequest<'context, Message, Output>
where
    Output: Send + 'static,
{
    /// Make this resource request cooperatively cancellable.
    pub fn cancellable(self) -> CancellableBusinessResourceRequest<'context, Message, Output> {
        CancellableBusinessResourceRequest {
            request: self.request,
            token: CancellationToken::new(),
            resource: self.resource,
            output: PhantomData,
        }
    }

    /// Run fallible resource work and tag the output with its resource request.
    pub fn run(
        self,
        work: impl FnOnce(BusinessWorkContext) -> Result<Output, String> + Send + 'static,
        map: impl FnOnce(ResourceCompletion<Output>) -> Message + Send + 'static,
    ) {
        let resource = self.resource;
        self.request.run(
            move |context| {
                let load = match work(context) {
                    Ok(value) => resource.ready(value),
                    Err(error) => resource.failed(error),
                };
                ResourceCompletion::from_parts(ResourceCompletionParts {
                    request: resource,
                    load,
                })
            },
            map,
        );
    }
}

/// Cancellable builder for one resource business request.
pub struct CancellableBusinessResourceRequest<'context, Message, Output> {
    pub(super) request: BusinessRequest<'context, Message>,
    pub(super) token: CancellationToken,
    pub(super) resource: crate::runtime::ResourceRequest,
    pub(super) output: PhantomData<Output>,
}

impl<Message, Output> CancellableBusinessResourceRequest<'_, Message, Output> {
    /// Return a clone of the cancellation token owned by this request.
    pub fn token(&self) -> CancellationToken {
        self.token.clone()
    }
}

impl<'context, Message, Output> CancellableBusinessResourceRequest<'context, Message, Output>
where
    Output: Send + 'static,
{
    /// Run cancellable fallible resource work and return its cancellation token.
    pub fn run(
        self,
        work: impl FnOnce(BusinessWorkContext) -> Result<Output, String> + Send + 'static,
        map: impl FnOnce(ResourceCompletion<Output>) -> Message + Send + 'static,
    ) -> CancellationToken {
        let token = self.token.clone();
        let resource = self.resource;
        self.request.run_with_optional_cancellation(
            Some(self.token),
            move |context| {
                let load = match work(context) {
                    Ok(value) => resource.ready(value),
                    Err(error) => resource.failed(error),
                };
                ResourceCompletion::from_parts(ResourceCompletionParts {
                    request: resource,
                    load,
                })
            },
            map,
        );
        token
    }
}
