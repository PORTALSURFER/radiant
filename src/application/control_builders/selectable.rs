use crate::{
    application::{
        MappedWidget, TextContent, ViewNode, danger_style, default_selectable_sizing,
        primary_style, view_node_from_widget,
    },
    gui::types::Rgba8,
    runtime::{PaintText, WidgetMessageMapper},
    widgets::{
        ColorMarkerAlign, ColorMarkerProps, SelectableMessage, SelectableWidget, WidgetProminence,
        WidgetStyle,
    },
};

/// Builder for selectable controls that emit explicit host messages.
pub struct SelectableBuilder {
    label: PaintText,
    selected: bool,
    style: Option<WidgetStyle>,
    color_marker: Option<ColorMarkerProps>,
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

    /// Paint a passive color marker inside the selectable.
    pub fn color_marker(mut self, color: Option<Rgba8>) -> Self {
        let mut props = self
            .color_marker
            .unwrap_or_else(|| ColorMarkerProps::new(color));
        props.color = color;
        self.color_marker = Some(props);
        self
    }

    /// Paint a passive color marker with explicit marker geometry.
    pub fn color_marker_props(mut self, props: ColorMarkerProps) -> Self {
        self.color_marker = Some(props);
        self
    }

    /// Set the selectable color-marker side length.
    pub fn color_marker_side(self, side: u8) -> Self {
        self.map_color_marker_props(|props| props.side(side))
    }

    /// Set the selectable color-marker horizontal inset.
    pub fn color_marker_inset(self, inset: u8) -> Self {
        self.map_color_marker_props(|props| props.inset(inset))
    }

    /// Set the selectable color-marker horizontal alignment.
    pub fn color_marker_align(self, align: ColorMarkerAlign) -> Self {
        self.map_color_marker_props(|props| props.align(align))
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
        let SelectableBuilder {
            label,
            selected,
            style,
            color_marker,
        } = self;
        let sizing = default_selectable_sizing(&label);
        let mut widget = SelectableWidget::new(0, label, selected, sizing);
        if let Some(props) = color_marker {
            widget = widget.with_color_marker_props(props);
        }
        let mut node = view_node_from_widget(MappedWidget::new(
            widget,
            WidgetMessageMapper::selectable(map),
        ));
        node.style = style;
        node
    }

    fn map_color_marker_props(
        mut self,
        map: impl FnOnce(ColorMarkerProps) -> ColorMarkerProps,
    ) -> Self {
        let props = self
            .color_marker
            .unwrap_or_else(|| ColorMarkerProps::new(None));
        self.color_marker = Some(map(props));
        self
    }
}

/// Build a selectable control.
pub fn selectable(label: impl Into<TextContent>, selected: bool) -> SelectableBuilder {
    SelectableBuilder {
        label: label.into().into_paint_text(),
        selected,
        style: None,
        color_marker: None,
    }
}

/// Build a selectable control that maps value changes by selected state.
pub fn selectable_mapped<Message: 'static>(
    label: impl Into<TextContent>,
    selected: bool,
    map: impl Fn(bool) -> Message + Send + Sync + 'static,
) -> ViewNode<Message> {
    selectable(label, selected).message(map)
}
