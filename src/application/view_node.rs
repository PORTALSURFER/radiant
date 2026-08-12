#[path = "view_node/identity.rs"]
mod identity;
#[path = "view_node/lowering.rs"]
mod lowering;
#[path = "view_node/lowering_defaults.rs"]
mod lowering_defaults;
#[path = "view_node/modifiers.rs"]
mod modifiers;
#[path = "view_node/slot.rs"]
mod slot;
#[path = "view_node/virtual_layout.rs"]
mod virtual_layout;

pub(in crate::application) use identity::KeyedIdentity;
pub use identity::{ContinuityKey, preserve_state};
pub(crate) use identity::{
    DeclarativeIdentityOrigin, DeclarativeOverlaySource, DeclarativeSourceContext,
    SourceIdentitySeed,
};
pub(crate) use virtual_layout::{
    VirtualLayoutViewAdmissionError, VirtualLayoutViewBatch, lower_virtual_layout_batch,
    lower_virtual_layout_item, lower_virtual_layout_shell,
};

use slot::SlotBehavior;

use crate::{
    application::{Overlays, PointerTarget, TextContent, WidgetView},
    gui::{input::KeyPress, shortcuts::ShortcutResolution},
    layout::{
        ContainerPolicy, CrossAlign, FloatingLayerHorizontalOverflow,
        FloatingLayerVerticalOverflow, Insets, MainAlign, NodeId, Vector2,
    },
    runtime::{
        LayerKind, NativeFileDrop, NativeFileDropMessageMapper, ScrollMessageMapper, SurfaceNode,
    },
    widgets::{TextAlign, TextBackgroundRole, TextColorRole, TextWrap, WidgetSizing, WidgetStyle},
};
use std::{any::Any, rc::Rc};

/// A typed scene overlay layer.
pub struct Layer<Message> {
    pub(in crate::application) kind: LayerKind,
    pub(in crate::application) input_policy: LayerInputPolicy,
    pub(in crate::application) input: Option<ViewNode<Message>>,
    pub(in crate::application) view: ViewNode<Message>,
}

impl<Message> Layer<Message> {
    pub(in crate::application) fn new(kind: LayerKind, view: ViewNode<Message>) -> Self {
        Self {
            kind,
            input_policy: LayerInputPolicy::PassThrough,
            input: None,
            view,
        }
    }
}

pub(super) struct ExtractedLayerRoot<Message> {
    node: ViewNode<Message>,
    context: DeclarativeSourceContext,
    seed: SourceIdentitySeed,
}

pub(super) struct ExtractedLayer<Message> {
    kind: LayerKind,
    input: Option<ExtractedLayerRoot<Message>>,
    foreground: ExtractedLayerRoot<Message>,
}

/// Declarative input behavior for one transient scene layer.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum LayerInputPolicy {
    /// Do not add any synthesized input surface. Input outside foreground
    /// content continues to route to lower scene content.
    #[default]
    PassThrough,
    /// Add a full-scene transparent input surface that consumes pointer and
    /// wheel input without emitting host messages.
    BlockInput,
    /// Add a full-scene transparent input surface that emits a host message for
    /// outside pointer activation and blocks wheel input behind the layer.
    DismissOnOutsideClick,
}

/// Application view node with generated identity and default sizing.
pub struct ViewNode<Message> {
    kind: ViewNodeKind<Message>,
    id: Option<NodeId>,
    key: Option<identity::ContinuityKey>,
    keyed_identity: Option<identity::KeyedIdentity>,
    has_reserved_identity: bool,
    has_reserved_descendant_identity: bool,
    sizing: Option<WidgetSizing>,
    slot: SlotBehavior,
    padding: Option<Insets>,
    align_main: Option<MainAlign>,
    align_cross: Option<CrossAlign>,
    pub(in crate::application) style: Option<WidgetStyle>,
    hoverable: bool,
    input_only: bool,
    text_wrap: Option<TextWrap>,
    text_align: Option<TextAlign>,
    text_color: Option<TextColorRole>,
    text_background: Option<TextBackgroundRole>,
    text_inset: Option<Vector2>,
    tooltip: Option<String>,
    scroll_message: Option<ScrollMessageMapper<Message>>,
    accepts_native_file_drop: bool,
    native_file_drop: Option<NativeFileDropMessageMapper<Message>>,
    overlay_layers: Vec<Layer<Message>>,
}

