use crate::runtime::AuxiliaryWindow;

/// Optional host capability for projecting auxiliary top-level windows.
pub trait RuntimeWindowHost<Message> {
    /// Project additional top-level OS windows owned by the runtime.
    fn project_auxiliary_windows(&mut self) -> Vec<AuxiliaryWindow<Message>> {
        Vec::new()
    }
}

pub(crate) struct RuntimeWindowCapability<Bridge, Message> {
    pub project_auxiliary_windows: fn(&mut Bridge) -> Vec<AuxiliaryWindow<Message>>,
}

impl<Bridge, Message> RuntimeWindowCapability<Bridge, Message>
where
    Bridge: RuntimeWindowHost<Message>,
{
    pub const fn new() -> Self {
        Self {
            project_auxiliary_windows: Bridge::project_auxiliary_windows,
        }
    }
}
