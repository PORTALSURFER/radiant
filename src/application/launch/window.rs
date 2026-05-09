/// Builder for no-state native windows.
pub struct WindowBuilder {
    options: NativeRunOptions,
}

impl WindowBuilder {
    fn new(title: impl Into<String>) -> Self {
        Self {
            options: NativeRunOptions {
                title: title.into(),
                ..NativeRunOptions::default()
            },
        }
    }

    /// Set the initial logical window size.
    pub fn size(mut self, width: u32, height: u32) -> Self {
        self.options.inner_size = Some([width as f32, height as f32]);
        self
    }

    /// Set the minimum logical window size.
    pub fn min_size(mut self, width: u32, height: u32) -> Self {
        self.options.min_inner_size = Some([width as f32, height as f32]);
        self
    }

    /// Set the full native runtime options, preserving this builder as a thin adapter.
    pub fn options(mut self, options: NativeRunOptions) -> Self {
        self.options = options;
        self
    }

    /// Run one static view through the native Vello runtime.
    pub fn run<View>(self, view: View) -> Result
    where
        View: IntoView<()> + 'static,
    {
        let surface = Arc::new(view.into_surface());
        let bridge = declarative_command_runtime_bridge(
            surface,
            |surface| Arc::clone(surface),
            |_, ()| Command::none(),
        );
        run_native_vello_runtime(self.options, bridge)
    }

    /// Run an existing runtime bridge through this window builder.
    ///
    /// This keeps host-specific bridges on the same application/window launch
    /// API as ordinary Radiant examples while preserving their custom state and
    /// diagnostics.
    pub fn run_bridge<Bridge, Message>(self, bridge: Bridge) -> Result
    where
        Bridge: RuntimeBridge<Message> + 'static,
        Message: 'static,
    {
        run_native_vello_runtime(self.options, bridge)
    }

    /// Run an existing runtime bridge and return native runtime artifacts.
    pub fn run_bridge_with_artifacts<Bridge, Message>(
        self,
        bridge: Bridge,
    ) -> crate::gui_runtime::NativeGenericRunReport
    where
        Bridge: RuntimeBridge<Message> + 'static,
        Message: 'static,
    {
        crate::runtime::run_native_vello_runtime_with_artifacts(self.options, bridge)
    }
}