#[allow(clippy::large_enum_variant)]
pub(in crate::application) enum ViewNodeKind<Message> {
    Scene {
        base: Box<ViewNode<Message>>,
        layers: Vec<Layer<Message>>,
        presentation: Option<Box<dyn Any>>,
        shortcuts: Option<Box<dyn Fn(KeyPress) -> ShortcutResolution<Message>>>,
    },
    Runtime(SurfaceNode<Message>),
    VirtualLayout(super::VirtualLayoutParts<Message>),
    Widget(Box<dyn WidgetView<Message>>),
    Container {
        policy: ContainerPolicy,
        children: Vec<ViewNode<Message>>,
    },
    Scroll {
        child: Box<ViewNode<Message>>,
    },
    VirtualScroll {
        child: Box<ViewNode<Message>>,
        overscan_px: f32,
    },
    OverlayPanel {
        rect: crate::gui::types::Rect,
        label: Option<TextContent>,
    },
    FloatingLayer {
        offset: crate::gui::types::Point,
        size: crate::layout::Vector2,
        child: Box<ViewNode<Message>>,
        interactive: bool,
        horizontal_overflow: FloatingLayerHorizontalOverflow,
        vertical_overflow: FloatingLayerVerticalOverflow,
    },
}

impl<Message> From<SurfaceNode<Message>> for ViewNode<Message> {
    fn from(node: SurfaceNode<Message>) -> Self {
        Self::new(ViewNodeKind::Runtime(node)).with_reserved_identity()
    }
}

impl<Message> ViewNode<Message> {
    pub(in crate::application) fn new(kind: ViewNodeKind<Message>) -> Self {
        Self {
            kind,
            id: None,
            key: None,
            keyed_identity: None,
            has_reserved_identity: false,
            has_reserved_descendant_identity: false,
            sizing: None,
            slot: SlotBehavior::default(),
            padding: None,
            align_main: None,
            align_cross: None,
            style: None,
            hoverable: false,
            input_only: false,
            text_wrap: None,
            text_align: None,
            text_color: None,
            text_background: None,
            text_inset: None,
            tooltip: None,
            scroll_message: None,
            accepts_native_file_drop: false,
            native_file_drop: None,
            overlay_layers: Vec::new(),
        }
    }

    pub(in crate::application) fn with_reserved_descendant_identity(
        mut self,
        has_reserved_descendant_identity: bool,
    ) -> Self {
        self.has_reserved_descendant_identity = has_reserved_descendant_identity;
        self
    }

    pub(in crate::application) fn has_reserved_identity_in_subtree(&self) -> bool {
        self.has_reserved_identity || self.has_reserved_descendant_identity
    }

    fn with_reserved_identity(mut self) -> Self {
        self.has_reserved_identity = true;
        self
    }

    /// Mark this view subtree as accepting native file-drop events.
    ///
    /// This declares interest only; use [`Self::on_native_file_drop`] to map the
    /// dropped file event into a host message.
    pub fn accepts_native_file_drop(mut self) -> Self {
        self.accepts_native_file_drop = true;
        self
    }

    /// Emit a host message when a native file hover, cancel, or drop targets this view subtree.
    pub fn on_native_file_drop(
        mut self,
        map: impl Fn(NativeFileDrop) -> Message + 'static,
    ) -> Self {
        self.accepts_native_file_drop = true;
        self.native_file_drop = Some(Rc::new(map));
        self
    }

    pub(in crate::application) fn overlay_layer(mut self, layer: Layer<Message>) -> Self {
        if layer.has_reserved_identity_in_subtree() {
            self.has_reserved_descendant_identity = true;
        }
        self.overlay_layers.push(layer);
        self
    }

    /// Declare a collection of view-local scene overlays owned by this view subtree.
    pub fn overlays(mut self, overlays: Overlays<Message>) -> Self {
        for layer in overlays.into_layers() {
            self = self.overlay_layer(layer);
        }
        self
    }

    /// Attach a transparent pointer target to this view's bounds.
    ///
    /// The target is routed above the owner content but remains bounded by the
    /// owner's layout, which is useful for declaring drop or cancellation
    /// behavior on semantic containers.
    pub fn pointer_target(self, target: PointerTarget<Message>) -> Self
    where
        Message: 'static,
    {
        let slot = self.slot;
        let mut stack = crate::application::overlay_stack(self)
            .input(target.into_input())
            .into_view();
        stack.slot = slot;
        stack
    }

    /// Attach an optional transparent pointer target to this view's bounds.
    pub fn pointer_target_opt(self, target: Option<PointerTarget<Message>>) -> Self
    where
        Message: 'static,
    {
        match target {
            Some(target) => self.pointer_target(target),
            None => self,
        }
    }

    /// Attach a lazily built transparent pointer target when `condition` is true.
    pub fn pointer_target_if(
        self,
        condition: bool,
        target: impl FnOnce() -> PointerTarget<Message>,
    ) -> Self
    where
        Message: 'static,
    {
        if condition {
            self.pointer_target(target())
        } else {
            self
        }
    }
}

