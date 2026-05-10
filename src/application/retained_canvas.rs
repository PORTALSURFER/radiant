use crate::{
    application::{MappedWidget, ViewNode, default_canvas_sizing, view_node_from_widget},
    runtime::WidgetMessageMapper,
    widgets::{CanvasWidget, RetainedSurfaceDescriptor},
};

/// Build a retained canvas view with app-owned paint supplied by the app builder.
pub fn retained_canvas(key: u64) -> RetainedCanvasBuilder {
    RetainedCanvasBuilder {
        descriptor: RetainedSurfaceDescriptor {
            key,
            revision: 0,
            dirty_mask: 0,
            volatile: false,
        },
    }
}

/// Build a retained canvas view from explicit descriptor metadata.
pub fn retained_canvas_with(
    key: u64,
    revision: u64,
    dirty_mask: u64,
    volatile: bool,
) -> RetainedCanvasBuilder {
    retained_canvas(key)
        .revision(revision)
        .dirty_mask(dirty_mask)
        .volatile(volatile)
}

/// Builder for retained canvas views.
pub struct RetainedCanvasBuilder {
    descriptor: RetainedSurfaceDescriptor,
}

impl RetainedCanvasBuilder {
    /// Set the retained content revision.
    pub const fn revision(mut self, revision: u64) -> Self {
        self.descriptor.revision = revision;
        self
    }

    /// Set the retained content dirty mask.
    pub const fn dirty_mask(mut self, dirty_mask: u64) -> Self {
        self.descriptor.dirty_mask = dirty_mask;
        self
    }

    /// Mark this retained canvas as volatile for runtime cache planning.
    pub const fn volatile(mut self, volatile: bool) -> Self {
        self.descriptor.volatile = volatile;
        self
    }

    /// Build a non-emitting retained canvas view.
    pub fn view<Message: 'static>(self) -> ViewNode<Message> {
        view_node_from_widget(
            CanvasWidget::new(0, default_canvas_sizing()).with_retained_surface(self.descriptor),
        )
    }

    /// Build a retained canvas that maps canvas input to host messages.
    pub fn on_input<Message: 'static>(
        self,
        map: impl Fn(crate::widgets::CanvasMessage) -> Message + Send + Sync + 'static,
    ) -> ViewNode<Message> {
        view_node_from_widget(MappedWidget::new(
            CanvasWidget::new(0, default_canvas_sizing()).with_retained_surface(self.descriptor),
            WidgetMessageMapper::canvas(map),
        ))
    }
}
