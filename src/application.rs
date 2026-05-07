//! Readable Radiant application and view builders.
//!
//! This module is a convenience layer over [`crate::runtime`]. It generates
//! deterministic widget ids, supplies default widget sizing, and lowers every
//! view into the existing [`UiSurface`](crate::runtime::UiSurface) tree.

use crate::{
    gui_runtime::NativeRunOptions,
    layout::{
        ContainerKind, ContainerPolicy, Insets, NodeId, SizeModeCross, SizeModeMain, SlotParams,
        Vector2,
    },
    runtime::{
        Command, RuntimeBridge, SurfaceChild, SurfaceNode, UiSurface, WidgetMessageMapper,
        declarative_command_runtime_bridge, run_native_vello_runtime,
    },
    widgets::{
        ButtonWidget, DragHandleWidget, TextInputWidget, TextWidget, TextWrap, ToggleWidget,
        Widget, WidgetOutput, WidgetProminence, WidgetSizing, WidgetSpec, WidgetStyle, WidgetTone,
    },
};
use std::{collections::HashSet, marker::PhantomData, sync::Arc};

const ROOT_KEY_SCOPE: u64 = 0xcbf2_9ce4_8422_2325;
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

/// Result type used by native launch helpers.
pub type Result<T = ()> = std::result::Result<T, String>;

/// Application view node type used by builder functions.
pub type View<Message = ()> = ViewNode<Message>;

/// Application view node type for direct state-callback apps.
pub type StateView<State> = View<StateAction<State>>;

/// A state mutation emitted by application builders with direct callbacks.
pub struct StateAction<State> {
    apply: Arc<dyn Fn(&mut State) + Send + Sync>,
}

impl<State> Clone for StateAction<State> {
    fn clone(&self) -> Self {
        Self {
            apply: Arc::clone(&self.apply),
        }
    }
}

trait OptionalBaseline {
    fn with_optional_baseline(self, baseline: Option<f32>) -> Self;
}

impl OptionalBaseline for WidgetSizing {
    fn with_optional_baseline(self, baseline: Option<f32>) -> Self {
        if let Some(baseline) = baseline {
            self.with_baseline(baseline)
        } else {
            self
        }
    }
}

impl<State> StateAction<State> {
    fn new(apply: impl Fn(&mut State) + Send + Sync + 'static) -> Self {
        Self {
            apply: Arc::new(apply),
        }
    }

    fn run(&self, state: &mut State) {
        (self.apply)(state);
    }
}

/// Build a native window launcher for a simple Radiant view.
pub fn window(title: impl Into<String>) -> WindowBuilder {
    WindowBuilder::new(title)
}

/// Build a stateful app launcher over the existing command runtime bridge.
pub fn app<State>(state: State) -> StatefulAppBuilder<State> {
    StatefulAppBuilder::new(state)
}

/// Converts application view values into the existing runtime surface.
pub trait IntoView<Message> {
    /// Lower this value into a runtime surface node.
    fn into_node(self) -> SurfaceNode<Message>;

    /// Lower this value into a top-level runtime surface.
    fn into_surface(self) -> UiSurface<Message>
    where
        Self: Sized,
    {
        UiSurface::new(self.into_node())
    }
}

impl<Message> IntoView<Message> for SurfaceNode<Message> {
    fn into_node(self) -> SurfaceNode<Message> {
        self
    }
}

impl<Message> IntoView<Message> for UiSurface<Message> {
    fn into_node(self) -> SurfaceNode<Message> {
        self.into_root()
    }

    fn into_surface(self) -> UiSurface<Message> {
        self
    }
}

/// Builder for no-state native windows.
pub struct WindowBuilder {
    options: NativeRunOptions,
}

impl WindowBuilder {
    fn new(title: impl Into<String>) -> Self {
        Self {
            options: NativeRunOptions {
                title: title.into(),
                ..NativeRunOptions::default()
            },
        }
    }

    /// Set the initial logical window size.
    pub fn size(mut self, width: u32, height: u32) -> Self {
        self.options.inner_size = Some([width as f32, height as f32]);
        self
    }

