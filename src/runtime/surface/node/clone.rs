use super::{
    SurfaceChild, SurfaceChildParts, SurfaceContainer, SurfaceContainerParts, SurfaceFloatingLayer,
    SurfaceLayer, SurfaceNode, SurfaceScene,
};

impl<Message> Clone for SurfaceChildParts<Message> {
    fn clone(&self) -> Self {
        Self {
            slot: self.slot,
            child: self.child.clone(),
        }
    }
}

// These surface tree clones stay manual so recursive nodes and message mappers
// can be cloned without requiring host application message types to implement
// `Clone`.
impl<Message> Clone for SurfaceChild<Message> {
    fn clone(&self) -> Self {
        Self {
            slot: self.slot,
            child: self.child.clone(),
        }
    }
}

impl<Message> Clone for SurfaceContainerParts<Message> {
    fn clone(&self) -> Self {
        Self {
            id: self.id,
            policy: self.policy.clone(),
            children: self.children.clone(),
        }
    }
}

impl<Message> Clone for SurfaceContainer<Message> {
    fn clone(&self) -> Self {
        Self {
            id: self.id,
            policy: self.policy.clone(),
            style: self.style,
            hoverable: self.hoverable,
            scroll_message: self.scroll_message.clone(),
            children: self.children.clone(),
        }
    }
}

impl<Message> Clone for SurfaceLayer<Message> {
    fn clone(&self) -> Self {
        Self {
            kind: self.kind,
            input: self.input.clone(),
            node: self.node.clone(),
        }
    }
}

impl<Message> Clone for SurfaceScene<Message> {
    fn clone(&self) -> Self {
        Self {
            id: self.id,
            base: self.base.clone(),
            layers: self.layers.clone(),
        }
    }
}

impl<Message> Clone for SurfaceFloatingLayer<Message> {
    fn clone(&self) -> Self {
        Self {
            container: self.container.clone(),
            interactive: self.interactive,
        }
    }
}

// Keep this recursive clone implementation explicit: surface trees can be
// retained and replayed by the runtime even when the host message type is not
// itself cloneable.
impl<Message> Clone for SurfaceNode<Message> {
    fn clone(&self) -> Self {
        match self {
            Self::Scene(scene) => Self::Scene(scene.clone()),
            Self::Container(container) => Self::Container(container.clone()),
            Self::Widget(widget) => Self::Widget(widget.clone()),
            Self::Overlay(overlay) => Self::Overlay(overlay.clone()),
            Self::FloatingLayer(layer) => Self::FloatingLayer(layer.clone()),
        }
    }
}
