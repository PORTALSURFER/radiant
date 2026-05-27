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
    layout::{CrossAlign, Insets, MainAlign, NodeId},
    runtime::SurfaceNode,
    widgets::{TextAlign, TextWrap, WidgetSizing, WidgetStyle},
};

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
}

pub(in crate::application) enum ViewNodeKind<Message> {
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