    /// Set the minimum logical window size.
    pub fn min_size(mut self, width: u32, height: u32) -> Self {
        self.options.min_inner_size = Some([width as f32, height as f32]);
        self
    }

    /// Set the full native runtime options, preserving this builder as a thin adapter.
    pub fn options(mut self, options: NativeRunOptions) -> Self {
        self.options = options;
        self
    }

    /// Run one static view through the native Vello runtime.
    pub fn run<View>(self, view: View) -> Result
    where
        View: IntoView<()> + 'static,
    {
        let surface = Arc::new(view.into_surface());
        let bridge = declarative_command_runtime_bridge(
            surface,
            |surface| Arc::clone(surface),
            |_, ()| Command::none(),
        );
        run_native_vello_runtime(self.options, bridge)
    }
}

/// Initial builder for simple stateful Radiant apps.
pub struct StatefulAppBuilder<State> {
    state: State,
    options: NativeRunOptions,
}

impl<State> StatefulAppBuilder<State> {
    fn new(state: State) -> Self {
        Self {
            state,
            options: NativeRunOptions::default(),
        }
    }

    /// Set the native window title.
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.options.title = title.into();
        self
    }

    /// Set the initial logical window size.
    pub fn size(mut self, width: u32, height: u32) -> Self {
        self.options.inner_size = Some([width as f32, height as f32]);
        self
    }

    /// Set the minimum logical window size.
    pub fn min_size(mut self, width: u32, height: u32) -> Self {
        self.options.min_inner_size = Some([width as f32, height as f32]);
        self
    }

    /// Set the full native runtime options for apps that need explicit launch control.
    pub fn options(mut self, options: NativeRunOptions) -> Self {
        self.options = options;
        self
    }

    /// Attach a state projection closure.
    pub fn view<Message, Project, View>(
        self,
        project: Project,
    ) -> StatefulAppWithView<State, Message, Project, View>
    where
        Project: FnMut(&mut State) -> View,
        View: IntoView<Message>,
    {
        StatefulAppWithView {
            state: self.state,
            options: self.options,
            project,
            _message: PhantomData,
            _view: PhantomData,
        }
    }
}

/// Stateful app builder after a view projection has been supplied.
pub struct StatefulAppWithView<State, Message, Project, View> {
    state: State,
    options: NativeRunOptions,
    project: Project,
    _message: PhantomData<Message>,
    _view: PhantomData<View>,
}

impl<State, Message, Project, View> StatefulAppWithView<State, Message, Project, View>
where
    Project: FnMut(&mut State) -> View + 'static,
    View: IntoView<Message> + 'static,
    Message: 'static,
    State: 'static,
{
    /// Attach a reducer that mutates app state and requests a repaint.
    pub fn update<Update>(
        self,
        mut update: Update,
    ) -> RunnableStatefulApp<
        State,
        Message,
        Project,
        impl FnMut(&mut State, Message) -> Command<Message>,
        View,
    >
    where
        Update: FnMut(&mut State, Message) + 'static,
    {
        self.update_command(move |state, message| {
            update(state, message);
            Command::request_repaint()
        })
    }

    /// Attach a reducer that returns runtime-visible commands.
    pub fn update_command<Update>(
        self,
        update: Update,
    ) -> RunnableStatefulApp<State, Message, Project, Update, View>
    where
        Update: FnMut(&mut State, Message) -> Command<Message> + 'static,
    {
        RunnableStatefulApp {
            state: self.state,
            options: self.options,
            project: self.project,
            update,
            _message: PhantomData,
            _view: PhantomData,
        }
    }
}

impl<State, Project, View> StatefulAppWithView<State, StateAction<State>, Project, View>
where
    Project: FnMut(&mut State) -> View + 'static,
    View: IntoView<StateAction<State>> + 'static,
    State: 'static,
{
    /// Run this direct-callback app through the native Vello runtime.
    pub fn run(self) -> Result {
        let options = self.options.clone();
        run_native_vello_runtime(options, self.into_bridge())
    }

    /// Lower this direct-callback app into the existing runtime bridge without opening a window.
    pub fn into_bridge(self) -> impl RuntimeBridge<StateAction<State>> {
        let mut project = self.project;
        declarative_command_runtime_bridge(
            self.state,
            move |state| Arc::new(project(state).into_surface()),
            |state, action| {
                action.run(state);
                Command::request_repaint()
            },
        )
    }
}

