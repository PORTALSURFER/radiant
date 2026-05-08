fn view_node_from_widget<Message>(widget: impl WidgetView<Message> + 'static) -> ViewNode<Message> {
    ViewNode {
        kind: ViewNodeKind::Widget(Box::new(widget)),
        id: None,
        key: None,
        sizing: None,
        slot: SlotBehavior::default(),
        padding: Insets::default(),
        align_main: None,
        align_cross: None,
        style: None,
        hoverable: false,
        input_only: false,
        text_wrap: None,
    }
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

/// Build a passive canvas view for retained surfaces that need a generic paint
/// or input slot without host messages.
pub fn canvas<Message: 'static>() -> ViewNode<Message> {
    view_node_from_widget(CanvasWidget::new(0, default_canvas_sizing()))
}

/// Build a retained canvas view with app-owned paint supplied by the app builder.
pub fn retained_canvas(
    key: u64,
    revision: u64,
    dirty_mask: u64,
    volatile: bool,
) -> RetainedCanvasBuilder {
    RetainedCanvasBuilder {
        descriptor: RetainedSurfaceDescriptor {
            key,
            revision,
            dirty_mask,
            volatile,
        },
    }
}

/// Builder for retained canvas views.
pub struct RetainedCanvasBuilder {
    descriptor: RetainedSurfaceDescriptor,
}

impl RetainedCanvasBuilder {
    /// Build a non-emitting retained canvas view.
    pub fn view<Message: 'static>(self) -> ViewNode<Message> {
        view_node_from_widget(CanvasWidget::new(0, default_canvas_sizing()).with_retained_surface(
            self.descriptor,
        ))
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

/// Build a non-interactive raster image view.
pub fn image<Message: 'static>(image: Arc<ImageRgba>) -> ViewNode<Message> {
    let size = Vector2::new(image.width.max(1) as f32, image.height.max(1) as f32);
    view_node_from_widget(ImageWidget::new(0, image, WidgetSizing::fixed(size)))
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

/// Builder for buttons that can emit messages or mutate state directly.
pub struct ButtonBuilder {
    label: String,
    style: Option<WidgetStyle>,
    secondary_click: bool,
    drag: bool,
}

impl ButtonBuilder {
    /// Apply an explicit widget style before binding this button.
    pub fn style(mut self, style: WidgetStyle) -> Self {
        self.style = Some(style);
        self
    }

    /// Use the accent tone and strong prominence.
    pub fn primary(self) -> Self {
        self.style(primary_style())
    }

    /// Use the danger tone for destructive actions.
    pub fn danger(self) -> Self {
        self.style(danger_style())
    }

    /// Use a lower-prominence treatment.
    pub fn subtle(mut self) -> Self {
        let mut style = self.style.unwrap_or_default();
        style.prominence = WidgetProminence::Subtle;
        self.style = Some(style);
        self
    }

    /// Emit secondary/right-click messages from this button.
    pub fn secondary_clicks(mut self) -> Self {
        self.secondary_click = true;
        self
    }

    /// Emit drag lifecycle messages from the button surface.
    pub fn draggable(mut self) -> Self {
        self.drag = true;
        self
    }

    /// Emit one cloned host message when activated.
    pub fn message<Message>(self, message: Message) -> ViewNode<Message>
    where
        Message: Clone + Send + Sync + 'static,
    {
        self.mapped(move |_| message.clone())
    }

    /// Emit a mapped host message when activated.
    pub fn mapped<Message: 'static>(
        self,
        map: impl Fn(crate::widgets::ButtonMessage) -> Message + Send + Sync + 'static,
    ) -> ViewNode<Message> {
        let mut button = ButtonWidget::new(0, &self.label, default_button_sizing(&self.label));
        if self.secondary_click {
            button = button.with_secondary_click();
        }
        if self.drag {
            button = button.with_drag();
        }
        let mut node =
            view_node_from_widget(MappedWidget::new(button, WidgetMessageMapper::button(map)));
        node.style = self.style;
        node
    }

