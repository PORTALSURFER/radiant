use super::{SurfaceChild, SurfaceLayer, SurfaceNode};
use crate::runtime::{NativeFileDrop, NativeFileDropMessageMapper};
use std::sync::Arc;

impl<Message> SurfaceNode<Message> {
    pub(crate) fn with_native_file_drop_mapper(
        self,
        mapper: NativeFileDropMessageMapper<Message>,
    ) -> Self
    where
        Message: 'static,
    {
        match self {
            Self::Scene(mut scene) => {
                scene.base =
                    Box::new((*scene.base).with_native_file_drop_mapper(Arc::clone(&mapper)));
                scene.layers = scene
                    .layers
                    .into_iter()
                    .map(|layer| layer.with_native_file_drop_mapper(Arc::clone(&mapper)))
                    .collect();
                Self::Scene(scene)
            }
            Self::Container(mut container) => {
                container.children = container
                    .children
                    .into_iter()
                    .map(|child| SurfaceChild {
                        slot: child.slot,
                        child: child
                            .child
                            .with_native_file_drop_mapper(Arc::clone(&mapper)),
                    })
                    .collect();
                Self::Container(container)
            }
            Self::Widget(widget) => {
                Self::Widget(widget.with_native_file_drop(move |drop: NativeFileDrop| mapper(drop)))
            }
            Self::FloatingLayer(mut layer) => {
                layer.container.children = layer
                    .container
                    .children
                    .into_iter()
                    .map(|child| SurfaceChild {
                        slot: child.slot,
                        child: child
                            .child
                            .with_native_file_drop_mapper(Arc::clone(&mapper)),
                    })
                    .collect();
                Self::FloatingLayer(layer)
            }
            Self::Overlay(overlay) => Self::Overlay(overlay),
        }
    }

    pub(crate) fn accepting_native_file_drop(self) -> Self {
        match self {
            Self::Scene(mut scene) => {
                scene.base = Box::new((*scene.base).accepting_native_file_drop());
                scene.layers = scene
                    .layers
                    .into_iter()
                    .map(SurfaceLayer::accepting_native_file_drop)
                    .collect();
                Self::Scene(scene)
            }
            Self::Container(mut container) => {
                container.children = container
                    .children
                    .into_iter()
                    .map(|child| SurfaceChild {
                        slot: child.slot,
                        child: child.child.accepting_native_file_drop(),
                    })
                    .collect();
                Self::Container(container)
            }
            Self::Widget(widget) => Self::Widget(widget.accepting_native_file_drop()),
            Self::FloatingLayer(mut layer) => {
                layer.container.children = layer
                    .container
                    .children
                    .into_iter()
                    .map(|child| SurfaceChild {
                        slot: child.slot,
                        child: child.child.accepting_native_file_drop(),
                    })
                    .collect();
                Self::FloatingLayer(layer)
            }
            Self::Overlay(overlay) => Self::Overlay(overlay),
        }
    }
}

impl<Message> SurfaceLayer<Message> {
    fn with_native_file_drop_mapper(self, mapper: NativeFileDropMessageMapper<Message>) -> Self
    where
        Message: 'static,
    {
        Self {
            kind: self.kind,
            input: self
                .input
                .map(|input| input.with_native_file_drop_mapper(Arc::clone(&mapper))),
            node: self.node.with_native_file_drop_mapper(mapper),
        }
    }

    fn accepting_native_file_drop(self) -> Self {
        Self {
            kind: self.kind,
            input: self.input.map(SurfaceNode::accepting_native_file_drop),
            node: self.node.accepting_native_file_drop(),
        }
    }
}
