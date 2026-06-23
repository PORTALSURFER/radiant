use super::super::defaults::default_gpu_surface_sizing;
use super::core::view_node_from_widget;
use crate::{
    application::{MappedWidget, ViewNode},
    runtime::{GpuSurfaceCapabilities, GpuSurfaceContent, GpuSurfaceOverlay, WidgetMessageMapper},
    widgets::{GpuSurfaceMessage, GpuSurfaceParts, GpuSurfaceWidget, WidgetInput},
};

/// Named construction inputs for an input-emitting retained GPU surface view.
///
/// This keeps retained resource identity, content generation, and application
/// message mapping explicit at call sites that wire GPU-heavy interactive
/// widgets into the normal Radiant message path.
pub struct GpuSurfaceInputParts<Map> {
    /// Stable surface key used by native backends for retained GPU resources.
    pub key: u64,
    /// Monotonic content revision for retained GPU resources.
    pub revision: u64,
    /// Backend-neutral retained GPU content.
    pub content: GpuSurfaceContent,
    /// Mapper from routed widget input to the host application's message type.
    pub map: Map,
}

/// Named construction inputs for a configured retained GPU surface view.
///
/// This keeps retained resource identity, content generation, runtime
/// capabilities, and backend-composited overlays explicit without requiring a
/// host to write a custom widget whose only job is GPU-surface paint.
#[derive(Clone, Debug, PartialEq)]
pub struct GpuSurfaceConfiguredParts {
    /// Stable surface key used by native backends for retained GPU resources.
    pub key: u64,
    /// Monotonic content revision for retained GPU resources.
    pub revision: u64,
    /// Backend-neutral retained GPU content.
    pub content: GpuSurfaceContent,
    /// Runtime interaction capabilities requested by this GPU surface.
    pub capabilities: GpuSurfaceCapabilities,
    /// Optional lightweight overlays composited by the native GPU backend.
    pub overlays: Vec<GpuSurfaceOverlay>,
}

impl GpuSurfaceConfiguredParts {
    /// Build configured GPU surface parts from the required retained payload.
    pub fn new(key: u64, revision: u64, content: GpuSurfaceContent) -> Self {
        Self {
            key,
            revision,
            content,
            capabilities: GpuSurfaceCapabilities::default(),
            overlays: Vec::new(),
        }
    }

    /// Replace runtime interaction capabilities.
    pub fn capabilities(mut self, capabilities: GpuSurfaceCapabilities) -> Self {
        self.capabilities = capabilities;
        self
    }

    /// Replace lightweight backend-composited overlays.
    pub fn overlays(mut self, overlays: Vec<GpuSurfaceOverlay>) -> Self {
        self.overlays = overlays;
        self
    }
}

/// Build a retained GPU surface view with generated application identity.
///
/// The surface lowers through the same widget/layout/paint path as standard
/// widgets and emits a `PaintPrimitive::GpuSurface` for native GPU backends.
pub fn gpu_surface<Message: 'static>(
    key: u64,
    revision: u64,
    content: GpuSurfaceContent,
) -> ViewNode<Message> {
    gpu_surface_from_parts(GpuSurfaceParts {
        id: 0,
        sizing: default_gpu_surface_sizing(),
        key,
        revision,
        content,
    })
}

/// Build a retained GPU surface view from named construction inputs.
///
/// This is the readable companion to [`gpu_surface`] for call sites where the
/// retained resource identity and revision are easier to review as named fields.
pub fn gpu_surface_from_parts<Message: 'static>(parts: GpuSurfaceParts) -> ViewNode<Message> {
    view_node_from_widget(GpuSurfaceWidget::from_parts(parts))
}

/// Build a configured retained GPU surface view from named construction inputs.
///
/// Use this when a host needs generic runtime capabilities such as fast pointer
/// motion, coalesced wheel routing, runtime overlays, or lightweight
/// backend-composited overlays while keeping the surface on the normal
/// declarative application path.
pub fn gpu_surface_configured_from_parts<Message: 'static>(
    parts: GpuSurfaceConfiguredParts,
) -> ViewNode<Message> {
    view_node_from_widget(
        GpuSurfaceWidget::from_parts(GpuSurfaceParts {
            id: 0,
            sizing: default_gpu_surface_sizing(),
            key: parts.key,
            revision: parts.revision,
            content: parts.content,
        })
        .with_capabilities(parts.capabilities)
        .with_overlays(parts.overlays),
    )
}

/// Build a retained GPU surface view with generated identity and capabilities.
///
/// Use this when a passive retained GPU surface needs generic runtime behavior
/// such as fast pointer motion, coalesced wheel routing, or runtime-owned
/// overlays without dropping to named construction parts.
pub fn gpu_surface_with_capabilities<Message: 'static>(
    key: u64,
    revision: u64,
    content: GpuSurfaceContent,
    capabilities: GpuSurfaceCapabilities,
) -> ViewNode<Message> {
    view_node_from_widget(
        GpuSurfaceWidget::from_parts(GpuSurfaceParts {
            id: 0,
            sizing: default_gpu_surface_sizing(),
            key,
            revision,
            content,
        })
        .with_capabilities(capabilities),
    )
}

/// Build an input-emitting retained GPU surface view with generated identity.
///
/// This keeps GPU-heavy widgets on the same application message path as
/// standard widgets while leaving plain [`gpu_surface`] views passive.
pub fn gpu_surface_input<Message: 'static>(
    key: u64,
    revision: u64,
    content: GpuSurfaceContent,
    map: impl Fn(WidgetInput) -> Message + Send + Sync + 'static,
) -> ViewNode<Message> {
    gpu_surface_input_from_parts(GpuSurfaceInputParts {
        key,
        revision,
        content,
        map,
    })
}

/// Build an input-emitting retained GPU surface view from named construction inputs.
pub fn gpu_surface_input_from_parts<Message, Map>(
    parts: GpuSurfaceInputParts<Map>,
) -> ViewNode<Message>
where
    Message: 'static,
    Map: Fn(WidgetInput) -> Message + Send + Sync + 'static,
{
    view_node_from_widget(MappedWidget::new(
        GpuSurfaceWidget::from_parts(GpuSurfaceParts {
            id: 0,
            sizing: default_gpu_surface_sizing(),
            key: parts.key,
            revision: parts.revision,
            content: parts.content,
        })
        .with_input_events(true),
        WidgetMessageMapper::typed(move |message: GpuSurfaceMessage| match message {
            GpuSurfaceMessage::Input { input } => (parts.map)(input),
        }),
    ))
}