impl<Message> Layer<Message> {
    pub(in crate::application) fn has_reserved_identity_in_subtree(&self) -> bool {
        self.view.has_reserved_identity_in_subtree()
            || self
                .input
                .as_ref()
                .is_some_and(ViewNode::has_reserved_identity_in_subtree)
    }
}

impl<Message> ViewNode<Message> {
    pub(super) fn drain_layer_list_in_declaration_order(
        layers: &mut Vec<Layer<Message>>,
        owner_scope: NodeId,
        context: &DeclarativeSourceContext,
        output: &mut Vec<ExtractedLayer<Message>>,
    ) {
        let declared_layers = std::mem::take(layers);
        for (index, layer) in declared_layers.into_iter().enumerate() {
            let layer_context = context.with_overlay(DeclarativeOverlaySource {
                identity_scope: crate::application::ids::structural_id(
                    owner_scope,
                    crate::application::ids::StructuralKind::Overlay,
                    crate::application::ids::StructuralRole::SceneLayer(index),
                ),
                layer_kind: layer.kind,
            });
            let input = layer.input.map(|input| {
                Self::extract_layer_root(
                    input,
                    owner_scope,
                    crate::application::ids::StructuralRole::SceneInput(index),
                    layer_context.clone(),
                    output,
                )
            });
            let foreground = Self::extract_layer_root(
                layer.view,
                owner_scope,
                crate::application::ids::StructuralRole::SceneLayer(index),
                layer_context,
                output,
            );
            output.push(ExtractedLayer {
                kind: layer.kind,
                input,
                foreground,
            });
        }
    }

    fn extract_layer_root(
        mut node: ViewNode<Message>,
        owner_scope: NodeId,
        role: crate::application::ids::StructuralRole,
        context: DeclarativeSourceContext,
        output: &mut Vec<ExtractedLayer<Message>>,
    ) -> ExtractedLayerRoot<Message> {
        let seed = node.source_identity_seed(owner_scope, role);
        node.drain_overlay_layers_in_declaration_order_with_seed(&context, output, seed);
        ExtractedLayerRoot {
            node,
            context,
            seed,
        }
    }

    pub(in crate::application) fn drain_overlay_layers_in_declaration_order(
        &mut self,
        parent_scope: NodeId,
        role: crate::application::ids::StructuralRole,
        context: &DeclarativeSourceContext,
        layers: &mut Vec<ExtractedLayer<Message>>,
    ) {
        let seed = self.source_identity_seed(parent_scope, role);
        self.drain_overlay_layers_in_declaration_order_with_seed(context, layers, seed);
    }

    fn drain_overlay_layers_in_declaration_order_with_seed(
        &mut self,
        context: &DeclarativeSourceContext,
        layers: &mut Vec<ExtractedLayer<Message>>,
        seed: SourceIdentitySeed,
    ) {
        let node_scope = seed.structural_scope;
        let node_context = context.with_node(seed);
        match &mut self.kind {
            ViewNodeKind::Scene {
                base,
                layers: scene_layers,
                ..
            } => {
                base.drain_overlay_layers_in_declaration_order(
                    node_scope,
                    crate::application::ids::StructuralRole::SceneBase,
                    &node_context,
                    layers,
                );
                Self::drain_layer_list_in_declaration_order(
                    scene_layers,
                    node_scope,
                    &node_context,
                    layers,
                );
            }
            ViewNodeKind::Container { children, .. } => {
                for (index, child) in children.iter_mut().enumerate() {
                    child.drain_overlay_layers_in_declaration_order(
                        node_scope,
                        crate::application::ids::StructuralRole::ContainerChild(index),
                        &node_context,
                        layers,
                    );
                }
            }
            ViewNodeKind::Scroll { child } => {
                child.drain_overlay_layers_in_declaration_order(
                    node_scope,
                    crate::application::ids::StructuralRole::ScrollChild,
                    &node_context,
                    layers,
                );
            }
            ViewNodeKind::VirtualScroll { child, .. } => {
                child.drain_overlay_layers_in_declaration_order(
                    node_scope,
                    crate::application::ids::StructuralRole::VirtualScrollChild,
                    &node_context,
                    layers,
                );
            }
            ViewNodeKind::FloatingLayer { child, .. } => {
                child.drain_overlay_layers_in_declaration_order(
                    node_scope,
                    crate::application::ids::StructuralRole::FloatingLayerChild,
                    &node_context,
                    layers,
                );
            }
            ViewNodeKind::Runtime(_)
            | ViewNodeKind::VirtualLayout(_)
            | ViewNodeKind::Widget(_)
            | ViewNodeKind::OverlayPanel { .. } => {}
        }
        Self::drain_layer_list_in_declaration_order(
            &mut self.overlay_layers,
            node_scope,
            &node_context,
            layers,
        );
    }
}
