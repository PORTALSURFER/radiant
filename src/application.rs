//! Readable Radiant application and view builders.
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
    Scroll {
        child: Box<ViewNode<Message>>,
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
            ViewNodeKind::Toggle { label, .. } => default_toggle_sizing(label),
            ViewNodeKind::TextInput { .. } => default_text_input_sizing(),
            _ => WidgetSizing::fixed(Vector2::new(0.0, 0.0)),
        });
        self.sizing = Some(sizing.with_baseline(baseline));
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
                    .map(|child| SurfaceChild::fill(child.lower(ids, child_scope)))
                    .collect(),
            ),
            ViewNodeKind::Column { spacing, children } => SurfaceNode::column(
                id,
                spacing,
                children
                    .into_iter()
                    .map(|child| SurfaceChild::fill(child.lower(ids, child_scope)))
                    .collect(),
            ),
            ViewNodeKind::Scroll { child } => {
                SurfaceNode::scroll_area(id, child.lower(ids, child_scope))
            }
        }
    }
}

/// Build a non-interactive text view with generated identity and default sizing.
pub fn text<Message>(value: impl Into<String>) -> ViewNode<Message> {
    ViewNode {
        kind: ViewNodeKind::Text(value.into()),
        id: None,
        key: None,
        sizing: None,
    }
}

/// Builder for buttons that can emit messages or mutate state directly.
pub struct ButtonBuilder {
    label: String,
}

impl ButtonBuilder {
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

/// Builder for toggles that can emit messages or mutate state directly.
pub struct ToggleBuilder {
    label: String,
    checked: bool,
}

impl ToggleBuilder {
    /// Emit a host message mapped from checked state.
    pub fn message<Message>(
        self,
        map: impl Fn(bool) -> Message + Send + Sync + 'static,
    ) -> ViewNode<Message> {
        ViewNode {
            kind: ViewNodeKind::Toggle {
                label: self.label,
                checked: self.checked,
                map: Arc::new(map),
            },
            id: None,
            key: None,
            sizing: None,
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
}

impl TextInputBuilder {
    /// Emit a host message mapped from the input value.
    pub fn message<Message>(
        self,
        map: impl Fn(String) -> Message + Send + Sync + 'static,
    ) -> ViewNode<Message> {
        ViewNode {
            kind: ViewNodeKind::TextInput {
                value: self.value,
                map: Arc::new(map),
            },
            id: None,
            key: None,
            sizing: None,
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
    }
}

/// Build a keyed column container with fill-slot children.
pub fn column_key<Message>(
    key: impl ToString,
    children: impl IntoIterator<Item = ViewNode<Message>>,
) -> ViewNode<Message> {
    column(children).key(key)
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
    }
}

/// Build a scroll viewport containing a column projected from an iterator.
pub fn scroll_column<Message, Item>(
    items: impl IntoIterator<Item = Item>,
    mut project: impl FnMut(Item) -> ViewNode<Message>,
) -> ViewNode<Message> {
    scroll(column(
        items
            .into_iter()
            .map(|item| project(item))
            .collect::<Vec<_>>(),
    ))
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
