use crate::{
    application::{MappedWidget, ViewNode, view_node_from_widget},
    runtime::WidgetMessageMapper,
    widgets::{InteractiveRowActions, InteractiveRowMessage},
};

use super::InteractiveRowBuilder;

impl InteractiveRowBuilder {
    /// Emit mapped host messages for row interactions.
    pub fn mapped<Message: 'static>(
        self,
        map: impl Fn(InteractiveRowMessage) -> Message + Send + Sync + 'static,
    ) -> ViewNode<Message> {
        self.with_message_mapper(WidgetMessageMapper::interactive_row(map))
    }

    /// Emit host messages for selected row interactions.
    pub fn filter_mapped<Message: 'static>(
        self,
        map: impl Fn(InteractiveRowMessage) -> Option<Message> + Send + Sync + 'static,
    ) -> ViewNode<Message> {
        self.with_message_mapper(WidgetMessageMapper::interactive_row_filtered(map))
    }

    /// Emit host messages for common row actions.
    pub fn actions<Message: 'static>(
        self,
        actions: InteractiveRowActions<Message>,
    ) -> ViewNode<Message> {
        self.filter_mapped(move |message| actions.route(message))
    }

    fn with_message_mapper<Message: 'static>(
        self,
        messages: WidgetMessageMapper<Message>,
    ) -> ViewNode<Message> {
        let style = self.style;
        let row = self.widget();
        let mut node = view_node_from_widget(MappedWidget::new(row, messages));
        node.style = style;
        node
    }
}
