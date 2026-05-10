#[path = "view_node/identity.rs"]
mod identity;
#[path = "view_node/lowering.rs"]
mod lowering;
#[path = "view_node/modifiers.rs"]
mod modifiers;
#[path = "view_node/slot.rs"]
mod slot;

pub(in crate::application) use slot::SlotBehavior;

use crate::{
    application::WidgetView,
    layout::{CrossAlign, Insets, MainAlign, NodeId},
    runtime::SurfaceNode,
    widgets::{TextAlign, TextWrap, WidgetSizing, WidgetStyle},
};

/// Application view node with generated identity and default sizing.
pub struct ViewNode<Message> {
    pub(in crate::application) kind: ViewNodeKind<Message>,
    pub(in crate::application) id: Option<NodeId>,
    pub(in crate::application) key: Option<String>,
    pub(in crate::application) sizing: Option<WidgetSizing>,
    pub(in crate::application) slot: SlotBehavior,
    pub(in crate::application) padding: Insets,
    pub(in crate::application) align_main: Option<MainAlign>,
    pub(in crate::application) align_cross: Option<CrossAlign>,
    pub(in crate::application) style: Option<WidgetStyle>,
    pub(in crate::application) hoverable: bool,
    pub(in crate::application) input_only: bool,
    pub(in crate::application) text_wrap: Option<TextWrap>,
    pub(in crate::application) text_align: Option<TextAlign>,
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
}

impl<Message> From<SurfaceNode<Message>> for ViewNode<Message> {
    fn from(node: SurfaceNode<Message>) -> Self {
        Self {
            kind: ViewNodeKind::Runtime(node),
            id: None,
            key: None,
            sizing: None,
            slot: SlotBehavior::default(),
            padding: Insets::default(),
            align_main: None,
            align_cross: None,
            style: None,
            hoverable: false,
            input_only: false,
            text_wrap: None,
            text_align: None,
        }
    }
}
