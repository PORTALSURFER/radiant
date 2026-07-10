use super::{SurfaceNode, UiSurface};
use crate::layout::LayoutNode;

impl<Message> Clone for UiSurface<Message> {
    fn clone(&self) -> Self {
        Self {
            root: self.root.clone(),
        }
    }
}

impl<Message> UiSurface<Message> {
    /// Build a top-level UI surface from one declarative root node.
    pub fn new(root: SurfaceNode<Message>) -> Self {
        Self { root }
    }

    /// Return the root declarative node.
    pub fn root(&self) -> &SurfaceNode<Message> {
        &self.root
    }

    /// Consume the surface and return its root declarative node.
    pub fn into_root(self) -> SurfaceNode<Message> {
        self.root
    }

    /// Project the surface into the public layout tree consumed by layout engines.
    pub fn layout_node(&self) -> LayoutNode {
        self.root.layout_node()
    }

    /// Count widget output mappings backed by allocated dynamic callbacks.
    ///
    /// This diagnostic excludes native file-drop callbacks and constant-message
    /// bindings represented inline by the surface.
    pub fn widget_callback_allocation_count(&self) -> usize {
        widget_callback_allocation_count(&self.root)
    }
}

fn widget_callback_allocation_count<Message>(node: &SurfaceNode<Message>) -> usize {
    match node {
        SurfaceNode::Scene(scene) => {
            widget_callback_allocation_count(&scene.base)
                + scene
                    .layers
                    .iter()
                    .map(|layer| {
                        layer
                            .input
                            .as_ref()
                            .map(widget_callback_allocation_count)
                            .unwrap_or(0)
                            + widget_callback_allocation_count(&layer.node)
                    })
                    .sum::<usize>()
        }
        SurfaceNode::Container(container) => container
            .children
            .iter()
            .map(|child| widget_callback_allocation_count(&child.child))
            .sum(),
        SurfaceNode::Widget(widget) => usize::from(widget.uses_dynamic_output_callback()),
        SurfaceNode::Overlay(_) => 0,
        SurfaceNode::FloatingLayer(layer) => layer
            .container
            .children
            .iter()
            .map(|child| widget_callback_allocation_count(&child.child))
            .sum(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        layout::Vector2,
        widgets::{ButtonMessage, WidgetOutput, WidgetSizing},
    };
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };

    #[derive(Debug)]
    struct CountedMessage {
        clone_count: Arc<AtomicUsize>,
    }

    impl Clone for CountedMessage {
        fn clone(&self) -> Self {
            self.clone_count.fetch_add(1, Ordering::Relaxed);
            Self {
                clone_count: Arc::clone(&self.clone_count),
            }
        }
    }

    #[test]
    fn callback_allocation_count_distinguishes_constant_and_dynamic_button_mappers() {
        let sizing = WidgetSizing::fixed(Vector2::new(80.0, 24.0));
        let constant = UiSurface::new(SurfaceNode::button(1, "Constant", sizing, ()));
        let dynamic = UiSurface::new(SurfaceNode::button_mapped(2, "Dynamic", sizing, |_| ()));

        assert_eq!(constant.widget_callback_allocation_count(), 0);
        assert_eq!(dynamic.widget_callback_allocation_count(), 1);
    }

    #[test]
    fn cloning_surface_shares_constant_message_without_cloning_it() {
        let clone_count = Arc::new(AtomicUsize::new(0));
        let surface = UiSurface::new(SurfaceNode::button(
            1,
            "Constant",
            WidgetSizing::fixed(Vector2::new(80.0, 24.0)),
            CountedMessage {
                clone_count: Arc::clone(&clone_count),
            },
        ));

        let cloned = surface.clone();
        assert_eq!(clone_count.load(Ordering::Relaxed), 0);

        assert!(
            surface
                .dispatch_widget_output(1, WidgetOutput::typed(ButtonMessage::Activate))
                .is_some()
        );
        assert!(
            cloned
                .dispatch_widget_output(1, WidgetOutput::typed(ButtonMessage::Activate))
                .is_some()
        );
        assert_eq!(clone_count.load(Ordering::Relaxed), 2);
    }
}