    /// Mutate application state directly when activated.
    pub fn on_click<State: 'static>(
        self,
        apply: impl Fn(&mut State) + Send + Sync + 'static,
    ) -> ViewNode<StateAction<State>> {
        self.message(StateAction::new(apply))
    }

    /// Mutate application state on primary or secondary/right activation.
    pub fn on_click_or_secondary<State: 'static>(
        self,
        primary: impl Fn(&mut State) + Send + Sync + 'static,
        secondary: impl Fn(&mut State) + Send + Sync + 'static,
    ) -> ViewNode<StateAction<State>> {
        let primary = Arc::new(primary);
        let secondary = Arc::new(secondary);
        self.secondary_clicks().mapped(move |message| {
            let primary = Arc::clone(&primary);
            let secondary = Arc::clone(&secondary);
            StateAction::new(move |state| match message {
                crate::widgets::ButtonMessage::Activate => primary(state),
                crate::widgets::ButtonMessage::SecondaryActivate { .. } => secondary(state),
                crate::widgets::ButtonMessage::Drag(_) => {}
            })
        })
    }

    /// Mutate application state on primary activation or secondary/right
    /// activation with pointer position.
    pub fn on_click_or_secondary_at<State: 'static>(
        self,
        primary: impl Fn(&mut State) + Send + Sync + 'static,
        secondary: impl Fn(&mut State, crate::gui::types::Point) + Send + Sync + 'static,
    ) -> ViewNode<StateAction<State>> {
        let primary = Arc::new(primary);
        let secondary = Arc::new(secondary);
        self.secondary_clicks().mapped(move |message| {
            let primary = Arc::clone(&primary);
            let secondary = Arc::clone(&secondary);
            StateAction::new(move |state| match message {
                crate::widgets::ButtonMessage::Activate => primary(state),
                crate::widgets::ButtonMessage::SecondaryActivate { position } => {
                    secondary(state, position);
                }
                crate::widgets::ButtonMessage::Drag(_) => {}
            })
        })
    }

    /// Mutate application state on primary, secondary/right, or drag lifecycle messages.
    pub fn on_click_secondary_or_drag<State: 'static>(
        self,
        primary: impl Fn(&mut State) + Send + Sync + 'static,
        secondary: impl Fn(&mut State) + Send + Sync + 'static,
        drag: impl Fn(&mut State, crate::widgets::DragHandleMessage) + Send + Sync + 'static,
    ) -> ViewNode<StateAction<State>> {
        let primary = Arc::new(primary);
        let secondary = Arc::new(secondary);
        let drag = Arc::new(drag);
        self.secondary_clicks().draggable().mapped(move |message| {
            let primary = Arc::clone(&primary);
            let secondary = Arc::clone(&secondary);
            let drag = Arc::clone(&drag);
            StateAction::new(move |state| match message {
                crate::widgets::ButtonMessage::Activate => primary(state),
                crate::widgets::ButtonMessage::SecondaryActivate { .. } => secondary(state),
                crate::widgets::ButtonMessage::Drag(message) => drag(state, message),
            })
        })
    }

    /// Mutate application state on primary, secondary/right with pointer
    /// position, or drag lifecycle messages.
    pub fn on_click_secondary_at_or_drag<State: 'static>(
        self,
        primary: impl Fn(&mut State) + Send + Sync + 'static,
        secondary: impl Fn(&mut State, crate::gui::types::Point) + Send + Sync + 'static,
        drag: impl Fn(&mut State, crate::widgets::DragHandleMessage) + Send + Sync + 'static,
    ) -> ViewNode<StateAction<State>> {
        let primary = Arc::new(primary);
        let secondary = Arc::new(secondary);
        let drag = Arc::new(drag);
        self.secondary_clicks().draggable().mapped(move |message| {
            let primary = Arc::clone(&primary);
            let secondary = Arc::clone(&secondary);
            let drag = Arc::clone(&drag);
            StateAction::new(move |state| match message {
                crate::widgets::ButtonMessage::Activate => primary(state),
                crate::widgets::ButtonMessage::SecondaryActivate { position } => {
                    secondary(state, position);
                }
                crate::widgets::ButtonMessage::Drag(message) => drag(state, message),
            })
        })
    }
}

