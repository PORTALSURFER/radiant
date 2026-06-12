use super::geometry::{
    DEFAULT_PANEL_SECTION_PADDING, DEFAULT_PANEL_SECTION_SPACING,
    DEFAULT_PANEL_SECTION_TITLE_HEIGHT, PanelSectionGeometry,
};
use crate::{
    application::{ViewNode, drag_handle_mapped},
    widgets::{DragHandleMessage, WidgetProminence, WidgetStyle, WidgetTone},
};

const DEFAULT_PANEL_SECTION_HEADER_SPACING: f32 = 4.0;
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

    /// Return reusable panel-section geometry for these parts.
    pub fn geometry(&self) -> PanelSectionGeometry {
        PanelSectionGeometry::new()
            .padding(self.padding)
            .spacing(self.spacing)
            .title_height(self.title_height)
    }

    /// Return the vertical offset from the panel's top edge to the content area.
    pub fn content_top_offset(&self) -> f32 {
        self.geometry().content_top_offset()
    }

    /// Return the vertical inset from the panel's bottom edge to the content top.
    pub fn content_top_inset_from_bottom(&self, panel_height: f32) -> f32 {
        self.geometry().content_top_inset_from_bottom(panel_height)
    }

    /// Return the vertical inset from the panel's bottom edge to the content bottom.
    pub fn content_bottom_inset(&self) -> f32 {
        self.geometry().content_bottom_inset()
    }

    /// Return the total section height needed for a fixed content height.
    pub fn section_height_for_content_height(&self, content_height: f32) -> f32 {
        self.geometry()
            .section_height_for_content_height(content_height)
    }

    /// Return the content height available inside a fixed section height.
    pub fn content_height_for_section_height(&self, section_height: f32) -> f32 {
        self.geometry()
            .content_height_for_section_height(section_height)
    }
}
