use crate::runtime::{
    PlatformCompletion, PlatformRequest, PlatformResultServiceFallback, PlatformServiceFallback,
    RuntimePlatformResultSink,
};

/// Optional host capability for typed platform services.
#[deprecated(note = "use RuntimePlatformResultHost for new hosts")]
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

/// Optional result-only platform capability for custom runtime hosts.
///
/// Hosts receive only a platform-neutral request and a send-safe result sink;
/// application messages and UI-local mappers remain owned by `SurfaceRuntime`.
pub trait RuntimePlatformResultHost {
    /// Request a host-visible platform service.
    fn request_platform_result(
        &mut self,
        request: PlatformRequest,
        on_completed: RuntimePlatformResultSink,
    ) -> Result<(), PlatformResultServiceFallback> {
        Err(Box::new((request, on_completed)))
    }
}

pub(crate) struct RuntimePlatformCapability<Bridge, Message>(
    std::marker::PhantomData<fn(&mut Bridge, Message)>,
);

type PlatformResultRequestFn<Bridge> = fn(
    &mut Bridge,
    PlatformRequest,
    RuntimePlatformResultSink,
) -> Result<(), PlatformResultServiceFallback>;

pub(crate) struct RuntimePlatformResultCapability<Bridge> {
    pub request_platform_result: PlatformResultRequestFn<Bridge>,
}

#[allow(deprecated)]
impl<Bridge, Message> RuntimePlatformCapability<Bridge, Message>
where
    Bridge: RuntimePlatformHost<Message>,
{
    pub const fn new() -> Self {
        Self(std::marker::PhantomData)
    }
}

impl<Bridge> RuntimePlatformResultCapability<Bridge>
where
    Bridge: RuntimePlatformResultHost,
{
    pub const fn new() -> Self {
        Self {
            request_platform_result: Bridge::request_platform_result,
        }
    }
}
