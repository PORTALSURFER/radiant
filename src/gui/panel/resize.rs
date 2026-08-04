use crate::{
    gui::types::Point,
    widgets::{DragHandleMessage, DragHandleMetadata, EditEvent, EditPhase, InteractionProvenance},
};

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

    /// Build resize constraints for a left-edge handle.
    pub fn left(min_size: f32, max_size: f32) -> Self {
        Self::new(PanelResizeEdge::Left, min_size, max_size)
    }

    /// Build resize constraints for a right-edge handle.
    pub fn right(min_size: f32, max_size: f32) -> Self {
        Self::new(PanelResizeEdge::Right, min_size, max_size)
    }

    /// Build resize constraints for a top-edge handle.
    pub fn top(min_size: f32, max_size: f32) -> Self {
        Self::new(PanelResizeEdge::Top, min_size, max_size)
    }

    /// Build resize constraints for a bottom-edge handle.
    pub fn bottom(min_size: f32, max_size: f32) -> Self {
        Self::new(PanelResizeEdge::Bottom, min_size, max_size)
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

    /// Build collapsible resize constraints for a left-edge handle.
    pub fn left(min_size: f32, max_size: f32, collapsed_size: f32) -> Self {
        PanelResizeConstraints::left(min_size, max_size).collapsible(collapsed_size)
    }

    /// Build collapsible resize constraints for a right-edge handle.
    pub fn right(min_size: f32, max_size: f32, collapsed_size: f32) -> Self {
        PanelResizeConstraints::right(min_size, max_size).collapsible(collapsed_size)
    }

    /// Build collapsible resize constraints for a top-edge handle.
    pub fn top(min_size: f32, max_size: f32, collapsed_size: f32) -> Self {
        PanelResizeConstraints::top(min_size, max_size).collapsible(collapsed_size)
    }

    /// Build collapsible resize constraints for a bottom-edge handle.
    pub fn bottom(min_size: f32, max_size: f32, collapsed_size: f32) -> Self {
        PanelResizeConstraints::bottom(min_size, max_size).collapsible(collapsed_size)
    }
}

/// Durable panel size plus transient resize-drag and shared-edit state.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PanelResizeState {
    size: f32,
    last_expanded_size: Option<f32>,
    active_drag: Option<PanelResizeDrag>,
    active_edit: Option<EditEvent<f32>>,
}

