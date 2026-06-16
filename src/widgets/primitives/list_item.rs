//! Reusable list-row and list-item primitive.

use crate::gui::automation::AutomationRole;
use crate::gui::types::Rect;
use crate::layout::LayoutOutput;
use crate::runtime::{PaintPrimitive, PaintText};
use crate::theme::ThemeTokens;

use super::support::WidgetCommon;
use crate::widgets::contract::{FocusBehavior, Widget, WidgetId, WidgetSizing};
use crate::widgets::interaction::{ListItemMessage, WidgetInput, WidgetOutput};

mod builders;
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

/// Named construction fields for [`ListItemWidget`].
#[derive(Clone, Debug, PartialEq)]
pub struct ListItemWidgetParts {
    /// Stable widget identity used by layout, events, and state synchronization.
    pub id: WidgetId,
    /// Primary row label.
    pub label: PaintText,
    /// Intrinsic row sizing contract.
    pub sizing: WidgetSizing,
}

impl ListItemWidget {
    /// Build a list-item descriptor from named identity, content, and sizing fields.
    pub fn from_parts(parts: ListItemWidgetParts) -> Self {
        let mut common = WidgetCommon::new(parts.id, parts.sizing);
        common.focus = FocusBehavior::Keyboard;
        Self {
            common,
            label: parts.label,
            detail: None,
        }
    }

    /// Build a list-item descriptor that can be focused, selected, and invoked.
    pub fn new(id: WidgetId, label: impl Into<PaintText>, sizing: WidgetSizing) -> Self {
        Self::from_parts(ListItemWidgetParts {
            id,
            label: label.into(),
            sizing,
        })
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

    fn automation_role(&self) -> AutomationRole {
        AutomationRole::Row
    }

    fn automation_label(&self) -> Option<String> {
        Some(self.label.as_str().to_owned())
    }

    fn automation_value_text(&self) -> Option<String> {
        self.detail
            .as_ref()
            .map(|detail| detail.as_str().to_owned())
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