/// Build a button.
pub fn button(label: impl Into<String>) -> ButtonBuilder {
    ButtonBuilder {
        label: label.into(),
        style: None,
        secondary_click: false,
        drag: false,
    }
}

/// Build a button that emits one cloned host message when activated.
pub fn button_message<Message>(label: impl Into<String>, message: Message) -> ViewNode<Message>
where
    Message: Clone + Send + Sync + 'static,
{
    button(label).message(message)
}

/// Build a button with a custom widget-message mapper.
pub fn button_mapped<Message: 'static>(
    label: impl Into<String>,
    map: impl Fn(crate::widgets::ButtonMessage) -> Message + Send + Sync + 'static,
) -> ViewNode<Message> {
    button(label).mapped(map)
}

/// Builder for compact drag handles that can emit messages or mutate state directly.
pub struct DragHandleBuilder;

impl DragHandleBuilder {
    /// Emit a mapped host message for drag lifecycle events.
    pub fn mapped<Message: 'static>(
        self,
        map: impl Fn(crate::widgets::DragHandleMessage) -> Message + Send + Sync + 'static,
    ) -> ViewNode<Message> {
        view_node_from_widget(MappedWidget::new(
            DragHandleWidget::new(0, default_drag_handle_sizing()),
            WidgetMessageMapper::drag_handle(map),
        ))
    }

    /// Mutate application state directly when the handle is dragged.
    pub fn on_drag<State: 'static>(
        self,
        apply: impl Fn(&mut State, crate::widgets::DragHandleMessage) + Send + Sync + 'static,
    ) -> ViewNode<StateAction<State>> {
        let apply = Arc::new(apply);
        self.mapped(move |message| {
            let apply = Arc::clone(&apply);
            StateAction::new(move |state| apply(state, message))
        })
    }
}

/// Build a compact drag handle for pointer-driven reordering.
pub fn drag_handle() -> DragHandleBuilder {
    DragHandleBuilder
}

/// Build a drag handle with a custom widget-message mapper.
pub fn drag_handle_mapped<Message: 'static>(
    map: impl Fn(crate::widgets::DragHandleMessage) -> Message + Send + Sync + 'static,
) -> ViewNode<Message> {
    drag_handle().mapped(map)
}

/// Builder for toggles that can emit messages or mutate state directly.
pub struct ToggleBuilder {
    label: String,
    checked: bool,
    compact: bool,
    style: Option<WidgetStyle>,
}

impl ToggleBuilder {
    /// Apply an explicit widget style before binding this toggle.
    pub fn style(mut self, style: WidgetStyle) -> Self {
        self.style = Some(style);
        self
    }

    /// Use the accent tone and strong prominence.
    pub fn primary(self) -> Self {
        self.style(primary_style())
    }

    /// Use the danger tone.
    pub fn danger(self) -> Self {
        self.style(danger_style())
    }

    /// Use a lower-prominence treatment.
    pub fn subtle(mut self) -> Self {
        let mut style = self.style.unwrap_or_default();
        style.prominence = WidgetProminence::Subtle;
        self.style = Some(style);
        self
    }

