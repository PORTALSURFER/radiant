//! Reusable selectable surface primitive.

use crate::gui::types::Rect;
use crate::layout::LayoutOutput;
use crate::runtime::{PaintPrimitive, PaintText};
use crate::theme::ThemeTokens;

use super::support::WidgetCommon;
use crate::widgets::contract::{FocusBehavior, Widget, WidgetId, WidgetSizing};
use crate::widgets::interaction::{SelectableMessage, WidgetInput, WidgetOutput};

mod builders;
mod input;
mod model;
mod paint;

pub use model::SelectableProps;

/// Public selectable primitive for cards, rows, tiles, and options.
#[derive(Clone, Debug, PartialEq)]
pub struct SelectableWidget {
    /// Shared widget contract.
    pub common: WidgetCommon,
    /// Immutable user-facing selectable configuration.
    pub props: SelectableProps,
}

/// Named construction fields for [`SelectableWidget`].
#[derive(Clone, Debug, PartialEq)]
pub struct SelectableWidgetParts {
    /// Stable widget identity used by layout, events, and state synchronization.
    pub id: WidgetId,
    /// User-facing selectable label.
    pub label: PaintText,
    /// Initial selected state.
    pub selected: bool,
    /// Intrinsic selectable sizing contract.
    pub sizing: WidgetSizing,
}

impl SelectableWidget {
    /// Build a selectable descriptor from named identity, content, state, and sizing fields.
    pub fn from_parts(parts: SelectableWidgetParts) -> Self {
        let mut common = WidgetCommon::new(parts.id, parts.sizing);
        common.focus = FocusBehavior::Keyboard;
        common.state.selected = parts.selected;
        Self {
            common,
            props: SelectableProps { label: parts.label },
        }
    }

    /// Build a selectable descriptor with the provided selected state.
    pub fn new(
        id: WidgetId,
        label: impl Into<PaintText>,
        selected: bool,
        sizing: WidgetSizing,
    ) -> Self {
        Self::from_parts(SelectableWidgetParts {
            id,
            label: label.into(),
            selected,
            sizing,
        })
    }

    /// Route one backend-neutral interaction into the selectable.
    pub fn handle_input(&mut self, bounds: Rect, input: WidgetInput) -> Option<SelectableMessage> {
        input::handle_selectable_input(self, bounds, input)
    }
}

impl Widget for SelectableWidget {
    fn common(&self) -> &WidgetCommon {
        &self.common
    }

    fn common_mut(&mut self) -> &mut WidgetCommon {
        &mut self.common
    }

    fn handle_input(&mut self, bounds: Rect, input: WidgetInput) -> Option<WidgetOutput> {
        SelectableWidget::handle_input(self, bounds, input).map(WidgetOutput::typed)
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
        paint::push_selectable_widget_paint(primitives, self, bounds, theme);
    }
}
