use super::{InteractiveRowActionRouter, InteractiveRowActions, InteractiveRowLocalActions};
use crate::widgets::interaction::InteractiveRowMessage;

impl<Message> InteractiveRowActions<Message> {
    /// Route a generic row interaction into the configured host action.
    pub fn route(&self, message: InteractiveRowMessage) -> Option<Message> {
        InteractiveRowActionRouter::Shared(&self.router).route(message)
    }

    /// Return whether this router maps ordinary row hover.
    pub fn routes_hover(&self) -> bool {
        InteractiveRowActionRouter::Shared(&self.router).routes_hover()
    }
}

impl<Message> InteractiveRowLocalActions<Message> {
    /// Route a generic row interaction into the configured UI-local action.
    pub fn route(&self, message: InteractiveRowMessage) -> Option<Message> {
        InteractiveRowActionRouter::Local(&self.router).route(message)
    }

    /// Return whether this router maps ordinary row hover.
    pub fn routes_hover(&self) -> bool {
        InteractiveRowActionRouter::Local(&self.router).routes_hover()
    }
}
