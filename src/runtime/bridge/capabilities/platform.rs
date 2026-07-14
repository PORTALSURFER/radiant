use crate::runtime::{PlatformCompletion, PlatformRequest, PlatformServiceFallback};

/// Optional host capability for typed platform services.
pub trait RuntimePlatformHost<Message> {
    /// Request a host-visible platform service.
    fn request_platform_service(
        &mut self,
        request: PlatformRequest,
        on_completed: PlatformCompletion<Message>,
    ) -> Result<(), PlatformServiceFallback<Message>> {
        Err(Box::new((request, on_completed)))
    }
}

type PlatformRequestFn<Bridge, Message> = fn(
    &mut Bridge,
    PlatformRequest,
    PlatformCompletion<Message>,
) -> Result<(), PlatformServiceFallback<Message>>;

pub(crate) struct RuntimePlatformCapability<Bridge, Message> {
    pub request_platform_service: PlatformRequestFn<Bridge, Message>,
}

impl<Bridge, Message> RuntimePlatformCapability<Bridge, Message>
where
    Bridge: RuntimePlatformHost<Message>,
{
    pub const fn new() -> Self {
        Self {
            request_platform_service: Bridge::request_platform_service,
        }
    }
}
