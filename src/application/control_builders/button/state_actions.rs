use crate::{
    application::compatibility::StateAction,
    gui::types::Point,
    widgets::{ButtonMessage, DragHandleMessage},
};
use std::sync::Arc;

pub(super) fn click_or_secondary_action<State, Primary, Secondary>(
    message: ButtonMessage,
    primary: Arc<Primary>,
    secondary: Arc<Secondary>,
) -> StateAction<State>
where
    State: 'static,
    Primary: Fn(&mut State) + Send + Sync + 'static,
    Secondary: Fn(&mut State) + Send + Sync + 'static,
{
    StateAction::new(move |state| match message {
        ButtonMessage::Activate => primary(state),
        ButtonMessage::SecondaryActivate { .. } => secondary(state),
        ButtonMessage::Drag(_) => {}
    })
}

pub(super) fn click_or_secondary_at_action<State, Primary, Secondary>(
    message: ButtonMessage,
    primary: Arc<Primary>,
    secondary: Arc<Secondary>,
) -> StateAction<State>
where
    State: 'static,
    Primary: Fn(&mut State) + Send + Sync + 'static,
    Secondary: Fn(&mut State, Point) + Send + Sync + 'static,
{
    StateAction::new(move |state| match message {
        ButtonMessage::Activate => primary(state),
        ButtonMessage::SecondaryActivate { position } => {
            secondary(state, position);
        }
        ButtonMessage::Drag(_) => {}
    })
}

pub(super) fn click_secondary_or_drag_action<State, Primary, Secondary, Drag>(
    message: ButtonMessage,
    primary: Arc<Primary>,
    secondary: Arc<Secondary>,
    drag: Arc<Drag>,
) -> StateAction<State>
where
    State: 'static,
    Primary: Fn(&mut State) + Send + Sync + 'static,
    Secondary: Fn(&mut State) + Send + Sync + 'static,
    Drag: Fn(&mut State, DragHandleMessage) + Send + Sync + 'static,
{
    StateAction::new(move |state| match message {
        ButtonMessage::Activate => primary(state),
        ButtonMessage::SecondaryActivate { .. } => secondary(state),
        ButtonMessage::Drag(message) => drag(state, message),
    })
}

pub(super) fn click_secondary_at_or_drag_action<State, Primary, Secondary, Drag>(
    message: ButtonMessage,
    primary: Arc<Primary>,
    secondary: Arc<Secondary>,
    drag: Arc<Drag>,
) -> StateAction<State>
where
    State: 'static,
    Primary: Fn(&mut State) + Send + Sync + 'static,
    Secondary: Fn(&mut State, Point) + Send + Sync + 'static,
    Drag: Fn(&mut State, DragHandleMessage) + Send + Sync + 'static,
{
    StateAction::new(move |state| match message {
        ButtonMessage::Activate => primary(state),
        ButtonMessage::SecondaryActivate { position } => {
            secondary(state, position);
        }
        ButtonMessage::Drag(message) => drag(state, message),
    })
}
