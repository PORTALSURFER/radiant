use super::ViewLowering;
use crate::{
    application::{Layer, ViewNode, ids::StructuralRole},
    gui::{input::KeyPress, shortcuts::ShortcutResolution},
    layout::NodeId,
    runtime::{SurfaceLayer, SurfaceNode},
};
use std::any::Any;

impl<Message: 'static> ViewLowering<'_, '_, Message> {
    // Scene-only temporary trees must not inflate every recursive container
    // lowering frame, including the supported 256-component path boundary.
    #[inline(never)]
    #[expect(
        clippy::boxed_local,
        reason = "Keep the existing scene base box out of recursive caller stack frames"
    )]
    pub(super) fn lower_scene(
        &mut self,
        id: NodeId,
        child_scope: u64,
        base: Box<ViewNode<Message>>,
        mut layers: Vec<Layer<Message>>,
        presentation: Option<Box<dyn Any>>,
        shortcuts: Option<Box<dyn Fn(KeyPress) -> ShortcutResolution<Message>>>,
    ) -> SurfaceNode<Message> {
        if (presentation.is_some() || shortcuts.is_some() || !layers.is_empty())
            && let Some(context) = self.application_context.as_deref_mut()
        {
            context.mark_unsupported();
        }
        self.scene.capture(presentation, shortcuts);
        let mut base = *base;
        let mut collected_layers = Vec::new();
        base.drain_overlay_layers_in_declaration_order(
            child_scope,
            StructuralRole::SceneBase,
            &self.source_context,
            &mut collected_layers,
        );
        ViewNode::drain_layer_list_in_declaration_order(
            &mut layers,
            child_scope,
            &self.source_context,
            &mut collected_layers,
        );
        let base = self.lower_node(base, child_scope, StructuralRole::SceneBase);
        let mut escape_dismissals = Vec::with_capacity(collected_layers.len());
        let layers = collected_layers
            .into_iter()
            .enumerate()
            .map(|(index, layer)| {
                escape_dismissals.push(layer.escape_dismissal);
                let input = layer.input.map(|input| {
                    self.lower_extracted_layer_root(
                        input,
                        child_scope,
                        StructuralRole::SceneInput(index),
                    )
                });
                let foreground = self.lower_extracted_layer_root(
                    layer.foreground,
                    child_scope,
                    StructuralRole::SceneLayer(index),
                );
                SurfaceLayer::with_input(layer.kind, input, foreground)
            })
            .collect();
        SurfaceNode::scene(id, base, layers).with_overlay_escape_dismissals(escape_dismissals)
    }
}