    /// Emit a host message mapped from checked state.
    pub fn message<Message: 'static>(
        self,
        map: impl Fn(bool) -> Message + Send + Sync + 'static,
    ) -> ViewNode<Message> {
        let mut node = view_node_from_widget(MappedWidget::new(
            ToggleWidget::new(
                0,
                &self.label,
                default_toggle_sizing(&self.label, self.compact),
            )
            .with_checked(self.checked),
            WidgetMessageMapper::toggle(move |message| match message {
                crate::widgets::ToggleMessage::ValueChanged { checked } => map(checked),
            }),
        ));
        node.style = self.style;
        node
    }

    /// Mutate application state directly when checked state changes.
    pub fn on_change<State: 'static>(
        self,
        apply: impl Fn(&mut State, bool) + Send + Sync + 'static,
    ) -> ViewNode<StateAction<State>> {
        let apply = Arc::new(apply);
        self.message(move |checked| {
            let apply = Arc::clone(&apply);
            StateAction::new(move |state| apply(state, checked))
        })
    }
}

/// Build a toggle.
pub fn toggle(label: impl Into<String>, checked: bool) -> ToggleBuilder {
    ToggleBuilder {
        label: label.into(),
        checked,
        compact: false,
        style: None,
    }
}

/// Build a compact checkbox.
pub fn checkbox(checked: bool) -> ToggleBuilder {
    ToggleBuilder {
        label: String::new(),
        checked,
        compact: true,
        style: None,
    }
}

/// Build a toggle that maps value changes by checked state.
pub fn toggle_mapped<Message: 'static>(
    label: impl Into<String>,
    checked: bool,
    map: impl Fn(bool) -> Message + Send + Sync + 'static,
) -> ViewNode<Message> {
    toggle(label, checked).message(map)
}

/// Builder for text inputs that can emit messages or mutate state directly.
pub struct TextInputBuilder {
    value: String,
    placeholder: Option<String>,
    style: Option<WidgetStyle>,
}

impl TextInputBuilder {
    /// Show placeholder text when the input value is empty.
    pub fn placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.placeholder = Some(placeholder.into());
        self
    }

    /// Apply an explicit widget style before binding this text input.
    pub fn style(mut self, style: WidgetStyle) -> Self {
        self.style = Some(style);
        self
    }

    /// Use a lower-prominence treatment.
    pub fn subtle(mut self) -> Self {
        let mut style = self.style.unwrap_or_default();
        style.prominence = WidgetProminence::Subtle;
        self.style = Some(style);
        self
    }

    /// Emit a host message mapped from the input value.
    pub fn message<Message: 'static>(
        self,
        map: impl Fn(String) -> Message + Send + Sync + 'static,
    ) -> ViewNode<Message> {
        let mut input = TextInputWidget::new(0, self.value, default_text_input_sizing());
        input.props.placeholder = self.placeholder;
        let mut node = view_node_from_widget(MappedWidget::new(
            input,
            WidgetMessageMapper::text_input(move |message| match message {
                crate::widgets::TextInputMessage::Changed { value }
                | crate::widgets::TextInputMessage::Submitted { value } => map(value),
            }),
        ));
        node.style = self.style;
        node
    }

    /// Mutate application state directly when the input value changes.
    pub fn on_change<State: 'static>(
        self,
        apply: impl Fn(&mut State, String) + Send + Sync + 'static,
    ) -> ViewNode<StateAction<State>> {
        let apply = Arc::new(apply);
        self.message(move |value| {
            let apply = Arc::clone(&apply);
            StateAction::new(move |state| apply(state, value.clone()))
        })
    }

    /// Bind this input to a mutable `String` field on application state.
    pub fn bind<State: 'static>(
        self,
        field: impl for<'a> Fn(&'a mut State) -> &'a mut String + Send + Sync + 'static,
    ) -> ViewNode<StateAction<State>> {
        self.on_change(move |state, value| *field(state) = value)
    }

    /// Bind edits to a mutable `String` field and run a state callback on submit.
    pub fn bind_submit<State: 'static>(
        self,
        field: impl for<'a> Fn(&'a mut State) -> &'a mut String + Send + Sync + 'static,
        submit: impl Fn(&mut State) + Send + Sync + 'static,
    ) -> ViewNode<StateAction<State>> {
        let field = Arc::new(field);
        let submit = Arc::new(submit);
        let mut input = TextInputWidget::new(0, self.value, default_text_input_sizing());
        input.props.placeholder = self.placeholder;
        let mut node = view_node_from_widget(MappedWidget::new(
            input,
            WidgetMessageMapper::text_input(move |message| {
                let field = Arc::clone(&field);
                let submit = Arc::clone(&submit);
                StateAction::new(move |state| match &message {
                    crate::widgets::TextInputMessage::Changed { value } => {
                        *field(state) = value.clone();
                    }
                    crate::widgets::TextInputMessage::Submitted { value } => {
                        *field(state) = value.clone();
                        submit(state);
                    }
                })
            }),
        ));
        node.style = self.style;
        node
    }
}

