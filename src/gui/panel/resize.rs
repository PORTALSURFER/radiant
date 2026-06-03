use crate::{gui::types::Point, widgets::DragHandleMessage};

/// Panel edge that is being resized by a drag handle.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PanelResizeEdge {
    /// Left edge of a panel; moving left increases size.
    Left,
    /// Right edge of a panel; moving right increases size.
    Right,
    /// Top edge of a panel; moving up increases size.
    Top,
    /// Bottom edge of a panel; moving down increases size.
    Bottom,
}

/// Drag state for resizing a panel or split-pane slot along one edge.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PanelResizeDrag {
    /// Panel edge controlled by the drag handle.
    pub edge: PanelResizeEdge,
    /// Pointer position captured when the drag starts.
    pub start_pointer: Point,
    /// Size captured when the drag starts.
    pub start_size: f32,
}

impl PanelResizeDrag {
    /// Start a panel resize drag.
    pub fn new(edge: PanelResizeEdge, start_pointer: Point, start_size: f32) -> Self {
        Self {
            edge,
            start_pointer,
            start_size: finite_or(start_size, 0.0),
        }
    }

    /// Return the resized panel size for the current pointer position.
    pub fn size_at(self, pointer: Point, min_size: f32, max_size: f32) -> f32 {
        let delta = match self.edge {
            PanelResizeEdge::Left => self.start_pointer.x - pointer.x,
            PanelResizeEdge::Right => pointer.x - self.start_pointer.x,
            PanelResizeEdge::Top => self.start_pointer.y - pointer.y,
            PanelResizeEdge::Bottom => pointer.y - self.start_pointer.y,
        };
        let min_size = finite_or(min_size, 0.0).max(0.0);
        let max_size = finite_or(max_size, min_size).max(min_size);
        finite_or(self.start_size + finite_or(delta, 0.0), self.start_size)
            .clamp(min_size, max_size)
    }
}

/// Apply one drag-handle message to panel resize state.
///
/// Hosts keep the durable size and optional active drag state. This helper
/// centralizes the generic resize lifecycle: start captures the current size,
/// move/end emits a clamped size when an active drag exists, and end clears the
/// active drag. It returns `None` for a start message and for orphaned move/end
/// messages that have no active resize.
pub fn update_panel_resize_drag(
    active_drag: &mut Option<PanelResizeDrag>,
    message: DragHandleMessage,
    edge: PanelResizeEdge,
    current_size: f32,
    min_size: f32,
    max_size: f32,
) -> Option<f32> {
    match message {
        DragHandleMessage::Started { position } => {
            *active_drag = Some(PanelResizeDrag::new(edge, position, current_size));
            None
        }
        DragHandleMessage::Moved { position } | DragHandleMessage::Ended { position } => {
            let size = active_drag.map(|drag| drag.size_at(position, min_size, max_size));
            if message.is_ended() {
                *active_drag = None;
            }
            size
        }
        DragHandleMessage::Cancelled { .. } => {
            *active_drag = None;
            None
        }
        DragHandleMessage::DoubleActivate { .. } => None,
    }
}

fn finite_or(value: f32, fallback: f32) -> f32 {
    if value.is_finite() { value } else { fallback }
}
