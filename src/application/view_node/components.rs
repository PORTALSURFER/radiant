//! Bounded, application-owned memoization of pure component projections.

use super::{ViewNode, ViewNodeKind, slot::SlotBehavior};
use crate::{
    application::IntoView,
    runtime::{ResolvedEnvironment, SurfaceNode},
};
use std::{
    any::{Any, TypeId},
    collections::{HashMap, HashSet},
    panic::panic_any,
};

pub(in crate::application) type ComponentEnvironmentSource<State> = std::rc::Rc<
    std::cell::RefCell<std::rc::Rc<dyn Fn(&State) -> crate::application::ApplicationEnvironment>>,
>;

const MAX_COMPONENTS: usize = 64;
const MAX_RETAINED_NODES: usize = 32_768;
const MAX_KEY_BYTES: usize = 256;

struct Entry<Message> {
    input: Box<dyn Any>,
    projector: TypeId,
    root: SurfaceNode<Message>,
    slot: SlotBehavior,
    nodes: usize,
}

pub(in crate::application) struct ComponentProjectionCache<Message> {
    entries: HashMap<String, Entry<Message>>,
    environment: Option<ResolvedEnvironment>,
}

impl<Message> Default for ComponentProjectionCache<Message> {
    fn default() -> Self {
        Self {
            entries: HashMap::new(),
            environment: None,
        }
    }
}

/// Work performed during one opt-in component projection.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ComponentProjectionCounters {
    /// Component functions actually called, including uncached fallbacks.
    pub callbacks: usize,
    /// Components whose exact input and environment reused a completed result.
    pub cache_hits: usize,
    /// Nodes retained by the component cache after the latest call.
    pub retained_nodes: usize,
}

/// UI-local context for explicitly dependency-qualified component functions.
///
/// Components remain ordinary pure Rust functions. Pass every state, theme,
/// mapper, and resource dependency in the comparable input. Functions receive
/// the current window and application environment automatically. Results with
/// application Scene bindings are projected normally rather than cached.
/// This caches declarative snapshots, never runtime focus, capture or edit state.
pub struct ComponentProjectionContext<'a, Message> {
    cache: &'a mut ComponentProjectionCache<Message>,
    environment: ResolvedEnvironment,
    seen: HashSet<String>,
    counters: ComponentProjectionCounters,
    finished: bool,
}

impl<Message> ComponentProjectionCache<Message> {
    pub(in crate::application) fn begin(
        &mut self,
        environment: ResolvedEnvironment,
    ) -> ComponentProjectionContext<'_, Message> {
        if self.environment.as_ref() != Some(&environment) {
            self.entries.clear();
            self.environment = Some(environment.clone());
        }
        let retained_nodes = self.entries.values().map(|entry| entry.nodes).sum();
        ComponentProjectionContext {
            cache: self,
            environment,
            seen: HashSet::new(),
            counters: ComponentProjectionCounters {
                retained_nodes,
                ..Default::default()
            },
            finished: false,
        }
    }
}

impl<Message: 'static> ComponentProjectionContext<'_, Message> {
    /// Project one stable component key using exact comparable inputs.
    ///
    /// Function items and capture-free closures qualify for reuse. Capturing
    /// closures and function pointers use fresh projection because their type
    /// alone cannot identify their behavior. The function must be pure: mutable
    /// globals and undeclared dependencies cannot participate in memoization.
    /// Input equality must cover every value that changes the returned view.
    ///
    /// Keys are unique within one projection; duplicates panic before a
    /// surface is published. At most 64 results with keys of at most 256 bytes
    /// are retained, with a total budget of 32,768 nodes. Capacity overflow
    /// remains valid and projects freshly without retaining the result.
    pub fn project<Input, Project>(
        &mut self,
        key: impl Into<String>,
        input: Input,
        project: Project,
    ) -> ViewNode<Message>
    where
        Input: PartialEq + 'static,
        Project: FnOnce(&Input, &ResolvedEnvironment) -> ViewNode<Message> + 'static,
    {
        let key = key.into();
        if !self.seen.insert(key.clone()) {
            panic_any("duplicate component projection key");
        }
        let cacheable = std::mem::size_of::<Project>() == 0 && key.len() <= MAX_KEY_BYTES;
        if cacheable
            && let Some(entry) = self.cache.entries.get(&key)
            && entry.projector == TypeId::of::<Project>()
            && entry.input.downcast_ref::<Input>() == Some(&input)
        {
            self.counters.cache_hits += 1;
            return snapshot_view(entry.root.clone(), entry.slot);
        }
        self.counters.callbacks += 1;
        let view = project(&input, &self.environment).key(key.clone());
        let old_nodes = self
            .cache
            .entries
            .remove(&key)
            .map_or(0, |entry| entry.nodes);
        self.counters.retained_nodes -= old_nodes;
        if !plain_component(&view) {
            return view;
        }
        let slot = view.slot;
        let root = view.into_surface().into_root();
        let available = MAX_RETAINED_NODES - self.counters.retained_nodes;
        if cacheable
            && self.cache.entries.len() < MAX_COMPONENTS
            && let Some(nodes) = root.component_cache_node_count(available)
        {
            self.cache.entries.insert(
                key,
                Entry {
                    input: Box::new(input),
                    projector: TypeId::of::<Project>(),
                    root: root.clone(),
                    slot,
                    nodes,
                },
            );
            self.counters.retained_nodes += nodes;
        }
        snapshot_view(root, slot)
    }

    /// Read actual callback/cache work performed so far in this projection.
    pub fn counters(&self) -> ComponentProjectionCounters {
        self.counters
    }

    /// Borrow the exact environment used to qualify every cached result.
    pub fn environment(&self) -> &ResolvedEnvironment {
        &self.environment
    }
}

impl<Message> ComponentProjectionContext<'_, Message> {
    pub(in crate::application) fn finish(mut self) {
        self.cache.entries.retain(|key, _| self.seen.contains(key));
        self.finished = true;
    }
}

impl<Message> Drop for ComponentProjectionContext<'_, Message> {
    fn drop(&mut self) {
        if !self.finished {
            // A panicking projector never leaves a partially updated cache.
            self.cache.entries.clear();
            self.cache.environment = None;
        }
    }
}

fn snapshot_view<Message>(root: SurfaceNode<Message>, slot: SlotBehavior) -> ViewNode<Message> {
    let mut view = ViewNode::from(root);
    view.slot = slot;
    view
}

fn plain_component<Message>(view: &ViewNode<Message>) -> bool {
    if !view.overlay_layers.is_empty() || view.effect_owner.is_some() {
        return false;
    }
    match &view.kind {
        ViewNodeKind::Widget(_) => true,
        ViewNodeKind::Container { children, .. } | ViewNodeKind::CustomLayout { children, .. } => {
            children.iter().all(plain_component)
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests;
