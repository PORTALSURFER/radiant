use super::defaults::{
    default_button_sizing, default_canvas_sizing, default_card_sizing, default_gpu_surface_sizing,
    default_text_input_sizing, default_text_sizing, default_toggle_sizing,
};
use crate::{
    application::{DynamicWidget, MappedWidget, ViewNode, ViewNodeKind, WidgetView},
    gui::types::ImageRgba,
    layout::Vector2,
    runtime::{GpuSurfaceContent, PaintText, WidgetMessageMapper},
    widgets::{
        ButtonWidget, CanvasWidget, CardWidget, GpuSurfaceMessage, GpuSurfaceParts,
        GpuSurfaceWidget, ImageWidget, TextInputWidget, TextWidget, ToggleWidget, Widget,
        WidgetInput, WidgetOutput, WidgetSizing,
    },
};
use std::sync::Arc;

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

pub(in crate::application) fn view_node_from_widget<Message>(
    widget: impl WidgetView<Message> + 'static,
) -> ViewNode<Message> {
    ViewNode::new(ViewNodeKind::Widget(Box::new(widget)))
}

/// Build a view node from any application widget view.
pub fn widget<Message>(widget: impl WidgetView<Message> + 'static) -> ViewNode<Message> {
    view_node_from_widget(widget)
}

/// Build a non-interactive text view with generated identity and default sizing.
pub fn text<Message: 'static>(value: impl Into<String>) -> ViewNode<Message> {
    view_node_from_widget(TextWidget::new(
        0,
        PaintText::from(value.into()),
        default_text_sizing(),
    ))
}

/// Build a passive button view for retained surfaces that need button chrome
/// without host messages.
pub fn passive_button<Message: 'static>(label: impl Into<String>) -> ViewNode<Message> {
    view_node_from_widget(ButtonWidget::new(
        0,
        PaintText::from(label.into()),
        default_button_sizing(""),
    ))
}

/// Build a passive toggle view for retained surfaces that need toggle chrome
/// without host messages.
pub fn passive_toggle<Message: 'static>(
    label: impl Into<String>,
    checked: bool,
) -> ViewNode<Message> {
    view_node_from_widget(
        ToggleWidget::new(
            0,
            PaintText::from(label.into()),
            default_toggle_sizing("", true),
        )
        .with_checked(checked),
    )
}

/// Build a passive single-line text input view for retained surfaces that need
/// input chrome without host messages.
pub fn passive_text_input<Message: 'static>(
    value: impl Into<String>,
    placeholder: impl Into<String>,
) -> ViewNode<Message> {
    let mut input = TextInputWidget::new(0, value, default_text_input_sizing());
    let placeholder = placeholder.into();
    if !placeholder.is_empty() {
        input.props.placeholder = Some(placeholder.into());
    }
    view_node_from_widget(input)
}

/// Build a passive card or panel view.
pub fn card<Message: 'static>() -> ViewNode<Message> {
    view_node_from_widget(CardWidget::new(0, default_card_sizing()))
}

/// Build a passive canvas view for retained surfaces that need a generic paint
/// or input slot without host messages.
pub fn canvas<Message: 'static>() -> ViewNode<Message> {
    view_node_from_widget(CanvasWidget::new(0, default_canvas_sizing()))
}

/// Build a non-interactive raster image view.
pub fn image<Message: 'static>(image: Arc<ImageRgba>) -> ViewNode<Message> {
    let size = Vector2::new(image.width.max(1) as f32, image.height.max(1) as f32);
    view_node_from_widget(ImageWidget::new(0, image, WidgetSizing::fixed(size)))
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

/// Build a minimal passive spacer view.
pub fn spacer<Message: 'static>() -> ViewNode<Message> {
    canvas().size(1.0, 1.0)
}

/// Build a custom widget view with generated identity and an output mapper.
pub fn custom_widget<Message: 'static>(
    widget: impl Widget + Clone + 'static,
    map: impl Fn(WidgetOutput) -> Option<Message> + Send + Sync + 'static,
) -> ViewNode<Message> {
    view_node_from_widget(DynamicWidget::new(widget, map))
}

/// Build a custom widget view with a typed output mapper.
///
/// This is the application-builder companion to
/// [`WidgetMessageMapper::typed`]. Use it when a custom widget emits one
/// concrete output payload with [`WidgetOutput::typed`] or
/// [`WidgetOutput::custom`] and every matching output should become a host
/// message.
pub fn custom_widget_mapped<Output, Message>(
    widget: impl Widget + Clone + 'static,
    map: impl Fn(Output) -> Message + Send + Sync + 'static,
) -> ViewNode<Message>
where
    Output: Clone + Send + Sync + 'static,
    Message: 'static,
{
    view_node_from_widget(MappedWidget::new(widget, WidgetMessageMapper::typed(map)))
}
