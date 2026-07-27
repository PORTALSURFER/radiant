//! Common host-action routing for interactive row messages.

use crate::{
    gui::types::Point,
    widgets::interaction::{DragHandleMessage, InteractiveRowMessage, PointerModifiers},
};
use std::{rc::Rc, sync::Arc};

/// Legacy transfer-safe callback storage retained for existing callers.
#[derive(Clone)]
struct SharedActionRouter<Message> {
    activate: Option<Arc<dyn Fn(()) -> Message + Send + Sync + 'static>>,
    activate_with_modifiers:
        Option<Arc<dyn Fn(PointerModifiers) -> Message + Send + Sync + 'static>>,
    double_activate: Option<Arc<dyn Fn(()) -> Message + Send + Sync + 'static>>,
    hover: Option<Arc<dyn Fn(Point) -> Message + Send + Sync + 'static>>,
    secondary: Option<Arc<dyn Fn(Point) -> Message + Send + Sync + 'static>>,
    drag: Option<Arc<dyn Fn(DragHandleMessage) -> Message + Send + Sync + 'static>>,
    drop: Option<Arc<dyn Fn(()) -> Message + Send + Sync + 'static>>,
    hover_drop: Option<Arc<dyn Fn(Point) -> Message + Send + Sync + 'static>>,
    clear_drop: Option<Arc<dyn Fn(Point) -> Message + Send + Sync + 'static>>,
}

/// UI-local callback storage. `Rc` intentionally keeps this router on the UI thread.
#[derive(Clone)]
struct LocalActionRouter<Message> {
    activate: Option<Rc<dyn Fn(()) -> Message + 'static>>,
    activate_with_modifiers: Option<Rc<dyn Fn(PointerModifiers) -> Message + 'static>>,
    double_activate: Option<Rc<dyn Fn(()) -> Message + 'static>>,
    hover: Option<Rc<dyn Fn(Point) -> Message + 'static>>,
    secondary: Option<Rc<dyn Fn(Point) -> Message + 'static>>,
    drag: Option<Rc<dyn Fn(DragHandleMessage) -> Message + 'static>>,
    drop: Option<Rc<dyn Fn(()) -> Message + 'static>>,
    hover_drop: Option<Rc<dyn Fn(Point) -> Message + 'static>>,
    clear_drop: Option<Rc<dyn Fn(Point) -> Message + 'static>>,
}

macro_rules! route_action {
    ($router:expr, $message:expr) => {{
        let message = $message;
        if let Some(position) = message.hover_position() {
            return $router.hover.as_ref().map(|callback| callback(position));
        }
        if let Some(position) = message.secondary_position() {
            return $router
                .secondary
                .as_ref()
                .map(|callback| callback(position));
        }
        if let Some(drag) = message.drag_message() {
            return $router.drag.as_ref().map(|callback| callback(drag));
        }
        if message.is_drop() {
            return $router.drop.as_ref().map(|callback| callback(()));
        }
        if let Some(position) = message.hover_drop_position() {
            return $router
                .hover_drop
                .as_ref()
                .map(|callback| callback(position));
        }
        if let Some(position) = message.clear_drop_position() {
            return $router
                .clear_drop
                .as_ref()
                .map(|callback| callback(position));
        }
        if message.is_double_activation() {
            return $router
                .double_activate
                .as_ref()
                .map(|callback| callback(()));
        }
        if let Some(modifiers) = message.single_activation_modifiers() {
            if let Some(callback) = &$router.activate_with_modifiers {
                return Some(callback(modifiers));
            }
            return $router.activate.as_ref().map(|callback| callback(()));
        }
        None
    }};
}

impl<Message> SharedActionRouter<Message> {
    fn new() -> Self {
        Self {
            activate: None,
            activate_with_modifiers: None,
            double_activate: None,
            hover: None,
            secondary: None,
            drag: None,
            drop: None,
            hover_drop: None,
            clear_drop: None,
        }
    }

    fn route(&self, message: InteractiveRowMessage) -> Option<Message> {
        route_action!(self, message)
    }

    fn routes_hover(&self) -> bool {
        self.hover.is_some()
    }
}

impl<Message> LocalActionRouter<Message> {
    fn new() -> Self {
        Self {
            activate: None,
            activate_with_modifiers: None,
            double_activate: None,
            hover: None,
            secondary: None,
            drag: None,
            drop: None,
            hover_drop: None,
            clear_drop: None,
        }
    }

    fn route(&self, message: InteractiveRowMessage) -> Option<Message> {
        route_action!(self, message)
    }

    fn routes_hover(&self) -> bool {
        self.hover.is_some()
    }
}

/// Small internal router abstraction used by both public callback owners.
enum InteractiveRowActionRouter<'a, Message> {
    Shared(&'a SharedActionRouter<Message>),
    Local(&'a LocalActionRouter<Message>),
}

impl<Message> InteractiveRowActionRouter<'_, Message> {
    fn route(&self, message: InteractiveRowMessage) -> Option<Message> {
        match self {
            Self::Shared(router) => router.route(message),
            Self::Local(router) => router.route(message),
        }
    }

    fn routes_hover(&self) -> bool {
        match self {
            Self::Shared(router) => router.routes_hover(),
            Self::Local(router) => router.routes_hover(),
        }
    }
}

/// Host callbacks for common interactive-row message routing.
///
/// This is the legacy transfer-safe surface. Existing callers remain backed by
/// `Arc` callbacks and retain their `Send + Sync` bounds.
#[derive(Clone)]
pub struct InteractiveRowActions<Message> {
    router: SharedActionRouter<Message>,
}

impl<Message> InteractiveRowActions<Message> {
    /// Build an empty row-action router.
    pub fn new() -> Self {
        Self {
            router: SharedActionRouter::new(),
        }
    }
}

/// UI-local host callbacks for common interactive-row message routing.
#[derive(Clone)]
pub struct InteractiveRowLocalActions<Message> {
    router: LocalActionRouter<Message>,
}

impl<Message> InteractiveRowLocalActions<Message> {
    /// Build an empty UI-local row-action router.
    pub fn new() -> Self {
        Self {
            router: LocalActionRouter::new(),
        }
    }
}

mod activation;
mod drag_drop;
mod routing;
mod secondary;

impl<Message> Default for InteractiveRowActions<Message> {
    fn default() -> Self {
        Self::new()
    }
}

impl<Message> Default for InteractiveRowLocalActions<Message> {
    fn default() -> Self {
        Self::new()
    }
}

impl<Message> std::fmt::Debug for InteractiveRowActions<Message> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("InteractiveRowActions")
            .finish_non_exhaustive()
    }
}

impl<Message> std::fmt::Debug for InteractiveRowLocalActions<Message> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("InteractiveRowLocalActions")
            .finish_non_exhaustive()
    }
}
