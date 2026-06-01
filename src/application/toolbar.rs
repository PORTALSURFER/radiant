use crate::{
    application::{ViewNode, row, spacer},
    widgets::WidgetStyle,
};

/// Horizontal placement for compact toolbar controls.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ToolbarAlignment {
    /// Place controls from the leading edge.
    #[default]
    Start,
    /// Place controls against the trailing edge.
    End,
}

/// Named construction fields for a compact toolbar or action strip.
pub struct ToolbarParts<Message> {
    /// Ordered controls shown in the toolbar.
    pub controls: Vec<ViewNode<Message>>,
    /// Horizontal control alignment inside the available width.
    pub alignment: ToolbarAlignment,
    /// Total toolbar height.
    pub height: f32,
    /// Gap between controls.
    pub spacing: f32,
    /// Horizontal toolbar padding.
    pub padding_x: f32,
    /// Vertical toolbar padding.
    pub padding_y: f32,
    /// Height used by the flexible spacer for aligned toolbars.
    pub spacer_height: f32,
    /// Semantic style applied to the toolbar row.
    pub style: WidgetStyle,
}

impl<Message> ToolbarParts<Message> {
    /// Build toolbar parts from ordered controls.
    pub fn new(controls: impl IntoIterator<Item = ViewNode<Message>>) -> Self {
        Self {
            controls: controls.into_iter().collect(),
            alignment: ToolbarAlignment::Start,
            height: 34.0,
            spacing: 4.0,
            padding_x: 0.0,
            padding_y: 3.0,
            spacer_height: 24.0,
            style: WidgetStyle::default(),
        }
    }

    /// Align controls to the trailing edge.
    pub fn align_end(mut self) -> Self {
        self.alignment = ToolbarAlignment::End;
        self
    }

    /// Set total toolbar height.
    pub fn height(mut self, height: f32) -> Self {
        self.height = height;
        self
    }

    /// Set the gap between controls.
    pub fn spacing(mut self, spacing: f32) -> Self {
        self.spacing = spacing;
        self
    }

    /// Set horizontal and vertical padding.
    pub fn padding(mut self, x: f32, y: f32) -> Self {
        self.padding_x = x;
        self.padding_y = y;
        self
    }

    /// Set the spacer height used for end-aligned controls.
    pub fn spacer_height(mut self, height: f32) -> Self {
        self.spacer_height = height;
        self
    }

    /// Set the semantic row style.
    pub fn style(mut self, style: WidgetStyle) -> Self {
        self.style = style;
        self
    }
}

/// Build a compact toolbar from ordered controls.
pub fn toolbar<Message: 'static>(
    controls: impl IntoIterator<Item = ViewNode<Message>>,
) -> ViewNode<Message> {
    toolbar_from_parts(ToolbarParts::new(controls))
}

/// Build a compact toolbar or action strip from named parts.
pub fn toolbar_from_parts<Message: 'static>(parts: ToolbarParts<Message>) -> ViewNode<Message> {
    let mut children = Vec::with_capacity(parts.controls.len() + 1);
    if parts.alignment == ToolbarAlignment::End {
        children.push(spacer().height(parts.spacer_height).fill_width());
    }
    children.extend(parts.controls);
    row(children)
        .padding_x(parts.padding_x)
        .padding_y(parts.padding_y)
        .style(parts.style)
        .spacing(parts.spacing)
        .fill_width()
        .height(parts.height)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        application::{IntoView, button},
        layout::{LayoutNode, SizeModeMain},
    };

    #[test]
    fn toolbar_aligns_controls_to_trailing_edge_with_fill_spacer() {
        let view = toolbar_from_parts(
            ToolbarParts::new([
                button("A").message(()).width(28.0).height(24.0),
                button("B").message(()).width(28.0).height(24.0),
            ])
            .align_end(),
        );

        let layout = view.into_surface().layout_node();
        let LayoutNode::Container(row) = layout else {
            panic!("toolbar should lower to a row");
        };
        assert_eq!(row.children.len(), 3);
        assert!(matches!(
            row.children[0].slot.size_main,
            SizeModeMain::Fill(_)
        ));
        assert!(matches!(
            row.children[1].slot.size_main,
            SizeModeMain::Fixed(width) if (width - 28.0).abs() < 0.01
        ));
    }
}
