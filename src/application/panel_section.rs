use crate::{
    application::{
        AnchoredLayerParts, LayerHorizontalAnchor, LayerVerticalAnchor, ViewNode,
        anchored_layer_from_parts, close_button, column, row, text,
    },
    layout::Vector2,
    widgets::{WidgetProminence, WidgetStyle, WidgetTone},
};

const DEFAULT_PANEL_SECTION_PADDING: f32 = 6.0;
const DEFAULT_PANEL_SECTION_SPACING: f32 = 4.0;
const DEFAULT_PANEL_SECTION_HEADER_SPACING: f32 = 4.0;
const DEFAULT_PANEL_SECTION_TITLE_HEIGHT: f32 = 20.0;

/// Named construction fields for a compact titled panel section.
pub struct PanelSectionParts<Message> {
    /// Section title shown in the leading header area.
    pub title: String,
    /// Main section content.
    pub content: ViewNode<Message>,
    /// Optional trailing header content such as an action button or drag handle.
    pub trailing: Option<ViewNode<Message>>,
    /// Optional fixed section height. When omitted the section uses intrinsic height.
    pub height: Option<f32>,
    /// Visual styling applied to the section container.
    pub style: WidgetStyle,
    /// Inner container padding.
    pub padding: f32,
    /// Vertical spacing between the header and content.
    pub spacing: f32,
    /// Horizontal spacing between the title and trailing header content.
    pub header_spacing: f32,
    /// Fixed title/header row height.
    pub title_height: f32,
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

impl<Message> PanelSectionParts<Message> {
    /// Build titled panel-section parts with Radiant's compact neutral defaults.
    pub fn new(title: impl Into<String>, content: ViewNode<Message>) -> Self {
        Self {
            title: title.into(),
            content,
            trailing: None,
            height: None,
            style: WidgetStyle {
                tone: WidgetTone::Neutral,
                prominence: WidgetProminence::Subtle,
            },
            padding: DEFAULT_PANEL_SECTION_PADDING,
            spacing: DEFAULT_PANEL_SECTION_SPACING,
            header_spacing: DEFAULT_PANEL_SECTION_HEADER_SPACING,
            title_height: DEFAULT_PANEL_SECTION_TITLE_HEIGHT,
        }
    }

    /// Set fixed section height.
    pub fn height(mut self, height: f32) -> Self {
        self.height = Some(height);
        self
    }

    /// Add trailing header content such as a compact action button.
    pub fn trailing(mut self, trailing: ViewNode<Message>) -> Self {
        self.trailing = Some(trailing);
        self
    }

    /// Override section container style.
    pub fn style(mut self, style: WidgetStyle) -> Self {
        self.style = style;
        self
    }

    /// Override inner container padding.
    pub fn padding(mut self, padding: f32) -> Self {
        self.padding = padding;
        self
    }

    /// Override vertical spacing between the header and content.
    pub fn spacing(mut self, spacing: f32) -> Self {
        self.spacing = spacing;
        self
    }

    /// Override horizontal spacing inside the header row.
    pub fn header_spacing(mut self, spacing: f32) -> Self {
        self.header_spacing = spacing;
        self
    }

    /// Override fixed title/header row height.
    pub fn title_height(mut self, height: f32) -> Self {
        self.title_height = height;
        self
    }
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

/// Build a compact titled panel section with Radiant's neutral panel defaults.
pub fn panel_section<Message: 'static>(
    title: impl Into<String>,
    content: ViewNode<Message>,
    height: f32,
) -> ViewNode<Message> {
    panel_section_from_parts(PanelSectionParts::new(title, content).height(height))
}

/// Build a compact titled panel section from named parts.
pub fn panel_section_from_parts<Message: 'static>(
    parts: PanelSectionParts<Message>,
) -> ViewNode<Message> {
    let header = panel_section_header(
        parts.title,
        parts.trailing,
        parts.title_height,
        parts.header_spacing,
    );
    let mut section = column([header, parts.content])
        .style(parts.style)
        .padding(parts.padding)
        .spacing(parts.spacing)
        .fill_width();
    if let Some(height) = parts.height {
        section = section.height(height);
    }
    section
}

/// Build a fixed-size panel section in a parent-anchored layer.
pub fn panel_section_layer_from_parts<Message: 'static>(
    parts: PanelSectionLayerParts<Message>,
) -> ViewNode<Message> {
    let size = Vector2::new(parts.size.x.max(1.0), parts.size.y.max(1.0));
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

/// Build a closeable compact titled panel section from named parts.
pub fn closeable_panel_section_from_parts<Message>(
    parts: PanelSectionParts<Message>,
    close_message: Message,
) -> ViewNode<Message>
where
    Message: Clone + Send + Sync + 'static,
{
    panel_section_from_parts(
        parts.trailing(
            close_button()
                .subtle()
                .message(close_message)
                .width(24.0)
                .height(20.0),
        ),
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
    let size = Vector2::new(parts.size.x.max(1.0), parts.size.y.max(1.0));
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

fn panel_section_header<Message: 'static>(
    title: String,
    trailing: Option<ViewNode<Message>>,
    height: f32,
    spacing: f32,
) -> ViewNode<Message> {
    let title = text(title).height(height).fill_width();
    match trailing {
        Some(trailing) => row([title, trailing])
            .spacing(spacing)
            .fill_width()
            .height(height),
        None => title.fill_width().height(height),
    }
}
