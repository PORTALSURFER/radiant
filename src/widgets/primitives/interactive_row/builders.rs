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

    /// Build an interactive-row message mapper that can ignore selected row events.
    pub fn interactive_row_filtered(
        map: impl Fn(InteractiveRowMessage) -> Option<Message> + Send + Sync + 'static,
    ) -> Self {
        Self::dynamic(move |output| {
            output
                .typed_copied::<InteractiveRowMessage>()
                .and_then(&map)
        })
    }
}
