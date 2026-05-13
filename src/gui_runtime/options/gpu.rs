/// Explicit native GPU backend preference.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum NativeGpuBackend {
    /// Use WGPU's normal environment-aware backend selection.
    #[default]
    Auto,
    /// Prefer WGPU's primary production backends for the current platform.
    Primary,
    /// Restrict adapter selection to Vulkan.
    Vulkan,
    /// Restrict adapter selection to DirectX 12.
    Dx12,
    /// Restrict adapter selection to Metal.
    Metal,
    /// Restrict adapter selection to OpenGL or OpenGL ES.
    Gl,
    /// Restrict adapter selection to browser WebGPU.
    BrowserWebGpu,
}

/// Native GPU policy used by backend runtime adapters.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct NativeGpuOptions {
    /// Preferred GPU backend for adapter selection.
    pub backend: NativeGpuBackend,
}
