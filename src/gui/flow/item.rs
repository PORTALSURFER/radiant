/// One item and its desired main-axis width for flow packing.
#[derive(Clone, Debug, PartialEq)]
pub struct FlowItem<T> {
    /// Caller-owned item payload.
    pub value: T,
    /// Desired width in logical pixels.
    pub width: f32,
}

impl<T> FlowItem<T> {
    /// Construct one flow item from a caller payload and desired width.
    pub const fn new(value: T, width: f32) -> Self {
        Self { value, width }
    }
}

/// Trait for row payloads that expose a desired flow width.
pub trait FlowItemWidth {
    /// Desired width in logical pixels.
    fn flow_width(&self) -> f32;
}

impl<T> FlowItemWidth for FlowItem<T> {
    fn flow_width(&self) -> f32 {
        self.width
    }
}
