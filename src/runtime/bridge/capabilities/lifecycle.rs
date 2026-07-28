/// Optional host capability for native runtime lifecycle decisions.
pub trait RuntimeLifecycleHost {
    /// Lifecycle hook fired once when the runtime enters its closing phase.
    fn on_runtime_closing(&mut self) {}

    /// Lifecycle hook fired when the native runtime exits.
    fn on_runtime_exit(&mut self) -> Option<serde_json::Value> {
        None
    }

    /// Return whether the runtime should continue closing the active window.
    fn close_requested(&mut self) -> bool {
        true
    }
}

pub(crate) struct RuntimeLifecycleCapability<Bridge> {
    pub on_runtime_closing: fn(&mut Bridge),
    pub on_runtime_exit: fn(&mut Bridge) -> Option<serde_json::Value>,
    pub close_requested: fn(&mut Bridge) -> bool,
}

impl<Bridge> RuntimeLifecycleCapability<Bridge>
where
    Bridge: RuntimeLifecycleHost,
{
    pub const fn new() -> Self {
        Self {
            on_runtime_closing: Bridge::on_runtime_closing,
            on_runtime_exit: Bridge::on_runtime_exit,
            close_requested: Bridge::close_requested,
        }
    }
}
