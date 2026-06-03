use crate::{
    application::{
        MappedWidget, StateAction, ViewNode, default_drag_handle_sizing, view_node_from_widget,
    },
    runtime::WidgetMessageMapper,
    widgets::{DragHandleMessage, DragHandleWidget},
};
use std::sync::Arc;

/// Builder for compact drag handles that can emit messages or mutate state directly.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DragHandleBuilder {
    hover_chrome_only: bool,
}

impl DragHandleBuilder {
    /// Paint handle chrome only while hovered, pressed, or focused.
    pub fn hover_chrome_only(mut self) -> Self {
        self.hover_chrome_only = true;
        self
    }

    /// Emit a mapped host message for drag lifecycle events.
    pub fn mapped<Message: 'static>(
        self,
        map: impl Fn(DragHandleMessage) -> Message + Send + Sync + 'static,
    ) -> ViewNode<Message> {
        let mut handle = DragHandleWidget::new(0, default_drag_handle_sizing());
        if self.hover_chrome_only {
            handle = handle.with_hover_chrome_only();
        }
        view_node_from_widget(MappedWidget::new(
            handle,
            WidgetMessageMapper::drag_handle(map),
        ))
    }

    /// Mutate application state directly when the handle is dragged.
    pub fn on_drag<State: 'static>(
        self,
        apply: impl Fn(&mut State, DragHandleMessage) + Send + Sync + 'static,
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
    DragHandleBuilder {
        hover_chrome_only: false,
    }
}

/// Build a drag handle with a custom widget-message mapper.
pub fn drag_handle_mapped<Message: 'static>(
    map: impl Fn(DragHandleMessage) -> Message + Send + Sync + 'static,
) -> ViewNode<Message> {
    drag_handle().mapped(map)
}
