//! Beginner-facing Radiant application and view builders.
//!
//! This module is a convenience layer over [`crate::runtime`]. It generates
//! deterministic widget ids, supplies default widget sizing, and lowers every
//! view into the existing [`UiSurface`](crate::runtime::UiSurface) tree.

use crate::{
    gui_runtime::NativeRunOptions,
    layout::{NodeId, Vector2},
    runtime::{
        Command, RuntimeBridge, SurfaceChild, SurfaceNode, UiSurface,
        declarative_command_runtime_bridge, run_native_vello_runtime,
    },
    widgets::WidgetSizing,
};
use std::{collections::HashSet, marker::PhantomData, sync::Arc};

/// Beginner-facing result type used by native launch helpers.
pub type Result<T = ()> = std::result::Result<T, String>;

/// Build a native window launcher for a simple Radiant view.
pub fn window(title: impl Into<String>) -> WindowBuilder {
    WindowBuilder::new(title)
}

/// Build a stateful app launcher over the existing command runtime bridge.
pub fn app<State>(state: State) -> StatefulAppBuilder<State> {
    StatefulAppBuilder::new(state)
}

/// Converts beginner-facing view values into the existing runtime surface.
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

    /// Set the full native runtime options for apps that need advanced launch configuration.
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

/// Beginner-facing view node with generated identity and default sizing.
pub struct ViewNode<Message> {
    kind: ViewNodeKind<Message>,
    id: Option<NodeId>,
    sizing: Option<WidgetSizing>,
}

enum ViewNodeKind<Message> {
    Runtime(SurfaceNode<Message>),
    Text(String),
    ButtonMapped {
        label: String,
        map: Arc<dyn Fn(crate::widgets::ButtonMessage) -> Message + Send + Sync>,
    },
    Toggle {
        label: String,
        checked: bool,
        map: Arc<dyn Fn(bool) -> Message + Send + Sync>,
    },
    TextInput {
        value: String,
        map: Arc<dyn Fn(String) -> Message + Send + Sync>,
    },
    Row {
        spacing: f32,
        children: Vec<ViewNode<Message>>,
    },
    Column {
        spacing: f32,
        children: Vec<ViewNode<Message>>,
    },
}

impl<Message> ViewNode<Message> {
    /// Use an explicit stable id instead of the generated structural id.
    pub fn id(mut self, id: NodeId) -> Self {
        self.id = Some(id);
        self
    }

    /// Use explicit widget sizing instead of the beginner default.
    pub fn sizing(mut self, sizing: WidgetSizing) -> Self {
        self.sizing = Some(sizing);
        self
    }

    /// Use explicit fixed widget sizing instead of the beginner default.
    pub fn size(self, width: f32, height: f32) -> Self {
        self.sizing(WidgetSizing::fixed(Vector2::new(width, height)))
    }

    /// Set row or column spacing when this node is a container.
    pub fn spacing(mut self, spacing: f32) -> Self {
        match &mut self.kind {
            ViewNodeKind::Row {
                spacing: current, ..
            }
            | ViewNodeKind::Column {
                spacing: current, ..
            } => *current = spacing.max(0.0),
            _ => {}
        }
        self
    }

    fn collect_explicit_ids(&self, ids: &mut HashSet<NodeId>) {
        if let Some(id) = self.id {
            ids.insert(id);
        }
        match &self.kind {
            ViewNodeKind::Row { children, .. } | ViewNodeKind::Column { children, .. } => {
                for child in children {
                    child.collect_explicit_ids(ids);
                }
            }
            ViewNodeKind::Runtime(node) => {
                ids.insert(node.id());
            }
            _ => {}
        }
    }
}

impl<Message> From<SurfaceNode<Message>> for ViewNode<Message> {
    fn from(node: SurfaceNode<Message>) -> Self {
        Self {
            kind: ViewNodeKind::Runtime(node),
            id: None,
            sizing: None,
        }
    }
}

impl<Message> IntoView<Message> for ViewNode<Message>
where
    Message: 'static,
{
    fn into_node(self) -> SurfaceNode<Message> {
        let mut reserved = HashSet::new();
        self.collect_explicit_ids(&mut reserved);
        let mut ids = IdGenerator::new(reserved);
        self.lower(&mut ids)
    }
}

