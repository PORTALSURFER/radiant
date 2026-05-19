//! Runtime mapper helpers for interactive-row primitives.

use crate::runtime::WidgetMessageMapper;
use crate::widgets::interaction::InteractiveRowMessage;

impl<Message> WidgetMessageMapper<Message> {
    /// Build an interactive-row message mapper.
    pub fn interactive_row(
        map: impl Fn(InteractiveRowMessage) -> Message + Send + Sync + 'static,
    ) -> Self {
        Self::typed(map)
    }
}
