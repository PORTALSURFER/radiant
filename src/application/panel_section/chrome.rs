use super::{PanelSectionHeaderParts, PanelSectionParts};
use crate::{
    application::{TextContent, ViewNode, close_button, column, drag_handle, row, text},
    widgets::{DragHandleMessage, WidgetStyle, WidgetTone},
};

/// Build a compact titled panel section with Radiant's neutral panel defaults.
pub fn panel_section<Message: 'static>(
    title: impl Into<TextContent>,
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
        .padding(parts.padding)
        .spacing(parts.spacing)
        .fill_width();
    if parts.chrome {
        section = section.style(parts.style);
    }
    if let Some(height) = parts.height {
        section = section.height(height);
    }
    section
}

/// Build a compact panel section with an app-provided header view.
pub fn panel_section_from_header_parts<Message: 'static>(
    parts: PanelSectionHeaderParts<Message>,
) -> ViewNode<Message> {
    let mut section = column([parts.header, parts.content])
        .padding(parts.padding)
        .spacing(parts.spacing)
        .fill_width();
    if parts.chrome {
        section = section.style(parts.style);
    }
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

/// Build a full-width compact resize header for collapsible panel sections.
///
/// This is useful when the whole header strip should be the resize hit target
/// while host state keeps owning size, collapse policy, and resize messages.
pub fn panel_section_resize_header<Message: 'static>(
    key: impl ToString,
    height: f32,
    map: impl Fn(DragHandleMessage) -> Message + Send + Sync + 'static,
) -> ViewNode<Message> {
    drag_handle()
        .hover_chrome_only()
        .mapped(map)
        .key(key)
        .style(WidgetStyle::subtle(WidgetTone::Accent))
        .fill_width()
        .height(height)
}

fn panel_section_header<Message: 'static>(
    title: TextContent,
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