/// Runnable stateful app builder.
pub struct RunnableStatefulApp<State, Message, Project, Update, View> {
    state: State,
    options: NativeRunOptions,
    project: Project,
    update: Update,
    _message: PhantomData<Message>,
    _view: PhantomData<View>,
}

impl<State, Message, Project, Update, View>
    RunnableStatefulApp<State, Message, Project, Update, View>
where
    Project: FnMut(&mut State) -> View + 'static,
    Update: FnMut(&mut State, Message) -> Command<Message> + 'static,
    View: IntoView<Message> + 'static,
    Message: 'static,
    State: 'static,
{
    /// Run this app through the native Vello runtime.
    pub fn run(self) -> Result {
        let options = self.options.clone();
        run_native_vello_runtime(options, self.into_bridge())
    }

    /// Lower this app into the existing runtime bridge without opening a window.
    pub fn into_bridge(self) -> impl RuntimeBridge<Message> {
        let mut project = self.project;
        declarative_command_runtime_bridge(
            self.state,
            move |state| Arc::new(project(state).into_surface()),
            self.update,
        )
    }
}

/// Application view node with generated identity and default sizing.
pub struct ViewNode<Message> {
    kind: ViewNodeKind<Message>,
    id: Option<NodeId>,
    key: Option<String>,
    sizing: Option<WidgetSizing>,
    slot: SlotBehavior,
    padding: Insets,
    style: Option<WidgetStyle>,
    hoverable: bool,
    input_only: bool,
    text_wrap: Option<TextWrap>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct SlotBehavior {
    width: AxisSlotBehavior,
    height: AxisSlotBehavior,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
enum AxisSlotBehavior {
    #[default]
    Default,
    Intrinsic,
    Fill(f32),
    Fixed(f32),
}

enum ViewNodeKind<Message> {
    Runtime(SurfaceNode<Message>),
    Text(String),
    ButtonMapped {
        label: String,
        map: Arc<dyn Fn(crate::widgets::ButtonMessage) -> Message + Send + Sync>,
    },
    DragHandle {
        map: Arc<dyn Fn(crate::widgets::DragHandleMessage) -> Message + Send + Sync>,
    },
    Toggle {
        label: String,
        checked: bool,
        compact: bool,
        map: Arc<dyn Fn(bool) -> Message + Send + Sync>,
    },
    TextInput {
        value: String,
        placeholder: Option<String>,
        map: Arc<dyn Fn(String) -> Message + Send + Sync>,
    },
    CustomWidget {
        widget: Box<dyn Widget>,
        map: Arc<dyn Fn(WidgetOutput) -> Option<Message> + Send + Sync>,
    },
    Row {
        spacing: f32,
        children: Vec<ViewNode<Message>>,
    },
    Column {
        spacing: f32,
        children: Vec<ViewNode<Message>>,
    },
    Scroll {
        child: Box<ViewNode<Message>>,
    },
    Stack {
        children: Vec<ViewNode<Message>>,
    },
    OverlayPanel {
        rect: crate::gui::types::Rect,
        label: Option<String>,
    },
}

impl<Message> ViewNode<Message> {
    /// Use an explicit stable id instead of the generated structural id.
    pub fn id(mut self, id: NodeId) -> Self {
        self.id = Some(id);
        self.key = None;
        self
    }

    /// Use a scoped stable key instead of a numeric id.
    ///
    /// Child keys are scoped by their keyed or explicitly identified parent, so repeated rows can
    /// use names such as `"done"` or `"delete"` without colliding with sibling rows.
    pub fn key(mut self, key: impl ToString) -> Self {
        self.key = Some(key.to_string());
        self
    }

    /// Use explicit widget sizing instead of the generated default.
    pub fn sizing(mut self, sizing: WidgetSizing) -> Self {
        self.sizing = Some(sizing);
        self
    }

    /// Use explicit fixed widget sizing instead of the generated default.
    pub fn size(self, width: f32, height: f32) -> Self {
        self.sizing(WidgetSizing::fixed(Vector2::new(width, height)))
    }

    /// Use explicit fixed widget sizing instead of the generated default.
    pub fn fixed(self, width: f32, height: f32) -> Self {
        self.size(width, height)
    }

    /// Fill remaining space on the parent main axis and stretch on the cross axis.
    pub fn fill(mut self) -> Self {
        self.slot.width = AxisSlotBehavior::Fill(1.0);
        self.slot.height = AxisSlotBehavior::Fill(1.0);
        self
    }

    /// Fill remaining horizontal space in the parent layout.
    pub fn fill_width(mut self) -> Self {
        self.slot.width = AxisSlotBehavior::Fill(1.0);
        self
    }

    /// Fill remaining vertical space in the parent layout.
    pub fn fill_height(mut self) -> Self {
        self.slot.height = AxisSlotBehavior::Fill(1.0);
        self
    }

    /// Fill remaining main-axis space with the provided weight.
    pub fn grow(mut self, weight: f32) -> Self {
        self.slot.width = AxisSlotBehavior::Fill(weight);
        self.slot.height = AxisSlotBehavior::Fill(weight);
        self
    }

    /// Use intrinsic parent slot sizing on both axes.
    pub fn intrinsic(mut self) -> Self {
        self.slot.width = AxisSlotBehavior::Intrinsic;
        self.slot.height = AxisSlotBehavior::Intrinsic;
        self
    }

    /// Use a fixed parent slot width.
    pub fn width(mut self, width: f32) -> Self {
        self.slot.width = AxisSlotBehavior::Fixed(width);
        self
    }

    /// Use a fixed parent slot height.
    pub fn height(mut self, height: f32) -> Self {
        self.slot.height = AxisSlotBehavior::Fixed(height);
        self
    }

    /// Set the minimum widget size while preserving any existing preferred size.
    pub fn min_size(mut self, width: f32, height: f32) -> Self {
        let min = Vector2::new(width, height);
        let preferred = self.sizing.map(|sizing| sizing.preferred).unwrap_or(min);
        let baseline = self.sizing.and_then(|sizing| sizing.baseline);
        self.sizing = Some(WidgetSizing::new(min, preferred).with_optional_baseline(baseline));
        self
    }

    /// Set the preferred widget size while preserving any existing minimum size.
    pub fn preferred_size(mut self, width: f32, height: f32) -> Self {
        let preferred = Vector2::new(width, height);
        let min = self.sizing.map(|sizing| sizing.min).unwrap_or(preferred);
        let baseline = self.sizing.and_then(|sizing| sizing.baseline);
        self.sizing = Some(WidgetSizing::new(min, preferred).with_optional_baseline(baseline));
        self
    }

    /// Set the widget text baseline.
    pub fn baseline(mut self, baseline: f32) -> Self {
        let sizing = self.sizing.unwrap_or_else(|| match &self.kind {
            ViewNodeKind::Text(_) => default_text_sizing(),
            ViewNodeKind::ButtonMapped { label, .. } => default_button_sizing(label),
            ViewNodeKind::DragHandle { .. } => default_drag_handle_sizing(),
            ViewNodeKind::Toggle { label, compact, .. } => default_toggle_sizing(label, *compact),
            ViewNodeKind::TextInput { .. } => default_text_input_sizing(),
            ViewNodeKind::CustomWidget { widget, .. } => widget.common().sizing,
            _ => WidgetSizing::fixed(Vector2::new(0.0, 0.0)),
        });
        self.sizing = Some(sizing.with_baseline(baseline));
        self
    }

    /// Apply equal content padding when this node is a container.
    pub fn padding(mut self, padding: f32) -> Self {
        self.padding = Insets::all(padding.max(0.0));
        self
    }

    /// Apply horizontal content padding when this node is a container.
    pub fn padding_x(mut self, padding: f32) -> Self {
        let padding = padding.max(0.0);
        self.padding.left = padding;
        self.padding.right = padding;
        self
    }

    /// Apply vertical content padding when this node is a container.
    pub fn padding_y(mut self, padding: f32) -> Self {
        let padding = padding.max(0.0);
        self.padding.top = padding;
        self.padding.bottom = padding;
        self
    }

    /// Apply an explicit widget style.
    pub fn style(mut self, style: WidgetStyle) -> Self {
        self.style = Some(style);
        self
    }

    /// Allow this styled container to show hover chrome.
    pub fn hoverable(mut self) -> Self {
        self.hoverable = true;
        self
    }

    /// Keep an interactive widget in hit testing without painting its own chrome.
    pub fn input_only(mut self) -> Self {
        self.input_only = true;
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

    /// Allow text to wrap by words inside its assigned rectangle.
    pub fn wrap(mut self) -> Self {
        self.text_wrap = Some(TextWrap::Word);
        self
    }

    /// Keep text on one line and clip overflow.
    pub fn truncate(mut self) -> Self {
        self.text_wrap = Some(TextWrap::None);
        self
    }

    /// Set row or column spacing when this node is a container.
    pub fn spacing(mut self, spacing: f32) -> Self {
        self.set_spacing(spacing);
        self
    }

    fn set_spacing(&mut self, spacing: f32) {
        match &mut self.kind {
            ViewNodeKind::Row {
                spacing: current, ..
            }
            | ViewNodeKind::Column {
                spacing: current, ..
            } => *current = spacing.max(0.0),
            ViewNodeKind::Scroll { child } => child.set_spacing(spacing),
            _ => {}
        }
    }

    fn collect_reserved_ids(&self, scope: u64, ids: &mut HashSet<NodeId>) {
        if let Some(id) = self.resolved_id(scope) {
            ids.insert(id);
        }
        let child_scope = self.child_scope(scope);
        match &self.kind {
            ViewNodeKind::Row { children, .. } | ViewNodeKind::Column { children, .. } => {
                for child in children {
                    child.collect_reserved_ids(child_scope, ids);
                }
            }
            ViewNodeKind::Stack { children } => {
                for child in children {
                    child.collect_reserved_ids(child_scope, ids);
                }
            }
            ViewNodeKind::Scroll { child } => child.collect_reserved_ids(child_scope, ids),
            ViewNodeKind::Runtime(node) => {
                ids.insert(node.id());
            }
            _ => {}
        }
    }

    fn resolved_id(&self, scope: u64) -> Option<NodeId> {
        self.id
            .or_else(|| self.key.as_ref().map(|key| scoped_key_id(scope, key)))
    }

    fn child_scope(&self, parent_scope: u64) -> u64 {
        self.resolved_id(parent_scope).unwrap_or(parent_scope)
    }
}

impl<Message> From<SurfaceNode<Message>> for ViewNode<Message> {
    fn from(node: SurfaceNode<Message>) -> Self {
        Self {
            kind: ViewNodeKind::Runtime(node),
            id: None,
            key: None,
            sizing: None,
            slot: SlotBehavior::default(),
            padding: Insets::default(),
            style: None,
            hoverable: false,
            input_only: false,
            text_wrap: None,
        }
    }
}

impl<Message> IntoView<Message> for ViewNode<Message>
where
    Message: 'static,
{
    fn into_node(self) -> SurfaceNode<Message> {
        let mut reserved = HashSet::new();
        self.collect_reserved_ids(ROOT_KEY_SCOPE, &mut reserved);
        let mut ids = IdGenerator::new(reserved);
        self.lower(&mut ids, ROOT_KEY_SCOPE)
    }
}

impl<Message> ViewNode<Message>
where
    Message: 'static,
{
    fn lower(self, ids: &mut IdGenerator, scope: u64) -> SurfaceNode<Message> {
        let id = self.resolved_id(scope).unwrap_or_else(|| ids.next());
        let child_scope = id;
        match self.kind {
            ViewNodeKind::Runtime(node) => node,
            ViewNodeKind::Text(value) => {
                let mut text =
                    TextWidget::new(id, value, self.sizing.unwrap_or_else(default_text_sizing));
                if let Some(wrap) = self.text_wrap {
                    text.wrap = wrap;
                }
                if let Some(style) = self.style {
                    text.common.style = style;
                }
                SurfaceNode::static_widget(WidgetSpec::Text(text))
            }
            ViewNodeKind::ButtonMapped { label, map } => {
                let mut button = ButtonWidget::new(
                    id,
                    label.clone(),
                    self.sizing.unwrap_or_else(|| default_button_sizing(&label)),
                );
                if let Some(style) = self.style {
                    button.common.style = style;
                }
                if self.input_only {
                    button.common.paint.paints_state_layers = false;
                }
                SurfaceNode::widget(
                    WidgetSpec::Button(button),
                    WidgetMessageMapper::button(move |message| map(message)),
                )
            }
            ViewNodeKind::DragHandle { map } => {
                let mut handle = DragHandleWidget::new(
                    id,
                    self.sizing.unwrap_or_else(default_drag_handle_sizing),
                );
                if let Some(style) = self.style {
                    handle.common.style = style;
                }
                if self.input_only {
                    handle.common.paint.paints_state_layers = false;
                }
                SurfaceNode::widget(
                    WidgetSpec::DragHandle(handle),
                    WidgetMessageMapper::drag_handle(move |message| map(message)),
                )
            }
            ViewNodeKind::Toggle {
                label,
                checked,
                compact,
                map,
            } => {
                let mut toggle = ToggleWidget::new(
                    id,
                    label.clone(),
                    self.sizing
                        .unwrap_or_else(|| default_toggle_sizing(&label, compact)),
                )
                .with_checked(checked);
                if let Some(style) = self.style {
                    toggle.common.style = style;
                }
                if self.input_only {
                    toggle.common.paint.paints_state_layers = false;
                }
                SurfaceNode::widget(
                    WidgetSpec::Toggle(toggle),
                    WidgetMessageMapper::toggle(move |message| match message {
                        crate::widgets::ToggleMessage::ValueChanged { checked } => map(checked),
                    }),
                )
            }
            ViewNodeKind::TextInput {
                value,
                placeholder,
                map,
            } => {
                let mut input = TextInputWidget::new(
                    id,
                    value,
                    self.sizing.unwrap_or(default_text_input_sizing()),
                );
                input.props.placeholder = placeholder;
                if let Some(style) = self.style {
                    input.common.style = style;
                }
                if self.input_only {
                    input.common.paint.paints_state_layers = false;
                }
                SurfaceNode::widget(
                    WidgetSpec::TextInput(input),
                    WidgetMessageMapper::text_input(move |message| match message {
                        crate::widgets::TextInputMessage::Changed { value }
                        | crate::widgets::TextInputMessage::Submitted { value } => map(value),
                    }),
                )
            }
            ViewNodeKind::CustomWidget { mut widget, map } => {
                let common = widget.common_mut();
                common.id = id;
                if let Some(sizing) = self.sizing {
                    common.sizing = sizing;
                }
                if let Some(style) = self.style {
                    common.style = style;
                }
                if self.input_only {
                    common.paint.paints_state_layers = false;
                }
                SurfaceNode::custom_widget_box(
                    widget,
                    WidgetMessageMapper::dynamic(move |output| map(output)),
                )
            }
            ViewNodeKind::Row { spacing, children } => {
                let policy = ContainerPolicy {
                    kind: ContainerKind::Row,
                    spacing,
                    padding: self.padding,
                    ..ContainerPolicy::default()
                };
                let children = children
                    .into_iter()
                    .map(|child| child.lower_child(ids, child_scope, true))
                    .collect();
                if let Some(style) = self.style {
                    SurfaceNode::styled_container(id, policy, style, children)
                        .with_container_hoverable(self.hoverable)
                } else {
                    SurfaceNode::container(id, policy, children)
                }
            }
            ViewNodeKind::Column { spacing, children } => {
                let policy = ContainerPolicy {
                    kind: ContainerKind::Column,
                    spacing,
                    padding: self.padding,
                    ..ContainerPolicy::default()
                };
                let children = children
                    .into_iter()
                    .map(|child| child.lower_child(ids, child_scope, false))
                    .collect();
                if let Some(style) = self.style {
                    SurfaceNode::styled_container(id, policy, style, children)
                        .with_container_hoverable(self.hoverable)
                } else {
                    SurfaceNode::container(id, policy, children)
                }
            }
            ViewNodeKind::Scroll { child } => {
                let policy = ContainerPolicy {
                    kind: ContainerKind::ScrollView,
                    overflow: crate::layout::OverflowPolicy::Scroll,
                    padding: self.padding,
                    ..ContainerPolicy::default()
                };
                let children = vec![SurfaceChild::fill(child.lower(ids, child_scope))];
                if let Some(style) = self.style {
                    SurfaceNode::styled_container(id, policy, style, children)
                        .with_container_hoverable(self.hoverable)
                } else {
                    SurfaceNode::container(id, policy, children)
                }
            }
            ViewNodeKind::Stack { children } => {
                let policy = ContainerPolicy {
                    kind: ContainerKind::Stack,
                    padding: self.padding,
                    ..ContainerPolicy::default()
                };
                let children = children
                    .into_iter()
                    .map(|child| SurfaceChild::fill(child.lower(ids, child_scope)))
                    .collect();
                if let Some(style) = self.style {
                    SurfaceNode::styled_container(id, policy, style, children)
                        .with_container_hoverable(self.hoverable)
                } else {
                    SurfaceNode::container(id, policy, children)
                }
            }
            ViewNodeKind::OverlayPanel { rect, label } => {
                if let Some(label) = label {
                    SurfaceNode::overlay_panel(id, rect, label, self.style.unwrap_or_default())
                } else {
                    SurfaceNode::overlay_marker(id, rect, self.style.unwrap_or_default())
                }
            }
        }
    }

    fn lower_child(
        self,
        ids: &mut IdGenerator,
        scope: u64,
        parent_horizontal: bool,
    ) -> SurfaceChild<Message> {
        let slot = self.slot.to_slot_params(parent_horizontal);
        SurfaceChild::new(slot, self.lower(ids, scope))
    }
}

/// Build a non-interactive text view with generated identity and default sizing.
pub fn text<Message>(value: impl Into<String>) -> ViewNode<Message> {
    ViewNode {
        kind: ViewNodeKind::Text(value.into()),
        id: None,
        key: None,
        sizing: None,
        slot: SlotBehavior::default(),
        padding: Insets::default(),
        style: None,
        hoverable: false,
        input_only: false,
        text_wrap: None,
    }
}

/// Build a custom widget view with generated identity and an output mapper.
pub fn custom_widget<Message>(
    widget: impl Widget + Clone + 'static,
    map: impl Fn(WidgetOutput) -> Option<Message> + Send + Sync + 'static,
) -> ViewNode<Message> {
    ViewNode {
        kind: ViewNodeKind::CustomWidget {
            widget: Box::new(widget),
            map: Arc::new(map),
        },
        id: None,
        key: None,
        sizing: None,
        slot: SlotBehavior::default(),
        padding: Insets::default(),
        style: None,
        hoverable: false,
        input_only: false,
        text_wrap: None,
    }
}

/// Builder for buttons that can emit messages or mutate state directly.
pub struct ButtonBuilder {
    label: String,
    style: Option<WidgetStyle>,
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

    /// Emit one cloned host message when activated.
    pub fn message<Message>(self, message: Message) -> ViewNode<Message>
    where
        Message: Clone + Send + Sync + 'static,
    {
        self.mapped(move |_| message.clone())
    }

    /// Emit a mapped host message when activated.
    pub fn mapped<Message>(
        self,
        map: impl Fn(crate::widgets::ButtonMessage) -> Message + Send + Sync + 'static,
    ) -> ViewNode<Message> {
        ViewNode {
            kind: ViewNodeKind::ButtonMapped {
                label: self.label,
                map: Arc::new(map),
            },
            id: None,
            key: None,
            sizing: None,
            slot: SlotBehavior::default(),
            padding: Insets::default(),
            style: self.style,
            hoverable: false,
            input_only: false,
            text_wrap: None,
        }
    }

    /// Mutate application state directly when activated.
    pub fn on_click<State: 'static>(
        self,
        apply: impl Fn(&mut State) + Send + Sync + 'static,
    ) -> ViewNode<StateAction<State>> {
        self.message(StateAction::new(apply))
    }
}

/// Build a button.
pub fn button(label: impl Into<String>) -> ButtonBuilder {
    ButtonBuilder {
        label: label.into(),
        style: None,
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
pub fn button_mapped<Message>(
    label: impl Into<String>,
    map: impl Fn(crate::widgets::ButtonMessage) -> Message + Send + Sync + 'static,
) -> ViewNode<Message> {
    button(label).mapped(map)
}

/// Builder for compact drag handles that can emit messages or mutate state directly.
pub struct DragHandleBuilder;

impl DragHandleBuilder {
    /// Emit a mapped host message for drag lifecycle events.
    pub fn mapped<Message>(
        self,
        map: impl Fn(crate::widgets::DragHandleMessage) -> Message + Send + Sync + 'static,
    ) -> ViewNode<Message> {
        ViewNode {
            kind: ViewNodeKind::DragHandle { map: Arc::new(map) },
            id: None,
            key: None,
            sizing: None,
            slot: SlotBehavior::default(),
            padding: Insets::default(),
            style: None,
            hoverable: false,
            input_only: false,
            text_wrap: None,
        }
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
pub fn drag_handle_mapped<Message>(
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
    pub fn message<Message>(
        self,
        map: impl Fn(bool) -> Message + Send + Sync + 'static,
    ) -> ViewNode<Message> {
        ViewNode {
            kind: ViewNodeKind::Toggle {
                label: self.label,
                checked: self.checked,
                compact: self.compact,
                map: Arc::new(map),
            },
            id: None,
            key: None,
            sizing: None,
            slot: SlotBehavior::default(),
            padding: Insets::default(),
            style: self.style,
            hoverable: false,
            input_only: false,
            text_wrap: None,
        }
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
pub fn toggle_mapped<Message>(
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
    pub fn message<Message>(
        self,
        map: impl Fn(String) -> Message + Send + Sync + 'static,
    ) -> ViewNode<Message> {
        ViewNode {
            kind: ViewNodeKind::TextInput {
                value: self.value,
                placeholder: self.placeholder,
                map: Arc::new(map),
            },
            id: None,
            key: None,
            sizing: None,
            slot: SlotBehavior::default(),
            padding: Insets::default(),
            style: self.style,
            hoverable: false,
            input_only: false,
            text_wrap: None,
        }
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
pub fn text_input_mapped<Message>(
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
        style: Some(primary_style()),
        hoverable: false,
        input_only: false,
        text_wrap: None,
    }
}

impl SlotBehavior {
    fn to_slot_params(self, horizontal: bool) -> SlotParams {
        let main_axis = if horizontal { self.width } else { self.height };
        let cross_axis = if horizontal { self.height } else { self.width };
        SlotParams {
            size_main: main_axis.to_main(),
            size_cross: cross_axis.to_cross(),
            ..SlotParams::fill()
        }
    }
}

impl AxisSlotBehavior {
    fn to_main(self) -> SizeModeMain {
        match self {
            Self::Default | Self::Intrinsic => SizeModeMain::Intrinsic,
            Self::Fill(weight) => SizeModeMain::Fill(weight.max(0.0)),
            Self::Fixed(value) => SizeModeMain::Fixed(value.max(0.0)),
        }
    }

    fn to_cross(self) -> SizeModeCross {
        match self {
            Self::Default | Self::Intrinsic => SizeModeCross::Intrinsic,
            Self::Fill(_) => SizeModeCross::Fill,
            Self::Fixed(value) => SizeModeCross::Fixed(value.max(0.0)),
        }
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

struct IdGenerator {
    next: NodeId,
    reserved: HashSet<NodeId>,
}

impl IdGenerator {
    fn new(reserved: HashSet<NodeId>) -> Self {
        Self { next: 1, reserved }
    }

    fn next(&mut self) -> NodeId {
        while self.reserved.contains(&self.next) {
            self.next += 1;
        }
        let id = self.next;
        self.reserved.insert(id);
        self.next += 1;
        id
    }
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

fn scoped_key_id(scope: u64, key: &str) -> NodeId {
    let mut hash = ROOT_KEY_SCOPE;
    hash = hash_bytes(hash, &scope.to_le_bytes());
    hash = hash_bytes(hash, key.as_bytes());
    if hash == 0 { 1 } else { hash }
}

fn hash_bytes(mut hash: u64, bytes: &[u8]) -> u64 {
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash
}
