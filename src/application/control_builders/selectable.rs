use crate::{
    application::{
        MappedWidget, ViewNode, compatibility::StateAction, danger_style,
        default_selectable_sizing, primary_style, view_node_from_widget,
    },
    runtime::{PaintText, WidgetMessageMapper},
    widgets::{SelectableMessage, SelectableWidget, WidgetProminence, WidgetStyle},
};
use std::sync::Arc;

/// Builder for selectable controls that can emit messages or mutate state directly.
pub struct SelectableBuilder {
    label: PaintText,
    selected: bool,
    style: Option<WidgetStyle>,
}

impl SelectableBuilder {
    /// Apply an explicit widget style before binding this selectable.
    pub fn style(mut self, style: WidgetStyle) -> Self {
        self.style = Some(style);
        self
    }

    /// Use the accent tone and strong prominence.
    pub fn primary(self) -> Self {
        self.style(primary_style())
    }

    /// Use the danger tone.
    pub fn danger(self) -> Self {
        self.style(danger_style())
    }

    /// Use a lower-prominence treatment.
    pub fn subtle(mut self) -> Self {
        let mut style = self.style.unwrap_or_default();
        style.prominence = WidgetProminence::Subtle;
        self.style = Some(style);
        self
    }

    /// Emit a host message mapped from selected state.
    pub fn message<Message: 'static>(
        self,
        map: impl Fn(bool) -> Message + Send + Sync + 'static,
    ) -> ViewNode<Message> {
        self.mapped(move |message| match message {
            SelectableMessage::SelectionChanged { selected } => map(selected),
        })
    }

    /// Emit a mapped host message when selection changes.
    pub fn mapped<Message: 'static>(
        self,
        map: impl Fn(SelectableMessage) -> Message + Send + Sync + 'static,
    ) -> ViewNode<Message> {
        let sizing = default_selectable_sizing(&self.label);
        let mut node = view_node_from_widget(MappedWidget::new(
            SelectableWidget::new(0, self.label, self.selected, sizing),
            WidgetMessageMapper::selectable(map),
        ));
        node.style = self.style;
        node
    }

    /// Mutate application state directly when selected state changes.
    pub fn on_change<State: 'static>(
        self,
        apply: impl Fn(&mut State, bool) + Send + Sync + 'static,
    ) -> ViewNode<StateAction<State>> {
        let apply = Arc::new(apply);
        self.message(move |selected| {
            let apply = Arc::clone(&apply);
            StateAction::new(move |state| apply(state, selected))
        })
    }
}

/// Build a selectable control.
pub fn selectable(label: impl Into<String>, selected: bool) -> SelectableBuilder {
    SelectableBuilder {
        label: PaintText::from(label.into()),
        selected,
        style: None,
    }
}

/// Build a selectable control that maps value changes by selected state.
pub fn selectable_mapped<Message: 'static>(
    label: impl Into<String>,
    selected: bool,
    map: impl Fn(bool) -> Message + Send + Sync + 'static,
) -> ViewNode<Message> {
    selectable(label, selected).message(map)
}
