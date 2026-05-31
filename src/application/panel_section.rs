use crate::{
    application::{ViewNode, column, row, text},
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
