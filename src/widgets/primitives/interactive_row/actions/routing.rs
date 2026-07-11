use super::InteractiveRowActions;
use crate::widgets::interaction::InteractiveRowMessage;

impl<Message> InteractiveRowActions<Message> {
    /// Route a generic row interaction into the configured host action.
    pub fn route(&self, message: InteractiveRowMessage) -> Option<Message> {
        if let Some(position) = message.hover_position() {
            return self.hover.as_ref().map(|callback| callback(position));
        }
        if let Some(position) = message.secondary_position() {
            return self.secondary.as_ref().map(|callback| callback(position));
        }
        if let Some(drag) = message.drag_message() {
            return self.drag.as_ref().map(|callback| callback(drag));
        }
        if message.is_drop() {
            return self.drop.as_ref().map(|callback| callback());
        }
        if let Some(position) = message.hover_drop_position() {
            return self.hover_drop.as_ref().map(|callback| callback(position));
        }
        if let Some(position) = message.clear_drop_position() {
            return self.clear_drop.as_ref().map(|callback| callback(position));
        }
        if message.is_double_activation() {
            return self.double_activate.as_ref().map(|callback| callback());
        }
        if let Some(modifiers) = message.single_activation_modifiers() {
            if let Some(callback) = &self.activate_with_modifiers {
                return Some(callback(modifiers));
            }
            return self.activate.as_ref().map(|callback| callback());
        }
        None
    }

    /// Return whether this router maps ordinary row hover.
    pub fn routes_hover(&self) -> bool {
        self.hover.is_some()
    }
}
