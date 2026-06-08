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

use slot::SlotBehavior;

use crate::{
    application::WidgetView,
    layout::{CrossAlign, Insets, MainAlign, NodeId, Vector2},
    runtime::{LayerKind, ScrollMessageMapper, SurfaceNode},
    widgets::{TextAlign, TextBackgroundRole, TextColorRole, TextWrap, WidgetSizing, WidgetStyle},
};
use std::any::Any;

/// A typed transient scene layer.
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
    key: Option<String>,
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
    scroll_message: Option<ScrollMessageMapper<Message>>,
}

pub(in crate::application) enum ViewNodeKind<Message> {
    Scene {
        base: Box<ViewNode<Message>>,
        layers: Vec<Layer<Message>>,
        presentation: Option<Box<dyn Any>>,
    },
    Runtime(SurfaceNode<Message>),
    Widget(Box<dyn WidgetView<Message>>),
    Row {
        spacing: f32,
        children: Vec<ViewNode<Message>>,
    },
    Column {
        spacing: f32,
        children: Vec<ViewNode<Message>>,
    },
    Grid {
        columns: usize,
        column_gap: f32,
        row_gap: f32,
        children: Vec<ViewNode<Message>>,
    },
    Wrap {
        item_gap: f32,
        line_gap: f32,
        children: Vec<ViewNode<Message>>,
    },
    Scroll {
        child: Box<ViewNode<Message>>,
    },
    VirtualScroll {
        child: Box<ViewNode<Message>>,
        overscan_px: f32,
    },
    Stack {
        children: Vec<ViewNode<Message>>,
    },
    OverlayPanel {
        rect: crate::gui::types::Rect,
        label: Option<String>,
    },
    FloatingLayer {
        offset: crate::gui::types::Point,
        size: crate::layout::Vector2,
        child: Box<ViewNode<Message>>,
        interactive: bool,
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
            scroll_message: None,
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
}
