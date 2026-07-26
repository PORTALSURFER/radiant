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
    full_height_rail: bool,
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

    /// Paint the resize boundary as one continuous passive rail.
    pub fn full_height_rail(mut self) -> Self {
        self.full_height_rail = true;
        self
    }

    /// Finish the resizable content with a mapped trailing resize handle.
    pub fn resize_handle(
        self,
        map: impl Fn(DragHandleMessage) -> Message + 'static,
    ) -> ViewNode<Message> {
        let mut handle_builder = drag_handle();
        if self.hover_chrome_only {
            handle_builder = handle_builder.hover_chrome_only();
        }
        if self.full_height_rail {
            handle_builder = handle_builder.full_height_rail();
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
        map: impl Fn(DragHandleMessage) -> Message + 'static,
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
        full_height_rail: false,
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        application::{IntoView, resizable, text},
        layout::{ContainerKind, LayoutNode},
        runtime::UiSurface,
        widgets::{DragHandleMessage, DragHandleWidget, WidgetOutput},
    };
    use std::{cell::RefCell, rc::Rc};

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

    #[test]
    fn resizable_builder_mapper_invokes_and_drops_ui_local_capture() {
        let calls = Rc::new(RefCell::new(0usize));
        let calls_for_mapper = Rc::clone(&calls);
        let surface = resizable(text("Sidebar"))
            .resize_handle(move |_| {
                *calls_for_mapper.borrow_mut() += 1;
            })
            .into_surface();
        let handle_id = find_drag_handle_id(&surface, &surface.layout_node())
            .expect("resizable layout should include a drag handle");

        assert_eq!(
            surface.dispatch_widget_output(
                handle_id,
                WidgetOutput::typed(DragHandleMessage::started(crate::gui::types::Point::new(
                    120.0, 10.0
                ),)),
            ),
            Some(())
        );
        assert_eq!(*calls.borrow(), 1);
        drop(surface);
        assert_eq!(Rc::strong_count(&calls), 1);
    }

    #[test]
    fn subtle_resizable_builder_mapper_invokes_and_drops_ui_local_capture() {
        let calls = Rc::new(RefCell::new(0usize));
        let calls_for_mapper = Rc::clone(&calls);
        let surface = resizable(text("Sidebar"))
            .subtle_resize_handle("sidebar-handle", move |_| {
                *calls_for_mapper.borrow_mut() += 1;
            })
            .into_surface();
        let handle_id = find_drag_handle_id(&surface, &surface.layout_node())
            .expect("subtle resizable layout should include a drag handle");

        assert_eq!(
            surface.dispatch_widget_output(
                handle_id,
                WidgetOutput::typed(DragHandleMessage::started(crate::gui::types::Point::new(
                    120.0, 10.0
                ),)),
            ),
            Some(())
        );
        assert_eq!(*calls.borrow(), 1);
        drop(surface);
        assert_eq!(Rc::strong_count(&calls), 1);
    }

    fn find_drag_handle_id(surface: &UiSurface<()>, node: &LayoutNode) -> Option<u64> {
        match node {
            LayoutNode::Widget(widget) => surface
                .find_widget(widget.id)
                .filter(|widget| widget.widget().as_any().is::<DragHandleWidget>())
                .map(|_| widget.id),
            LayoutNode::Container(container) => container
                .children
                .iter()
                .find_map(|child| find_drag_handle_id(surface, &child.child)),
        }
    }
}
