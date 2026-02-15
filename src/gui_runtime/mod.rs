//! Shared GUI runtime host implementations.

mod native_vello;

/// RGBA icon bytes used to initialize a native window icon.
#[derive(Clone, Debug)]
pub struct WindowIconRgba {
    /// RGBA pixel bytes in row-major order.
    pub rgba: Vec<u8>,
    /// Icon width in pixels.
    pub width: u32,
    /// Icon height in pixels.
    pub height: u32,
}

/// Window configuration shared by native runtime entry points.
#[derive(Clone, Debug)]
pub struct NativeRunOptions {
    /// Window title.
    pub title: String,
    /// Initial window inner size in logical points.
    pub inner_size: Option<[f32; 2]>,
    /// Minimum window inner size in logical points.
    pub min_inner_size: Option<[f32; 2]>,
    /// Whether the window starts maximized.
    pub maximized: bool,
    /// Optional window icon.
    pub icon: Option<WindowIconRgba>,
}

impl Default for NativeRunOptions {
    fn default() -> Self {
        Self {
            title: String::from("Sempal"),
            inner_size: None,
            min_inner_size: None,
            maximized: false,
            icon: None,
        }
    }
}

pub use native_vello::{run_native_vello_app, run_native_vello_preview};
