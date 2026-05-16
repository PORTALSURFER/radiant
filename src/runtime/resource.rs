//! Domain-neutral resource loading state for runtime-managed background work.

mod key;
mod load;
mod request;
mod slot;

pub use key::ResourceKey;
pub use load::{ResourceCompletion, ResourceLoad};
pub use request::ResourceRequest;
pub use slot::{ResourceLoadState, ResourceSlot};