impl PanelResizeState {
    /// Build panel resize state with the provided durable size.
    pub fn new(size: f32) -> Self {
        let size = finite_or(size, 0.0).max(0.0);
        Self {
            size,
            last_expanded_size: Some(size),
            active_drag: None,
            active_edit: None,
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
        self.remember_expanded_size_above(self.size, constraints.min_size);
    }

    /// Apply one drag-handle message to this panel's resize state.
    ///
    /// Returns the new durable size when the message changes it.
    pub fn resize(
        &mut self,
        message: DragHandleMessage,
        constraints: PanelResizeConstraints,
    ) -> Option<f32> {
        let previous_size = self.size;
        let event = self.resize_edit(message, constraints)?;
        match event.phase {
            EditPhase::Begin => None,
            EditPhase::Update | EditPhase::Commit => Some(event.value),
            EditPhase::Cancel if previous_size != event.value => Some(event.value),
            EditPhase::Cancel => None,
        }
    }

    /// Apply one drag-handle message and return its accepted typed edit boundary.
    ///
    /// The returned event is one boundary for the active pointer transaction.
    /// A start emits `Begin`, active motion emits `Update`, release emits
    /// `Commit`, and cancellation emits `Cancel` after restoring the captured
    /// start size. Orphaned motion, release, and cancellation messages return
    /// `None`. The typed lifecycle is intentionally qualified; use
    /// [`Self::resize`] when only the concise size projection is needed.
    pub fn resize_edit(
        &mut self,
        message: DragHandleMessage,
        constraints: PanelResizeConstraints,
    ) -> Option<EditEvent<f32>> {
        self.apply_resize_edit(message, constraints.normalized(), None)
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
        let previous_size = self.size;
        if message.is_double_activate() {
            let _ = self.resize_collapsible_edit(message, constraints);
            return Some(self.size);
        }

        let event = self.resize_collapsible_edit(message, constraints)?;
        match event.phase {
            EditPhase::Begin => None,
            EditPhase::Update | EditPhase::Commit => Some(event.value),
            EditPhase::Cancel if previous_size != event.value => Some(event.value),
            EditPhase::Cancel => None,
        }
    }

    /// Apply one drag-handle message to a collapsible panel and return its
    /// accepted typed edit boundary.
    ///
    /// Double activation is a discrete collapse/restore command, not a
    /// continuous edit boundary. It therefore clears any active drag and edit,
    /// applies the existing collapse/restore behavior, and returns `None`.
    /// Normal pointer boundaries follow [`Self::resize_edit`], including
    /// cancellation rollback to the transaction's start size.
    pub fn resize_collapsible_edit(
        &mut self,
        message: DragHandleMessage,
        constraints: CollapsiblePanelResizeConstraints,
    ) -> Option<EditEvent<f32>> {
        let constraints = constraints
            .resize
            .normalized()
            .collapsible(constraints.collapsed_size);
        if message.is_double_activate() {
            self.active_drag = None;
            self.active_edit = None;
            self.size = self.double_activate_collapsible_size(constraints);
            return None;
        }

        self.apply_resize_edit(
            message,
            constraints.resize,
            Some(constraints.collapsed_size),
        )
    }

    fn apply_resize_edit(
        &mut self,
        message: DragHandleMessage,
        constraints: PanelResizeConstraints,
        collapsed_size: Option<f32>,
    ) -> Option<EditEvent<f32>> {
        let provenance = pointer_provenance(message.input_metadata());
        match message {
            DragHandleMessage::Started { origin, .. } => {
                self.active_drag = Some(PanelResizeDrag::new(constraints.edge, origin, self.size));
                let begin = EditEvent::begin(self.size, provenance);
                self.active_edit = Some(begin);
                Some(begin)
            }
            DragHandleMessage::Moved { position, .. } => {
                let drag = self.active_drag?;
                let previous = self.active_edit?;
                let size = drag.size_at(position, constraints.min_size, constraints.max_size);
                let update = previous.update(size, provenance)?;
                self.size = size;
                self.active_edit = Some(update);
                Some(update)
            }
            DragHandleMessage::Ended { position, .. } => {
                let drag = self.active_drag?;
                let previous = self.active_edit?;
                let size = drag.size_at(position, constraints.min_size, constraints.max_size);
                let commit = previous.commit(size, provenance)?;
                self.size = size;
                self.remember_collapsible_size(size, collapsed_size);
                self.active_drag = None;
                self.active_edit = None;
                Some(commit)
            }
            DragHandleMessage::Cancelled { .. } => {
                let previous = self.active_edit.take()?;
                let cancel = previous.cancel(provenance)?;
                self.size = cancel.value;
                self.active_drag = None;
                Some(cancel)
            }
            DragHandleMessage::DoubleActivate { .. } => None,
        }
    }

    fn remember_collapsible_size(&mut self, size: f32, collapsed_size: Option<f32>) {
        if let Some(collapsed_size) = collapsed_size {
            self.remember_expanded_size_above(size, collapsed_size);
        }
    }

    fn double_activate_collapsible_size(
        &mut self,
        constraints: CollapsiblePanelResizeConstraints,
    ) -> f32 {
        let collapsed_size = constraints.collapsed_size;
        if self.size <= collapsed_size {
            return self
                .last_expanded_size
                .map(|size| {
                    clamped_panel_size(
                        size,
                        constraints.resize.min_size,
                        constraints.resize.max_size,
                    )
                })
                .filter(|size| *size > collapsed_size)
                .unwrap_or(constraints.resize.max_size);
        }

        self.remember_expanded_size_above(self.size, collapsed_size);
        collapsed_size
    }

    fn remember_expanded_size_above(&mut self, size: f32, collapsed_size: f32) {
        if size > collapsed_size {
            self.last_expanded_size = Some(size);
        }
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
        DragHandleMessage::Started { origin, .. } => {
            *active_drag = Some(PanelResizeDrag::new(edge, origin, current_size));
            None
        }
        DragHandleMessage::Moved { position, .. } | DragHandleMessage::Ended { position, .. } => {
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

fn pointer_provenance(metadata: DragHandleMetadata) -> InteractionProvenance {
    InteractionProvenance::Pointer {
        modifiers: metadata.modifiers,
        timestamp: metadata.timestamp,
        sequence_range: metadata.sequence_range,
    }
}

fn finite_or(value: f32, fallback: f32) -> f32 {
    if value.is_finite() { value } else { fallback }
}
