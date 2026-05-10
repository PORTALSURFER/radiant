//! Reusable selectable surface primitive.

use crate::gui::types::Rect;
use crate::layout::LayoutOutput;
use crate::runtime::{PaintPrimitive, SurfaceNode, WidgetMessageMapper};
use crate::theme::ThemeTokens;

use super::support::WidgetCommon;
use crate::widgets::contract::{FocusBehavior, Widget, WidgetId, WidgetSizing};
use crate::widgets::interaction::{SelectableMessage, WidgetInput, WidgetOutput};

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

impl SelectableWidget {
    /// Build a selectable descriptor with the provided selected state.
    pub fn new(
        id: WidgetId,
        label: impl Into<String>,
        selected: bool,
        sizing: WidgetSizing,
    ) -> Self {
        let mut common = WidgetCommon::new(id, sizing);
        common.focus = FocusBehavior::Keyboard;
        common.state.selected = selected;
        Self {
            common,
            props: SelectableProps {
                label: label.into(),
            },
        }
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

impl<Message> WidgetMessageMapper<Message> {
    /// Build a selectable-message mapper.
    pub fn selectable(map: impl Fn(SelectableMessage) -> Message + Send + Sync + 'static) -> Self {
        Self::typed(map)
    }
}

impl<Message> SurfaceNode<Message> {
    /// Build a selectable leaf that maps selection changes by selected state.
    pub fn selectable(
        id: WidgetId,
        label: impl Into<String>,
        selected: bool,
        sizing: WidgetSizing,
        map: impl Fn(bool) -> Message + Send + Sync + 'static,
    ) -> Self {
        Self::selectable_mapped(id, label, selected, sizing, move |message| match message {
            SelectableMessage::SelectionChanged { selected } => map(selected),
        })
    }

    /// Build a selectable leaf with a custom widget-to-host message mapper.
    pub fn selectable_mapped(
        id: WidgetId,
        label: impl Into<String>,
        selected: bool,
        sizing: WidgetSizing,
        map: impl Fn(SelectableMessage) -> Message + Send + Sync + 'static,
    ) -> Self {
        Self::widget(
            SelectableWidget::new(id, label, selected, sizing),
            WidgetMessageMapper::selectable(map),
        )
    }
}
