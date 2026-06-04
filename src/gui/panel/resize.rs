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

/// Size bounds and resize edge for a resizable panel or split-pane slot.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PanelResizeConstraints {
    /// Panel edge controlled by the drag handle.
    pub edge: PanelResizeEdge,
    /// Smallest allowed panel size.
    pub min_size: f32,
    /// Largest allowed panel size.
    pub max_size: f32,
}

impl PanelResizeConstraints {
    /// Build resize constraints for a panel edge.
    pub fn new(edge: PanelResizeEdge, min_size: f32, max_size: f32) -> Self {
        Self {
            edge,
            min_size,
            max_size,
        }
        .normalized()
    }

    /// Add a double-activation collapse target to these resize constraints.
    pub fn collapsible(self, collapsed_size: f32) -> CollapsiblePanelResizeConstraints {
        let resize = self.normalized();
        CollapsiblePanelResizeConstraints {
            resize,
            collapsed_size: clamped_panel_size(collapsed_size, resize.min_size, resize.max_size),
        }
    }

    fn normalized(self) -> Self {
        let min_size = finite_or(self.min_size, 0.0).max(0.0);
        let max_size = finite_or(self.max_size, min_size).max(min_size);
        Self {
            edge: self.edge,
            min_size,
            max_size,
        }
    }
}

/// Size bounds, resize edge, and collapse target for a collapsible panel.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CollapsiblePanelResizeConstraints {
    /// Normal drag-resize constraints.
    pub resize: PanelResizeConstraints,
    /// Size applied when the resize handle is double-activated.
    pub collapsed_size: f32,
}

impl CollapsiblePanelResizeConstraints {
    /// Build collapsible resize constraints for a panel edge.
    pub fn new(edge: PanelResizeEdge, min_size: f32, max_size: f32, collapsed_size: f32) -> Self {
        PanelResizeConstraints::new(edge, min_size, max_size).collapsible(collapsed_size)
    }
}

/// Durable panel size plus transient resize-drag state.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PanelResizeState {
    size: f32,
    active_drag: Option<PanelResizeDrag>,
}

impl PanelResizeState {
    /// Build panel resize state with the provided durable size.
    pub fn new(size: f32) -> Self {
        Self {
            size: finite_or(size, 0.0).max(0.0),
            active_drag: None,
        }
    }

    /// Return the current durable panel size.
    pub fn size(self) -> f32 {
        self.size
    }

    /// Return the active resize drag, when a drag is in progress.
    pub fn active_drag(self) -> Option<PanelResizeDrag> {
        self.active_drag
    }

    /// Return whether the panel is currently being resized.
    pub fn is_resizing(self) -> bool {
        self.active_drag.is_some()
    }

    /// Set the durable panel size directly, clamped to the supplied constraints.
    pub fn set_size(&mut self, size: f32, constraints: PanelResizeConstraints) {
        let constraints = constraints.normalized();
        self.size = clamped_panel_size(size, constraints.min_size, constraints.max_size);
    }

    /// Apply one drag-handle message to this panel's resize state.
    ///
    /// Returns the new durable size when the message changes it.
    pub fn resize(
        &mut self,
        message: DragHandleMessage,
        constraints: PanelResizeConstraints,
    ) -> Option<f32> {
        let constraints = constraints.normalized();
        let size = update_panel_resize_drag(
            &mut self.active_drag,
            message,
            constraints.edge,
            self.size,
            constraints.min_size,
            constraints.max_size,
        )?;
        self.size = size;
        Some(size)
    }

    /// Apply one drag-handle message to this collapsible panel's resize state.
    ///
    /// Returns the new durable size when the message changes it.
    pub fn resize_collapsible(
        &mut self,
        message: DragHandleMessage,
        constraints: CollapsiblePanelResizeConstraints,
    ) -> Option<f32> {
        let constraints = constraints
            .resize
            .normalized()
            .collapsible(constraints.collapsed_size);
        let size = update_collapsible_panel_resize_drag(
            &mut self.active_drag,
            message,
            constraints.resize.edge,
            self.size,
            constraints.resize.min_size,
            constraints.resize.max_size,
            constraints.collapsed_size,
        )?;
        self.size = size;
        Some(size)
    }
}

impl Default for PanelResizeState {
    fn default() -> Self {
        Self::new(0.0)
    }
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

/// Apply one drag-handle message to collapsible panel resize state.
///
/// This extends [`update_panel_resize_drag`] with a double-activation collapse
/// target while preserving the same drag lifecycle. Hosts still own durable
/// panel size, active drag state, and their chosen min/max/collapsed sizes.
pub fn update_collapsible_panel_resize_drag(
    active_drag: &mut Option<PanelResizeDrag>,
    message: DragHandleMessage,
    edge: PanelResizeEdge,
    current_size: f32,
    min_size: f32,
    max_size: f32,
    collapsed_size: f32,
) -> Option<f32> {
    if message.is_double_activate() {
        *active_drag = None;
        return Some(clamped_panel_size(collapsed_size, min_size, max_size));
    }
    update_panel_resize_drag(active_drag, message, edge, current_size, min_size, max_size)
}

fn clamped_panel_size(size: f32, min_size: f32, max_size: f32) -> f32 {
    let min_size = finite_or(min_size, 0.0).max(0.0);
    let max_size = finite_or(max_size, min_size).max(min_size);
    finite_or(size, min_size).clamp(min_size, max_size)
}

fn finite_or(value: f32, fallback: f32) -> f32 {
    if value.is_finite() { value } else { fallback }
}
