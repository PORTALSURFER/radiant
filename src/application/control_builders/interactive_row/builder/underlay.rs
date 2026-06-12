use crate::{
    application::{ViewNode, input_underlay},
    widgets::{
        InteractiveRowActions, InteractiveRowMessage, WidgetId, WidgetStyle, stable_widget_id,
        stable_widget_id_u64,
    },
};

use super::{InteractiveRowBuilder, defaults::interactive_row};

/// Builder for arbitrary row content backed by a generic interactive row.
pub struct InteractiveRowUnderlayBuilder<Message> {
    content: ViewNode<Message>,
    row: InteractiveRowBuilder,
    input_id: Option<WidgetId>,
    style: Option<WidgetStyle>,
}

impl<Message: 'static> InteractiveRowUnderlayBuilder<Message> {
    /// Configure the backing interactive row before binding messages.
    pub fn row(
        mut self,
        configure: impl FnOnce(InteractiveRowBuilder) -> InteractiveRowBuilder,
    ) -> Self {
        self.row = configure(self.row);
        self
    }

    /// Configure the backing row as a host-tracked drop target.
    ///
    /// Use this when arbitrary visible row content should keep its own paint
    /// tree while the underlay owns generic drop and hover-drop routing.
    pub fn tracked_drop_target(mut self, drag_active: bool, active_target: bool) -> Self {
        self.row = self.row.tracked_drop_target(drag_active, active_target);
        self
    }

    /// Assign a stable widget id to the backing interactive row.
    pub fn input_id(mut self, id: WidgetId) -> Self {
        self.input_id = Some(id);
        self
    }

    /// Derive and assign a stable input widget id from a caller-owned text key.
    ///
    /// Use this for dynamic rows whose focus, hover, drag, or drop identity
    /// should survive projection changes without app-local input-id helpers.
    pub fn stable_input_id(mut self, scope: u64, key: impl AsRef<str>) -> Self {
        self.input_id = Some(stable_widget_id(scope, key));
        self
    }

    /// Derive and assign a stable input widget id from a caller-owned numeric key.
    ///
    /// Use this for dynamic rows keyed by durable numeric IDs or enum indexes
    /// without allocating temporary strings.
    pub fn stable_u64_input_id(mut self, scope: u64, key: u64) -> Self {
        self.input_id = Some(stable_widget_id_u64(scope, key));
        self
    }

    /// Apply an explicit style to the backing interactive row.
    pub fn style(mut self, style: WidgetStyle) -> Self {
        self.style = Some(style);
        self
    }

    /// Emit mapped host messages for row interactions.
    pub fn mapped(
        self,
        map: impl Fn(InteractiveRowMessage) -> Message + Send + Sync + 'static,
    ) -> ViewNode<Message> {
        let Self {
            content,
            row,
            input_id,
            style,
        } = self;
        Self::finish_parts(content, row.mapped(map), input_id, style)
    }

    /// Emit host messages for selected row interactions.
    pub fn filter_mapped(
        self,
        map: impl Fn(InteractiveRowMessage) -> Option<Message> + Send + Sync + 'static,
    ) -> ViewNode<Message> {
        let Self {
            content,
            row,
            input_id,
            style,
        } = self;
        Self::finish_parts(content, row.filter_mapped(map), input_id, style)
    }

    /// Emit host messages for common row actions.
    pub fn actions(self, actions: InteractiveRowActions<Message>) -> ViewNode<Message> {
        let Self {
            content,
            row,
            input_id,
            style,
        } = self;
        Self::finish_parts(content, row.actions(actions), input_id, style)
    }

    fn finish_parts(
        content: ViewNode<Message>,
        mut input: ViewNode<Message>,
        input_id: Option<WidgetId>,
        style: Option<WidgetStyle>,
    ) -> ViewNode<Message> {
        if let Some(id) = input_id {
            input = input.id(id);
        }
        if let Some(style) = style {
            input = input.style(style);
        }
        input_underlay(content, input)
    }
}

/// Build arbitrary visible content backed by an interactive row underlay.
///
/// The content remains visible above the row, while the backing row owns
/// activation, secondary activation, drag, drop, focus, and row feedback paint.
pub fn interactive_row_underlay<Message: 'static>(
    content: ViewNode<Message>,
) -> InteractiveRowUnderlayBuilder<Message> {
    InteractiveRowUnderlayBuilder {
        content,
        row: interactive_row(),
        input_id: None,
        style: None,
    }
}
