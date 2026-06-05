use crate::{
    application::{
        AnchoredLayerParts, LayerHorizontalAnchor, LayerVerticalAnchor, ViewNode,
        anchored_layer_from_parts, close_button, column, drag_handle_mapped, row, text,
    },
    layout::Vector2,
    widgets::{DragHandleMessage, WidgetProminence, WidgetStyle, WidgetTone},
};

const DEFAULT_PANEL_SECTION_PADDING: f32 = 6.0;
const DEFAULT_PANEL_SECTION_SPACING: f32 = 4.0;
const DEFAULT_PANEL_SECTION_HEADER_SPACING: f32 = 4.0;
const DEFAULT_PANEL_SECTION_TITLE_HEIGHT: f32 = 20.0;
const DEFAULT_PANEL_SECTION_RESIZE_HANDLE_WIDTH: f32 = 26.0;
const DEFAULT_PANEL_SECTION_RESIZE_HANDLE_HEIGHT: f32 = 18.0;

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

    /// Add Radiant's compact trailing resize handle to the section header.
    ///
    /// The host still owns durable panel size, resize constraints, and the
    /// reducer message. This helper only centralizes the common header control
    /// used by resizable panel sections.
    pub fn trailing_resize_handle<Map>(self, key: impl ToString, map: Map) -> Self
    where
        Message: 'static,
        Map: Fn(DragHandleMessage) -> Message + Send + Sync + 'static,
    {
        self.trailing(drag_handle_mapped(map).key(key).size(
            DEFAULT_PANEL_SECTION_RESIZE_HANDLE_WIDTH,
            DEFAULT_PANEL_SECTION_RESIZE_HANDLE_HEIGHT,
        ))
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

    /// Return the vertical offset from the panel's top edge to the content area.
    pub fn content_top_offset(&self) -> f32 {
        sanitized_panel_metric(self.padding)
            + sanitized_panel_metric(self.title_height)
            + sanitized_panel_metric(self.spacing)
    }

    /// Return the vertical inset from the panel's bottom edge to the content top.
    pub fn content_top_inset_from_bottom(&self, panel_height: f32) -> f32 {
        (sanitized_panel_metric(panel_height) - self.content_top_offset()).max(0.0)
    }

    /// Return the vertical inset from the panel's bottom edge to the content bottom.
    pub fn content_bottom_inset(&self) -> f32 {
        sanitized_panel_metric(self.padding)
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

fn sanitized_panel_metric(value: f32) -> f32 {
    if value.is_finite() {
        value.max(0.0)
    } else {
        0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{application::IntoView, layout::Vector2};

    #[test]
    fn panel_section_parts_adds_trailing_resize_handle() {
        let parts = PanelSectionParts::new("Inspector", text("Body"))
            .trailing_resize_handle("inspector-resize", |_| "resize")
            .height(80.0);

        assert!(parts.trailing.is_some());

        let frame = panel_section_from_parts(parts)
            .view_frame_at_size_with_default_theme(Vector2::new(240.0, 120.0));
        assert!(frame.paint_plan.contains_text("Inspector"));
    }

    #[test]
    fn panel_section_parts_exposes_content_offsets() {
        let parts: PanelSectionParts<()> = PanelSectionParts::new("Inspector", text("Body"))
            .padding(8.0)
            .title_height(22.0)
            .spacing(5.0);

        assert_eq!(parts.content_top_offset(), 35.0);
        assert_eq!(parts.content_top_inset_from_bottom(120.0), 85.0);
        assert_eq!(parts.content_bottom_inset(), 8.0);
    }

    #[test]
    fn panel_section_content_offsets_sanitize_invalid_inputs() {
        let parts: PanelSectionParts<()> = PanelSectionParts::new("Inspector", text("Body"))
            .padding(f32::NAN)
            .title_height(f32::INFINITY)
            .spacing(-4.0);

        assert_eq!(parts.content_top_offset(), 0.0);
        assert_eq!(parts.content_top_inset_from_bottom(f32::NAN), 0.0);
        assert_eq!(parts.content_bottom_inset(), 0.0);
    }
}
