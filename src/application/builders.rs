use crate::{
    application::{DynamicWidget, MappedWidget, ViewNode, ViewNodeKind, WidgetView},
    gui::types::ImageRgba,
    layout::Vector2,
    runtime::{GpuSurfaceContent, WidgetMessageMapper},
    widgets::{
        ButtonWidget, CanvasWidget, CardWidget, GpuSurfaceMessage, GpuSurfaceWidget, ImageWidget,
        TextInputWidget, TextWidget, ToggleWidget, Widget, WidgetOutput, WidgetProminence,
        WidgetSizing, WidgetStyle, WidgetTone,
    },
};
use std::sync::Arc;

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
    view_node_from_widget(TextWidget::new(0, value, default_text_sizing()))
}

/// Build a passive button view for retained surfaces that need button chrome
/// without host messages.
pub fn passive_button<Message: 'static>(label: impl Into<String>) -> ViewNode<Message> {
    view_node_from_widget(ButtonWidget::new(0, label, default_button_sizing("")))
}

/// Build a passive toggle view for retained surfaces that need toggle chrome
/// without host messages.
pub fn passive_toggle<Message: 'static>(
    label: impl Into<String>,
    checked: bool,
) -> ViewNode<Message> {
    view_node_from_widget(
        ToggleWidget::new(0, label, default_toggle_sizing("", true)).with_checked(checked),
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
        input.props.placeholder = Some(placeholder);
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
    view_node_from_widget(GpuSurfaceWidget::new(
        0,
        default_gpu_surface_sizing(),
        key,
        revision,
        content,
    ))
}

/// Build an input-emitting retained GPU surface view with generated identity.
///
/// This keeps GPU-heavy widgets on the same application message path as
/// standard widgets while leaving plain [`gpu_surface`] views passive.
pub fn gpu_surface_input<Message: 'static>(
    key: u64,
    revision: u64,
    content: GpuSurfaceContent,
    map: impl Fn(crate::widgets::WidgetInput) -> Message + Send + Sync + 'static,
) -> ViewNode<Message> {
    view_node_from_widget(MappedWidget::new(
        GpuSurfaceWidget::new(0, default_gpu_surface_sizing(), key, revision, content)
            .with_input_events(true),
        WidgetMessageMapper::typed(move |message: GpuSurfaceMessage| match message {
            GpuSurfaceMessage::Input { input } => map(input),
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

fn default_text_sizing() -> WidgetSizing {
    WidgetSizing::fixed(Vector2::new(160.0, 24.0)).with_baseline(17.0)
}

pub(in crate::application) fn default_button_sizing(label: &str) -> WidgetSizing {
    let width = (label.chars().count() as f32 * 9.0 + 36.0).clamp(88.0, 260.0);
    WidgetSizing::fixed(Vector2::new(width, 36.0)).with_baseline(23.0)
}

pub(in crate::application) fn default_drag_handle_sizing() -> WidgetSizing {
    WidgetSizing::fixed(Vector2::new(24.0, 24.0))
}

pub(in crate::application) fn default_badge_sizing(label: &str) -> WidgetSizing {
    let width = (label.chars().count() as f32 * 8.0 + 24.0).clamp(56.0, 180.0);
    WidgetSizing::fixed(Vector2::new(width, 24.0)).with_baseline(17.0)
}

pub(in crate::application) fn default_selectable_sizing(label: &str) -> WidgetSizing {
    let width = (label.chars().count() as f32 * 8.0 + 28.0).clamp(92.0, 260.0);
    WidgetSizing::fixed(Vector2::new(width, 30.0)).with_baseline(20.0)
}

pub(in crate::application) fn default_toggle_sizing(label: &str, compact: bool) -> WidgetSizing {
    if compact {
        return WidgetSizing::fixed(Vector2::new(22.0, 22.0)).with_baseline(16.0);
    }
    let width = (label.chars().count() as f32 * 8.0 + 52.0).clamp(96.0, 280.0);
    WidgetSizing::fixed(Vector2::new(width, 30.0))
}

pub(in crate::application) fn default_text_input_sizing() -> WidgetSizing {
    WidgetSizing::new(Vector2::new(180.0, 42.0), Vector2::new(280.0, 42.0)).with_baseline(26.0)
}

pub(in crate::application) fn default_canvas_sizing() -> WidgetSizing {
    WidgetSizing::fixed(Vector2::new(1.0, 1.0))
}

fn default_card_sizing() -> WidgetSizing {
    WidgetSizing::new(Vector2::new(120.0, 72.0), Vector2::new(220.0, 120.0))
}

fn default_gpu_surface_sizing() -> WidgetSizing {
    WidgetSizing::new(Vector2::new(160.0, 90.0), Vector2::new(320.0, 180.0))
}

pub(in crate::application) fn primary_style() -> WidgetStyle {
    WidgetStyle {
        tone: WidgetTone::Accent,
        prominence: WidgetProminence::Strong,
    }
}

pub(in crate::application) fn danger_style() -> WidgetStyle {
    WidgetStyle {
        tone: WidgetTone::Danger,
        prominence: WidgetProminence::Normal,
    }
}
