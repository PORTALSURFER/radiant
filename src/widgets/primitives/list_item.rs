//! Reusable list-row and list-item primitive.

use crate::gui::types::Rect;
use crate::layout::LayoutOutput;
use crate::runtime::{PaintPrimitive, PaintText, SurfaceNode, WidgetMessageMapper};
use crate::theme::ThemeTokens;

use super::support::WidgetCommon;
use crate::widgets::contract::{FocusBehavior, Widget, WidgetId, WidgetSizing};
use crate::widgets::interaction::{ListItemMessage, WidgetInput, WidgetOutput};

mod input;
mod paint;

/// Public list-row or list-item primitive.
#[derive(Clone, Debug, PartialEq)]
pub struct ListItemWidget {
    /// Shared widget contract.
    pub common: WidgetCommon,
    /// Primary row label.
    pub label: PaintText,
    /// Optional secondary text.
    pub detail: Option<PaintText>,
}

impl ListItemWidget {
    /// Build a list-item descriptor that can be focused, selected, and invoked.
    pub fn new(id: WidgetId, label: impl Into<PaintText>, sizing: WidgetSizing) -> Self {
        let mut common = WidgetCommon::new(id, sizing);
        common.focus = FocusBehavior::Keyboard;
        Self {
            common,
            label: label.into(),
            detail: None,
        }
    }

    /// Route one backend-neutral interaction into the list item.
    pub fn handle_input(&mut self, bounds: Rect, input: WidgetInput) -> Option<ListItemMessage> {
        input::handle_list_item_input(self, bounds, input)
    }
}

impl Widget for ListItemWidget {
    fn common(&self) -> &WidgetCommon {
        &self.common
    }

    fn common_mut(&mut self) -> &mut WidgetCommon {
        &mut self.common
    }

    fn handle_input(&mut self, bounds: Rect, input: WidgetInput) -> Option<WidgetOutput> {
        ListItemWidget::handle_input(self, bounds, input).map(WidgetOutput::typed)
    }

    fn accepts_pointer_move(&self) -> bool {
        false
    }

    fn append_paint(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        bounds: Rect,
        _layout: &LayoutOutput,
        theme: &ThemeTokens,
    ) {
        paint::push_list_item_widget_paint(primitives, self, bounds, theme);
    }
}

impl<Message> WidgetMessageMapper<Message> {
    /// Build a list-item-message mapper.
    pub fn list_item(map: impl Fn(ListItemMessage) -> Message + Send + Sync + 'static) -> Self {
        Self::typed(map)
    }
}

impl<Message> SurfaceNode<Message> {
    /// Build a non-emitting list item leaf node.
    pub fn list_item(id: WidgetId, label: impl Into<String>, sizing: WidgetSizing) -> Self {
        Self::static_widget(ListItemWidget::new(
            id,
            PaintText::from(label.into()),
            sizing,
        ))
    }

    /// Build an invoking list item leaf node that emits one cloned host message.
    pub fn list_item_action(
        id: WidgetId,
        label: impl Into<String>,
        sizing: WidgetSizing,
        message: Message,
    ) -> Self
    where
        Message: Clone + Send + Sync + 'static,
    {
        Self::list_item_mapped(id, label, sizing, move |_| message.clone())
    }

    /// Build an invoking list item leaf node with a custom widget-to-host message mapper.
    pub fn list_item_mapped(
        id: WidgetId,
        label: impl Into<String>,
        sizing: WidgetSizing,
        map: impl Fn(ListItemMessage) -> Message + Send + Sync + 'static,
    ) -> Self {
        Self::widget(
            ListItemWidget::new(id, PaintText::from(label.into()), sizing),
            WidgetMessageMapper::list_item(map),
        )
    }
}