/// Build a single-line text input.
pub fn text_input(value: impl Into<String>) -> TextInputBuilder {
    TextInputBuilder {
        value: value.into(),
        placeholder: None,
        style: None,
    }
}

/// Build a single-line text input that maps edits and submissions by value.
pub fn text_input_mapped<Message: 'static>(
    value: impl Into<String>,
    map: impl Fn(String) -> Message + Send + Sync + 'static,
) -> ViewNode<Message> {
    text_input(value).message(map)
}

/// Build a row container with fill-slot children.
pub fn row<Message>(children: impl IntoIterator<Item = ViewNode<Message>>) -> ViewNode<Message> {
    ViewNode {
        kind: ViewNodeKind::Row {
            spacing: 8.0,
            children: children.into_iter().collect(),
        },
        id: None,
        key: None,
        sizing: None,
        slot: SlotBehavior::default(),
        padding: Insets::default(),
        align_main: None,
        align_cross: None,
        style: None,
        hoverable: false,
        input_only: false,
        text_wrap: None,
    }
}

/// Build a keyed row container with fill-slot children.
pub fn row_key<Message>(
    key: impl ToString,
    children: impl IntoIterator<Item = ViewNode<Message>>,
) -> ViewNode<Message> {
    row(children).key(key)
}

/// Build a column container with fill-slot children.
pub fn column<Message>(children: impl IntoIterator<Item = ViewNode<Message>>) -> ViewNode<Message> {
    ViewNode {
        kind: ViewNodeKind::Column {
            spacing: 6.0,
            children: children.into_iter().collect(),
        },
        id: None,
        key: None,
        sizing: None,
        slot: SlotBehavior::default(),
        padding: Insets::default(),
        align_main: None,
        align_cross: None,
        style: None,
        hoverable: false,
        input_only: false,
        text_wrap: None,
    }
}

/// Build a keyed column container with fill-slot children.
pub fn column_key<Message>(
    key: impl ToString,
    children: impl IntoIterator<Item = ViewNode<Message>>,
) -> ViewNode<Message> {
    column(children).key(key)
}

/// Build a stack container that overlays children in paint order.
pub fn stack<Message>(children: impl IntoIterator<Item = ViewNode<Message>>) -> ViewNode<Message> {
    ViewNode {
        kind: ViewNodeKind::Stack {
            children: children.into_iter().collect(),
        },
        id: None,
        key: None,
        sizing: None,
        slot: SlotBehavior::default(),
        padding: Insets::default(),
        align_main: None,
        align_cross: None,
        style: None,
        hoverable: false,
        input_only: false,
        text_wrap: None,
    }
}

/// Build a floating overlay panel in surface coordinates.
pub fn overlay_panel<Message>(
    label: impl Into<String>,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
) -> ViewNode<Message> {
    ViewNode {
        kind: ViewNodeKind::OverlayPanel {
            rect: crate::gui::types::Rect::from_min_size(
                crate::gui::types::Point::new(x, y),
                Vector2::new(width, height),
            ),
            label: Some(label.into()),
        },
        id: None,
        key: None,
        sizing: None,
        slot: SlotBehavior::default(),
        padding: Insets::default(),
        align_main: None,
        align_cross: None,
        style: None,
        hoverable: false,
        input_only: false,
        text_wrap: None,
    }
}

