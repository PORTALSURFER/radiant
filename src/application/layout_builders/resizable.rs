use crate::{
    application::{ViewNode, drag_handle, row},
    widgets::{DragHandleMessage, WidgetStyle, WidgetTone},
};

const DEFAULT_RESIZE_HANDLE_HIT_WIDTH: f32 = 5.0;
const DEFAULT_RESIZE_HANDLE_INSET: f32 = 1.0;

/// Builder for content with a trailing resize drag handle.
pub struct ResizableBuilder<Message> {
    content: ViewNode<Message>,
    handle_width: f32,
    handle_inset: f32,
    handle_key: Option<String>,
    handle_style: Option<WidgetStyle>,
    hover_chrome_only: bool,
}

impl<Message: 'static> ResizableBuilder<Message> {
    /// Set the trailing resize handle hit width.
    pub fn handle_width(mut self, width: f32) -> Self {
        self.handle_width = width.max(0.0);
        self
    }

    /// Set padding around the trailing resize handle chrome.
    pub fn handle_inset(mut self, inset: f32) -> Self {
        self.handle_inset = inset.max(0.0);
        self
    }

    /// Assign a stable key to the trailing resize handle.
    pub fn handle_key(mut self, key: impl ToString) -> Self {
        self.handle_key = Some(key.to_string());
        self
    }

    /// Style the trailing resize handle.
    pub fn handle_style(mut self, style: WidgetStyle) -> Self {
        self.handle_style = Some(style);
        self
    }

    /// Paint handle chrome only while hovered, pressed, or focused.
    pub fn hover_chrome_only(mut self) -> Self {
        self.hover_chrome_only = true;
        self
    }

    /// Finish the resizable content with a mapped trailing resize handle.
    pub fn resize_handle(
        self,
        map: impl Fn(DragHandleMessage) -> Message + Send + Sync + 'static,
    ) -> ViewNode<Message> {
        let mut handle_builder = drag_handle();
        if self.hover_chrome_only {
            handle_builder = handle_builder.hover_chrome_only();
        }

        let mut handle = handle_builder
            .mapped(map)
            .width(self.handle_width)
            .fill_height()
            .padding(self.handle_inset);

        if let Some(key) = self.handle_key {
            handle = handle.key(key);
        }
        if let Some(style) = self.handle_style {
            handle = handle.style(style);
        }

        row([self.content, handle]).spacing(0.0).fill_height()
    }

    /// Finish with Radiant's standard subtle trailing resize handle.
    pub fn subtle_resize_handle(
        self,
        key: impl ToString,
        map: impl Fn(DragHandleMessage) -> Message + Send + Sync + 'static,
    ) -> ViewNode<Message> {
        self.hover_chrome_only()
            .handle_key(key)
            .handle_style(WidgetStyle::subtle(WidgetTone::Accent))
            .resize_handle(map)
    }
}

/// Wrap content with a configurable trailing resize drag handle.
pub fn resizable<Message>(content: ViewNode<Message>) -> ResizableBuilder<Message> {
    ResizableBuilder {
        content,
        handle_width: DEFAULT_RESIZE_HANDLE_HIT_WIDTH,
        handle_inset: DEFAULT_RESIZE_HANDLE_INSET,
        handle_key: None,
        handle_style: None,
        hover_chrome_only: false,
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        application::{IntoView, resizable, text},
        layout::{ContainerKind, LayoutNode},
    };

    #[test]
    fn resizable_wraps_content_and_resize_handle_in_a_row() {
        let layout = resizable(text("Sidebar"))
            .resize_handle(|_| ())
            .into_surface()
            .layout_node();

        let LayoutNode::Container(container) = layout else {
            panic!("resizable content should lower to a row container");
        };
        assert_eq!(container.policy.kind, ContainerKind::Row);
        assert_eq!(container.policy.spacing, 0.0);
        assert_eq!(container.children.len(), 2);
    }

    #[test]
    fn subtle_resize_handle_uses_standard_handle_configuration() {
        let layout = resizable(text("Sidebar"))
            .subtle_resize_handle("sidebar-handle", |_| ())
            .into_surface()
            .layout_node();

        let LayoutNode::Container(container) = layout else {
            panic!("resizable content should lower to a row container");
        };
        assert_eq!(container.policy.kind, ContainerKind::Row);
        assert_eq!(container.children.len(), 2);
    }
}
