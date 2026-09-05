use super::{CommandRequest, CommandSource, CommandTarget};

/// Owned semantic activation for a control or native menu adapter.
/// Captures an opaque target, never an already-mapped domain message.
#[derive(Clone, Debug)]
pub struct CommandActivation {
    pub(crate) target: CommandTarget,
    pub(crate) source: CommandSource,
}
impl CommandActivation {
    /// Borrow the request revalidated by the runtime before invoking the mapper.
    pub fn request(&self) -> CommandRequest<'_> {
        CommandRequest::Target(&self.target, self.source)
    }
}