/// Build a floating drop marker in surface coordinates.
pub fn drop_marker<Message>(x: f32, y: f32, width: f32, height: f32) -> ViewNode<Message> {
    ViewNode {
        kind: ViewNodeKind::OverlayPanel {
            rect: crate::gui::types::Rect::from_min_size(
                crate::gui::types::Point::new(x, y),
                Vector2::new(width, height),
            ),
            label: None,
        },
        id: None,
        key: None,
        sizing: None,
        slot: SlotBehavior::default(),
        padding: Insets::default(),
        align_main: None,
        align_cross: None,
        style: Some(primary_style()),
        hoverable: false,
        input_only: false,
        text_wrap: None,
    }
}

/// Build a scroll viewport around one child view.
pub fn scroll<Message>(child: ViewNode<Message>) -> ViewNode<Message> {
    ViewNode {
        kind: ViewNodeKind::Scroll {
            child: Box::new(child),
        },
        id: None,
        key: None,
        sizing: None,
        slot: SlotBehavior::default(),
        padding: Insets::default(),
        align_main: None,
        align_cross: None,
        style: None,
        hoverable: false,
        input_only: false,
        text_wrap: None,
    }
}

/// Build a scroll viewport containing a column projected from an iterator.
pub fn scroll_column<Message, Item>(
    items: impl IntoIterator<Item = Item>,
    project: impl FnMut(Item) -> ViewNode<Message>,
) -> ViewNode<Message> {
    scroll(column(items.into_iter().map(project).collect::<Vec<_>>()))
}

/// Build a scrollable vertical list with stable intrinsic-height rows.
pub fn list<Message, Item>(
    items: impl IntoIterator<Item = Item>,
    project: impl FnMut(Item) -> ViewNode<Message>,
) -> ViewNode<Message> {
    scroll_column(items, project)
        .style(WidgetStyle::default())
        .fill_height()
}

/// Build a keyed list row with full-width, fixed-height defaults.
pub fn list_row<Message>(
    key: impl ToString,
    children: impl IntoIterator<Item = ViewNode<Message>>,
) -> ViewNode<Message> {
    row_key(key, children)
        .style(WidgetStyle::default())
        .hoverable()
        .fill_width()
        .height(52.0)
        .padding_x(18.0)
        .padding_y(10.0)
        .spacing(16.0)
}

fn default_text_sizing() -> WidgetSizing {
    WidgetSizing::fixed(Vector2::new(160.0, 24.0)).with_baseline(17.0)
}

fn default_button_sizing(label: &str) -> WidgetSizing {
    let width = (label.chars().count() as f32 * 9.0 + 36.0).clamp(88.0, 260.0);
    WidgetSizing::fixed(Vector2::new(width, 36.0)).with_baseline(23.0)
}

fn default_drag_handle_sizing() -> WidgetSizing {
    WidgetSizing::fixed(Vector2::new(24.0, 24.0))
}

fn default_toggle_sizing(label: &str, compact: bool) -> WidgetSizing {
    if compact {
        return WidgetSizing::fixed(Vector2::new(22.0, 22.0)).with_baseline(16.0);
    }
    let width = (label.chars().count() as f32 * 8.0 + 52.0).clamp(96.0, 280.0);
    WidgetSizing::fixed(Vector2::new(width, 30.0))
}

fn default_text_input_sizing() -> WidgetSizing {
    WidgetSizing::new(Vector2::new(180.0, 42.0), Vector2::new(280.0, 42.0)).with_baseline(26.0)
}

fn default_canvas_sizing() -> WidgetSizing {
    WidgetSizing::fixed(Vector2::new(1.0, 1.0))
}

fn primary_style() -> WidgetStyle {
    WidgetStyle {
        tone: WidgetTone::Accent,
        prominence: WidgetProminence::Strong,
    }
}

fn danger_style() -> WidgetStyle {
    WidgetStyle {
        tone: WidgetTone::Danger,
        prominence: WidgetProminence::Normal,
    }
}
