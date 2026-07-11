//! Common host-action routing for interactive row messages.

use crate::{
    gui::types::Point,
    widgets::interaction::{DragHandleMessage, PointerModifiers},
};
use std::sync::Arc;

/// Host callbacks for common interactive-row message routing.
///
/// Use this router when a row host only needs the standard activation,
/// secondary-click, drag, drop, and hover-drop interaction shapes translated
/// into its own message type.
#[derive(Clone)]
pub struct InteractiveRowActions<Message> {
    activate: Option<Arc<dyn Fn() -> Message + Send + Sync + 'static>>,
    activate_with_modifiers:
        Option<Arc<dyn Fn(PointerModifiers) -> Message + Send + Sync + 'static>>,
    double_activate: Option<Arc<dyn Fn() -> Message + Send + Sync + 'static>>,
    hover: Option<Arc<dyn Fn(Point) -> Message + Send + Sync + 'static>>,
    secondary: Option<Arc<dyn Fn(Point) -> Message + Send + Sync + 'static>>,
    drag: Option<Arc<dyn Fn(DragHandleMessage) -> Message + Send + Sync + 'static>>,
    drop: Option<Arc<dyn Fn() -> Message + Send + Sync + 'static>>,
    hover_drop: Option<Arc<dyn Fn(Point) -> Message + Send + Sync + 'static>>,
    clear_drop: Option<Arc<dyn Fn(Point) -> Message + Send + Sync + 'static>>,
}

impl<Message> InteractiveRowActions<Message> {
    /// Build an empty row-action router.
    pub fn new() -> Self {
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

impl<Message> std::fmt::Debug for InteractiveRowActions<Message> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("InteractiveRowActions")
            .finish_non_exhaustive()
    }
}
