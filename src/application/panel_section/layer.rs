use super::{
    PanelSectionParts, chrome::closeable_panel_section_from_parts, chrome::panel_section_from_parts,
};
use crate::{
    application::{
        AnchoredLayerParts, LayerHorizontalAnchor, LayerVerticalAnchor, ViewNode,
        anchored_layer_from_parts,
    },
    layout::Vector2,
};

/// Named construction fields for a fixed-size panel section in an anchored layer.
pub struct PanelSectionLayerParts<Message> {
    /// Panel-section content and chrome configuration.
    pub panel: PanelSectionParts<Message>,
    /// Fixed foreground panel size.
    pub size: Vector2,
    /// Horizontal placement policy inside the parent layer.
    pub horizontal: LayerHorizontalAnchor,
    /// Vertical placement policy inside the parent layer.
    pub vertical: LayerVerticalAnchor,
    /// Horizontal inset from the selected horizontal anchor.
    pub inset_x: f32,
    /// Vertical inset from the selected vertical anchor.
    pub inset_y: f32,
}

impl<Message> PanelSectionLayerParts<Message> {
    /// Build anchored panel-section parts.
    pub fn new(panel: PanelSectionParts<Message>, size: Vector2) -> Self {
        Self {
            panel,
            size,
            horizontal: LayerHorizontalAnchor::Center,
            vertical: LayerVerticalAnchor::Center,
            inset_x: 0.0,
            inset_y: 0.0,
        }
    }

    /// Set the horizontal anchor.
    pub fn horizontal(mut self, anchor: LayerHorizontalAnchor) -> Self {
        self.horizontal = anchor;
        self
    }

    /// Set the vertical anchor.
    pub fn vertical(mut self, anchor: LayerVerticalAnchor) -> Self {
        self.vertical = anchor;
        self
    }

    /// Set both edge insets.
    pub fn inset(mut self, x: f32, y: f32) -> Self {
        self.inset_x = x.max(0.0);
        self.inset_y = y.max(0.0);
        self
    }
}

/// Build a fixed-size panel section in a parent-anchored layer.
pub fn panel_section_layer_from_parts<Message: 'static>(
    parts: PanelSectionLayerParts<Message>,
) -> ViewNode<Message> {
    let size = sanitized_layer_size(parts.size);
    let panel = panel_section_from_parts(parts.panel.height(size.y))
        .width(size.x)
        .height(size.y);
    panel_section_layer(
        panel,
        size,
        parts.horizontal,
        parts.vertical,
        parts.inset_x,
        parts.inset_y,
    )
}

/// Build a closeable fixed-size panel section in a parent-anchored layer.
pub fn closeable_panel_section_layer_from_parts<Message>(
    parts: PanelSectionLayerParts<Message>,
    close_message: Message,
) -> ViewNode<Message>
where
    Message: Clone + Send + Sync + 'static,
{
    let size = sanitized_layer_size(parts.size);
    let panel = closeable_panel_section_from_parts(parts.panel.height(size.y), close_message)
        .width(size.x)
        .height(size.y);
    panel_section_layer(
        panel,
        size,
        parts.horizontal,
        parts.vertical,
        parts.inset_x,
        parts.inset_y,
    )
}

fn panel_section_layer<Message: 'static>(
    panel: ViewNode<Message>,
    size: Vector2,
    horizontal: LayerHorizontalAnchor,
    vertical: LayerVerticalAnchor,
    inset_x: f32,
    inset_y: f32,
) -> ViewNode<Message> {
    anchored_layer_from_parts(
        AnchoredLayerParts::new(panel, size)
            .horizontal(horizontal)
            .vertical(vertical)
            .inset(inset_x, inset_y),
    )
}

fn sanitized_layer_size(size: Vector2) -> Vector2 {
    Vector2::new(size.x.max(1.0), size.y.max(1.0))
}
