//! Local overlay-stack builder for bounded content.

use crate::application::{ViewNode, stack_layers};

/// Builder for base content with optional local overlays and input surfaces.
///
/// Use this inside a view when overlays share the base content bounds, such as
/// loading feedback, local drag/drop targets, or transparent input shields.
/// Root-level transient UI should use [`crate::application::scene`] instead.
pub struct OverlayStack<Message> {
    children: Vec<ViewNode<Message>>,
}

/// Build a local overlay stack around base content.
pub fn overlay_stack<Message>(base: ViewNode<Message>) -> OverlayStack<Message> {
    OverlayStack {
        children: vec![base],
    }
}

impl<Message: 'static> OverlayStack<Message> {
    /// Add a visual or interactive overlay above previous children.
    pub fn overlay(mut self, overlay: ViewNode<Message>) -> Self {
        self.children.push(overlay);
        self
    }

    /// Add an optional visual or interactive overlay above previous children.
    pub fn overlay_opt(self, overlay: Option<ViewNode<Message>>) -> Self {
        match overlay {
            Some(overlay) => self.overlay(overlay),
            None => self,
        }
    }

    /// Add an input surface above previous children.
    ///
    /// The caller controls the input surface behavior and whether it should be
    /// `input_only()`. This method only declares stack ordering.
    pub fn input(mut self, input: ViewNode<Message>) -> Self {
        self.children.push(input);
        self
    }

    /// Add an optional input surface above previous children.
    pub fn input_opt(self, input: Option<ViewNode<Message>>) -> Self {
        match input {
            Some(input) => self.input(input),
            None => self,
        }
    }

    /// Project the local overlay stack into a view.
    pub fn into_view(self) -> ViewNode<Message> {
        stack_layers(self.children)
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        application::{IntoView, button, overlay_stack, text},
        gui::types::Point,
        layout::{ContainerKind, LayoutNode, Vector2},
        runtime::SurfaceRuntime,
        widgets::{PointerButton, PointerModifiers, TextWidget, WidgetInput},
    };

    #[derive(Clone, Debug, PartialEq)]
    enum DemoMessage {
        Activate,
    }

    #[derive(Default)]
    struct DemoState {
        activated: bool,
    }

    #[test]
    fn overlay_stack_with_only_base_returns_base_without_stack_container() {
        let layout = overlay_stack(text::<()>("Base").id(101))
            .into_view()
            .into_surface()
            .layout_node();

        assert!(
            matches!(layout, LayoutNode::Widget(_)),
            "base-only overlay stack should return the base view unchanged"
        );
    }

    #[test]
    fn overlay_stack_omits_none_overlays_and_inputs() {
        let layout = overlay_stack(text::<()>("Base").id(102))
            .overlay_opt(None)
            .input_opt(None)
            .into_view()
            .into_surface()
            .layout_node();

        assert!(
            matches!(layout, LayoutNode::Widget(_)),
            "none overlays should not allocate a stack container"
        );
    }

    #[test]
    fn overlay_stack_preserves_declared_order() {
        let layout = overlay_stack(text::<()>("Base").id(103))
            .overlay(text("Overlay").id(104))
            .input(text("Input").id(105))
            .into_view()
            .into_surface()
            .layout_node();

        let LayoutNode::Container(container) = layout else {
            panic!("multiple overlay-stack children should lower to a stack container");
        };
        assert_eq!(container.policy.kind, ContainerKind::Stack);
        assert_eq!(container.children.len(), 3);

        let child_ids: Vec<_> = container
            .children
            .iter()
            .map(|child| match &child.child {
                LayoutNode::Widget(widget) => widget.id,
                _ => panic!("test children should be widget leaves"),
            })
            .collect();
        assert_eq!(child_ids, [103, 104, 105]);
    }

    #[test]
    fn overlay_stack_routes_input_above_overlays() {
        let bridge = crate::application::app(DemoState::default())
            .view(|state| {
                overlay_stack(
                    text(if state.activated { "activated" } else { "idle" })
                        .id(106)
                        .fill(),
                )
                .overlay(text("Overlay").fill())
                .input(button("").message(DemoMessage::Activate).fill())
                .into_view()
                .fill()
            })
            .update(|state, message| match message {
                DemoMessage::Activate => state.activated = true,
            })
            .into_bridge();
        let mut runtime = SurfaceRuntime::new(bridge, Vector2::new(120.0, 22.0));
        let position = Point::new(8.0, 8.0);

        runtime.dispatch_input_at(
            position,
            WidgetInput::PointerPress {
                position,
                button: PointerButton::Primary,
                modifiers: PointerModifiers::default(),
            },
        );
        runtime.dispatch_input_at(
            position,
            WidgetInput::PointerRelease {
                position,
                button: PointerButton::Primary,
                modifiers: PointerModifiers::default(),
            },
        );

        assert_eq!(
            runtime
                .surface()
                .find_widget(106)
                .and_then(|widget| widget.widget_object().as_any().downcast_ref::<TextWidget>())
                .map(|widget| widget.text.as_str()),
            Some("activated")
        );
    }
}
