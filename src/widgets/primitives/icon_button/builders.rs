//! Runtime mapper helpers for icon-button primitives.

use crate::runtime::WidgetMessageMapper;
use crate::widgets::interaction::ButtonMessage;

impl<Message> WidgetMessageMapper<Message> {
    /// Build an icon-button mapper.
    pub fn icon_button(map: impl Fn(ButtonMessage) -> Message + Send + Sync + 'static) -> Self {
        Self::typed(map)
    }

    /// Build an allocation-free icon-button activation binding.
    pub(crate) fn icon_button_message(message: Message) -> Self
    where
        Message: Clone + Send + Sync + 'static,
    {
        Self::button_message(message)
    }
}