impl<Message> ViewNode<Message>
where
    Message: 'static,
{
    fn lower(self, ids: &mut IdGenerator) -> SurfaceNode<Message> {
        let id = self.id.unwrap_or_else(|| ids.next());
        match self.kind {
            ViewNodeKind::Runtime(node) => node,
            ViewNodeKind::Text(value) => {
                SurfaceNode::text(id, value, self.sizing.unwrap_or_else(default_text_sizing))
            }
            ViewNodeKind::ButtonMapped { label, map } => SurfaceNode::button_mapped(
                id,
                label.clone(),
                self.sizing.unwrap_or_else(|| default_button_sizing(&label)),
                move |message| map(message),
            ),
            ViewNodeKind::Toggle {
                label,
                checked,
                map,
            } => SurfaceNode::toggle_with_checked(
                id,
                label.clone(),
                checked,
                self.sizing.unwrap_or_else(|| default_toggle_sizing(&label)),
                move |checked| map(checked),
            ),
            ViewNodeKind::TextInput { value, map } => SurfaceNode::text_input(
                id,
                value,
                self.sizing.unwrap_or(default_text_input_sizing()),
                move |value| map(value),
            ),
            ViewNodeKind::Row { spacing, children } => SurfaceNode::row(
                id,
                spacing,
                children
                    .into_iter()
                    .map(|child| SurfaceChild::fill(child.lower(ids)))
                    .collect(),
            ),
            ViewNodeKind::Column { spacing, children } => SurfaceNode::column(
                id,
                spacing,
                children
                    .into_iter()
                    .map(|child| SurfaceChild::fill(child.lower(ids)))
                    .collect(),
            ),
        }
    }
}

/// Build a non-interactive text view with generated identity and default sizing.
pub fn text<Message>(value: impl Into<String>) -> ViewNode<Message> {
    ViewNode {
        kind: ViewNodeKind::Text(value.into()),
        id: None,
        sizing: None,
    }
}

/// Build a button that emits one cloned host message when activated.
pub fn button<Message>(label: impl Into<String>, message: Message) -> ViewNode<Message>
where
    Message: Clone + Send + Sync + 'static,
{
    ViewNode {
        kind: ViewNodeKind::ButtonMapped {
            label: label.into(),
            map: Arc::new(move |_| message.clone()),
        },
        id: None,
        sizing: None,
    }
}

/// Build a button with a custom widget-message mapper.
pub fn button_mapped<Message>(
    label: impl Into<String>,
    map: impl Fn(crate::widgets::ButtonMessage) -> Message + Send + Sync + 'static,
) -> ViewNode<Message> {
    ViewNode {
        kind: ViewNodeKind::ButtonMapped {
            label: label.into(),
            map: Arc::new(map),
        },
        id: None,
        sizing: None,
    }
}

/// Build a toggle that maps value changes by checked state.
pub fn toggle<Message>(
    label: impl Into<String>,
    checked: bool,
    map: impl Fn(bool) -> Message + Send + Sync + 'static,
) -> ViewNode<Message> {
    ViewNode {
        kind: ViewNodeKind::Toggle {
            label: label.into(),
            checked,
            map: Arc::new(map),
        },
        id: None,
        sizing: None,
    }
}

/// Build a single-line text input that maps edits and submissions by value.
pub fn text_input<Message>(
    value: impl Into<String>,
    map: impl Fn(String) -> Message + Send + Sync + 'static,
) -> ViewNode<Message> {
    ViewNode {
        kind: ViewNodeKind::TextInput {
            value: value.into(),
            map: Arc::new(map),
        },
        id: None,
        sizing: None,
    }
}

/// Build a row container with fill-slot children.
pub fn row<Message>(children: impl IntoIterator<Item = ViewNode<Message>>) -> ViewNode<Message> {
    ViewNode {
        kind: ViewNodeKind::Row {
            spacing: 8.0,
            children: children.into_iter().collect(),
        },
        id: None,
        sizing: None,
    }
}

/// Build a column container with fill-slot children.
pub fn column<Message>(children: impl IntoIterator<Item = ViewNode<Message>>) -> ViewNode<Message> {
    ViewNode {
        kind: ViewNodeKind::Column {
            spacing: 6.0,
            children: children.into_iter().collect(),
        },
        id: None,
        sizing: None,
    }
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
    let width = (label.chars().count() as f32 * 8.0 + 32.0).clamp(80.0, 240.0);
    WidgetSizing::fixed(Vector2::new(width, 32.0))
}

fn default_toggle_sizing(label: &str) -> WidgetSizing {
    let width = (label.chars().count() as f32 * 8.0 + 52.0).clamp(96.0, 280.0);
    WidgetSizing::fixed(Vector2::new(width, 30.0))
}

fn default_text_input_sizing() -> WidgetSizing {
    WidgetSizing::new(Vector2::new(160.0, 32.0), Vector2::new(240.0, 32.0))
}
