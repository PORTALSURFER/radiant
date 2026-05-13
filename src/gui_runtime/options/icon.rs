/// RGBA icon bytes used to initialize a native window icon.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WindowIconRgba {
    /// RGBA pixel bytes in row-major order.
    pub rgba: Vec<u8>,
    /// Icon width in pixels.
    pub width: u32,
    /// Icon height in pixels.
    pub height: u32,
}
