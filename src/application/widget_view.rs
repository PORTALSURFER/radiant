/// App-builder context supplied when a widget view becomes a runtime surface node.
///
/// Implementors usually call [`WidgetViewContext::apply_to`] before returning a
/// [`SurfaceNode`]. That keeps generated IDs, explicit sizing, styling, and
/// input-only chrome behavior consistent across built-in and custom widgets.
pub struct WidgetViewContext {
    /// Stable runtime id assigned by the view tree.
    pub id: NodeId,
    sizing: Option<WidgetSizing>,
    style: Option<WidgetStyle>,
    input_only: bool,
    text_wrap: Option<TextWrap>,
}

impl WidgetViewContext {
    /// Explicit sizing set on the enclosing [`ViewNode`], if any.
    pub fn sizing(&self) -> Option<WidgetSizing> {
        self.sizing
    }

    /// Explicit style set on the enclosing [`ViewNode`], if any.
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
        if let Some(wrap) = self.text_wrap
            && let Some(text) = widget.as_any_mut().downcast_mut::<TextWidget>()
        {
            text.wrap = wrap;
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
    /// Default sizing used before callers override the enclosing [`ViewNode`].
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

impl<W, Message> MappedWidget<W, Message> {
    /// Build a mapped widget view.
    pub fn new(widget: W, messages: WidgetMessageMapper<Message>) -> Self {
        Self { widget, messages }
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

impl<Message> DynamicWidget<Message> {
    /// Build a dynamic widget view from a boxed widget object.
    pub fn new(
        widget: impl Widget + Clone + 'static,
        map: impl Fn(WidgetOutput) -> Option<Message> + Send + Sync + 'static,
    ) -> Self {
        Self {
            widget: Box::new(widget),
            map: Arc::new(map),
        }
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
