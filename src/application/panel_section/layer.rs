use super::{
    PanelSectionParts, chrome::closeable_panel_section_from_parts, chrome::panel_section_from_parts,
};
use crate::{
    application::{
        AnchoredLayerParts, LayerHorizontalAnchor, LayerVerticalAnchor, TextContent, ViewNode,
        anchored_layer_from_parts,
    },
    layout::Vector2,
    widgets::WidgetTone,
};

/// Named construction fields for a fixed-size dialog panel in an anchored layer.
pub struct DialogLayerParts<Message> {
    /// Dialog title shown in the standard panel header.
    pub title: TextContent,
    /// Main dialog content.
    pub content: ViewNode<Message>,
    /// Visual tone applied to Radiant's standard strong dialog chrome.
    pub tone: WidgetTone,
    /// Fixed foreground dialog size.
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

impl<Message> DialogLayerParts<Message> {
    /// Build anchored dialog-layer parts with Radiant's standard dialog chrome.
    pub fn new(
        title: impl Into<TextContent>,
        content: ViewNode<Message>,
        tone: WidgetTone,
        size: Vector2,
    ) -> Self {
        Self {
            title: title.into(),
            content,
            tone,
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

    fn into_panel_layer_parts(self) -> PanelSectionLayerParts<Message> {
        PanelSectionLayerParts::new(
            PanelSectionParts::dialog(self.title, self.content, self.tone),
            self.size,
        )
        .horizontal(self.horizontal)
        .vertical(self.vertical)
        .inset(self.inset_x, self.inset_y)
    }
}

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

/// Build a fixed-size dialog panel in a parent-anchored layer.
///
/// Use this for modal dialogs, popovers, and floating utility panels that want
/// Radiant's standard dialog panel chrome without spelling out the lower-level
/// panel-section and anchored-layer parts.
pub fn dialog_layer<Message: 'static>(
    title: impl Into<TextContent>,
    content: ViewNode<Message>,
    tone: WidgetTone,
    size: Vector2,
) -> ViewNode<Message> {
    dialog_layer_from_parts(DialogLayerParts::new(title, content, tone, size))
}

/// Build a fixed-size dialog panel in a parent-anchored layer.
pub fn dialog_layer_from_parts<Message: 'static>(
    parts: DialogLayerParts<Message>,
) -> ViewNode<Message> {
    panel_section_layer_from_parts(parts.into_panel_layer_parts())
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

/// Build a closeable fixed-size dialog panel in a parent-anchored layer.
///
/// This convenience covers the common app-modal case while preserving
/// [`closeable_panel_section_layer_from_parts`] for callers that need custom
/// panel parts, anchoring, or insets.
pub fn closeable_dialog_layer<Message>(
    title: impl Into<TextContent>,
    content: ViewNode<Message>,
    tone: WidgetTone,
    size: Vector2,
    close_message: Message,
) -> ViewNode<Message>
where
    Message: Clone + Send + Sync + 'static,
{
    closeable_dialog_layer_from_parts(
        DialogLayerParts::new(title, content, tone, size),
        close_message,
    )
}

/// Build a closeable fixed-size dialog panel in a parent-anchored layer.
pub fn closeable_dialog_layer_from_parts<Message>(
    parts: DialogLayerParts<Message>,
    close_message: Message,
) -> ViewNode<Message>
where
    Message: Clone + Send + Sync + 'static,
{
    closeable_panel_section_layer_from_parts(parts.into_panel_layer_parts(), close_message)
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
