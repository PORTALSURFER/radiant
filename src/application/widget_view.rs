use crate::{
    layout::{NodeId, Vector2},
    runtime::{SurfaceNode, WidgetMessageMapper},
    widgets::{
        TextAlign, TextBackgroundRole, TextColorRole, TextWrap, Widget, WidgetOutput, WidgetSizing,
        WidgetStyle,
    },
};
use std::sync::Arc;

/// App-builder context supplied when a widget view becomes a runtime surface node.
///
/// Implementors usually call [`WidgetViewContext::apply_to`] before returning a
/// [`SurfaceNode`]. That keeps generated IDs, explicit sizing, styling, and
/// input-only chrome behavior consistent across built-in and custom widgets.
pub struct WidgetViewContext {
    /// Stable runtime id assigned by the view tree.
    pub id: NodeId,
    pub(in crate::application) sizing: Option<WidgetSizing>,
    pub(in crate::application) style: Option<WidgetStyle>,
    pub(in crate::application) input_only: bool,
    pub(in crate::application) text_wrap: Option<TextWrap>,
    pub(in crate::application) text_align: Option<TextAlign>,
    pub(in crate::application) text_color: Option<TextColorRole>,
    pub(in crate::application) text_background: Option<TextBackgroundRole>,
    pub(in crate::application) text_inset: Option<Vector2>,
}

impl WidgetViewContext {
    /// Explicit sizing set on the enclosing [`ViewNode`](crate::application::ViewNode), if any.
    pub fn sizing(&self) -> Option<WidgetSizing> {
        self.sizing
    }

    /// Explicit style set on the enclosing [`ViewNode`](crate::application::ViewNode), if any.
    pub fn style(&self) -> Option<WidgetStyle> {
        self.style
    }

    /// Whether the view requested hit testing without widget chrome.
    pub fn input_only(&self) -> bool {
        self.input_only
    }

    /// Apply common view-node options to a widget before lowering.
    pub fn apply_to(&self, widget: &mut dyn Widget) {
        let common = widget.common_mut();
        common.id = self.id;
        if let Some(sizing) = self.sizing {
            common.sizing = sizing;
        }
        if let Some(style) = self.style {
            common.style = style;
        }
        if self.input_only {
            common.paint.paints_state_layers = false;
        }
        if let Some(wrap) = self.text_wrap {
            widget.set_text_wrap(wrap);
        }
        if let Some(align) = self.text_align {
            widget.set_text_align(align);
        }
        if let Some(color) = self.text_color {
            widget.set_text_color(color);
        }
        if let Some(background) = self.text_background {
            widget.set_text_background(background);
        }
        if let Some(inset) = self.text_inset {
            widget.set_text_inset(inset);
        }
    }
}

/// A widget-shaped application view.
///
/// This is the application-layer companion to [`Widget`]: `Widget` owns runtime
/// input and paint behavior, while `WidgetView` owns how an application widget
/// becomes a message-mapped [`SurfaceNode`]. Any `Widget + Clone + 'static`
/// automatically implements `WidgetView<()>` as a non-emitting leaf. Interactive
/// widgets can use [`MappedWidget`] to bind widget output to host messages.
pub trait WidgetView<Message>: Send + Sync {
    /// Default sizing used before callers override the enclosing
    /// [`ViewNode`](crate::application::ViewNode).
    fn default_sizing(&self) -> WidgetSizing;

    /// Lower this widget view into the runtime surface tree.
    fn into_surface_node(self: Box<Self>, context: WidgetViewContext) -> SurfaceNode<Message>;
}

impl<W, Message> WidgetView<Message> for W
where
    W: Widget + Clone + 'static,
    Message: 'static,
{
    fn default_sizing(&self) -> WidgetSizing {
        self.common().sizing
    }

    fn into_surface_node(mut self: Box<Self>, context: WidgetViewContext) -> SurfaceNode<Message> {
        context.apply_to(self.as_mut());
        SurfaceNode::static_widget(*self)
    }
}

/// A widget plus message mapper for application views.
pub struct MappedWidget<W, Message> {
    widget: W,
    messages: WidgetMessageMapper<Message>,
}

/// Named construction fields for a [`MappedWidget`].
pub struct MappedWidgetParts<W, Message> {
    /// Widget object that owns input and paint behavior.
    pub widget: W,
    /// Mapper that turns widget output into host-defined messages.
    pub messages: WidgetMessageMapper<Message>,
}

impl<W, Message> MappedWidget<W, Message> {
    /// Build a mapped widget view from named parts.
    pub fn from_parts(parts: MappedWidgetParts<W, Message>) -> Self {
        Self {
            widget: parts.widget,
            messages: parts.messages,
        }
    }

    /// Build a mapped widget view.
    pub fn new(widget: W, messages: WidgetMessageMapper<Message>) -> Self {
        Self::from_parts(MappedWidgetParts { widget, messages })
    }
}

impl<W, Message> WidgetView<Message> for MappedWidget<W, Message>
where
    W: Widget + Clone + 'static,
    Message: 'static,
{
    fn default_sizing(&self) -> WidgetSizing {
        self.widget.common().sizing
    }

    fn into_surface_node(mut self: Box<Self>, context: WidgetViewContext) -> SurfaceNode<Message> {
        context.apply_to(&mut self.widget);
        SurfaceNode::widget(self.widget, self.messages)
    }
}

/// A boxed widget plus dynamic output mapper for application views.
pub struct DynamicWidget<Message> {
    widget: Box<dyn Widget>,
    map: Arc<dyn Fn(WidgetOutput) -> Option<Message> + Send + Sync>,
}

/// Named construction fields for a [`DynamicWidget`].
pub struct DynamicWidgetParts<Message> {
    /// Boxed widget object that owns input and paint behavior.
    pub widget: Box<dyn Widget>,
    /// Dynamic mapper that turns widget output into host-defined messages.
    pub map: Arc<dyn Fn(WidgetOutput) -> Option<Message> + Send + Sync>,
}

impl<Message> DynamicWidget<Message> {
    /// Build a dynamic widget view from named parts.
    pub fn from_parts(parts: DynamicWidgetParts<Message>) -> Self {
        Self {
            widget: parts.widget,
            map: parts.map,
        }
    }

    /// Build a dynamic widget view from a boxed widget object.
    pub fn new(
        widget: impl Widget + Clone + 'static,
        map: impl Fn(WidgetOutput) -> Option<Message> + Send + Sync + 'static,
    ) -> Self {
        Self::from_parts(DynamicWidgetParts {
            widget: Box::new(widget),
            map: Arc::new(map),
        })
    }
}

impl<Message> WidgetView<Message> for DynamicWidget<Message>
where
    Message: 'static,
{
    fn default_sizing(&self) -> WidgetSizing {
        self.widget.common().sizing
    }

    fn into_surface_node(mut self: Box<Self>, context: WidgetViewContext) -> SurfaceNode<Message> {
        context.apply_to(self.widget.as_mut());
        let map = self.map;
        SurfaceNode::custom_widget_box(
            self.widget,
            WidgetMessageMapper::dynamic(move |output| map(output)),
        )
    }
}
