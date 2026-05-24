//! Widget paint-boundary vocabulary shared by primitives and runtimes.

/// Shared paint clipping contract for widgets.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PaintBounds {
    /// Runtime traversal clips widget paint to the assigned widget rectangle.
    ClipToRect,
    /// Paint may extend beyond the assigned rectangle when the parent allows it.
    AllowOverflow,
}

/// Shared paint responsibilities required from every widget primitive.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PaintContract {
    /// Whether paint is clipped to the assigned widget rectangle.
    pub bounds: PaintBounds,
    /// Whether focus state should be expressed visually by the widget itself.
    pub paints_focus: bool,
    /// Whether selection/active state should be expressed visually by the widget.
    pub paints_state_layers: bool,
    /// Whether this widget's own chrome should block hover chrome on parent containers.
    pub suppresses_container_hover: bool,
}

impl Default for PaintContract {
    fn default() -> Self {
        Self {
            bounds: PaintBounds::ClipToRect,
            paints_focus: true,
            paints_state_layers: true,
            suppresses_container_hover: false,
        }
    }
}
