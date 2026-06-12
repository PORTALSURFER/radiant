use super::PanelSectionParts;
use crate::application::{ViewNode, close_button, column, row, text};

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
