//! Runtime builder helpers for retained GPU surface primitives.

use crate::runtime::{GpuSurfaceContent, SurfaceNode};
use crate::widgets::contract::{WidgetId, WidgetSizing};

use super::{GpuSurfaceParts, GpuSurfaceWidget};

impl<Message> SurfaceNode<Message> {
    /// Build a non-emitting retained GPU surface leaf node.
    pub fn gpu_surface(
        id: WidgetId,
        sizing: WidgetSizing,
        key: u64,
        revision: u64,
        content: GpuSurfaceContent,
    ) -> Self {
        Self::static_widget(GpuSurfaceWidget::from_parts(GpuSurfaceParts {
            id,
            sizing,
            key,
            revision,
            content,
        }))
    }
}
