use super::super::source::SourceMetadata;
use super::{SurfaceFloatingLayer, SurfaceOverlay, SurfaceScene};
use crate::{
    UiAffinity,
    gui::automation::{
        AutomationBounds, AutomationNodeId, AutomationNodeSemantics, AutomationNodeSnapshot,
        AutomationRole,
    },
    layout::{ContainerPolicy, LayoutCapabilities, NodeId, SlotParams},
    runtime::{
        DevtoolsLayoutDiagnostic, DevtoolsNodeKind, DevtoolsNodeSnapshot, DevtoolsWidgetSnapshot,
        surface::widget::{EventMapper, ScrollMessageMapper, SurfaceWidget},
    },
    widgets::WidgetStyle,
};
use std::{rc::Rc, time::Instant};

/// One slot-owned child attachment inside a surface container.
pub struct SurfaceChild<Message> {
    /// Parent-owned slot parameters.
    pub slot: SlotParams,
    /// Child node attached to the slot.
    pub child: SurfaceNode<Message>,
}

/// Runtime-internal named construction fields for a [`SurfaceChild`].
pub(in crate::runtime) struct SurfaceChildParts<Message> {
    /// Parent-owned slot parameters.
    pub slot: SlotParams,
    /// Child node attached to the slot.
    pub child: SurfaceNode<Message>,
}

impl<Message> SurfaceChild<Message> {
    /// Build a container-owned surface child from runtime-internal named parts.
    pub(in crate::runtime) fn from_parts(parts: SurfaceChildParts<Message>) -> Self {
        Self {
            slot: parts.slot,
            child: parts.child,
        }
    }

    /// Build a container-owned surface child.
    pub fn new(slot: SlotParams, child: SurfaceNode<Message>) -> Self {
        Self::from_parts(SurfaceChildParts { slot, child })
    }

    /// Build a child that fills the parent slot on both axes.
    pub fn fill(child: SurfaceNode<Message>) -> Self {
        Self::from_parts(SurfaceChildParts {
            slot: SlotParams::fill(),
            child,
        })
    }
}

/// A generic declarative container node built on top of public layout policy.
pub struct SurfaceContainer<Message> {
    pub(in crate::runtime::surface) _ui_affinity: UiAffinity,
    pub(in crate::runtime::surface) id: NodeId,
    pub(in crate::runtime::surface) policy: ContainerPolicy,
    pub(in crate::runtime::surface) style: Option<WidgetStyle>,
    pub(in crate::runtime::surface) hoverable: bool,
    pub(in crate::runtime::surface) layout_capabilities: Option<LayoutCapabilities<Message>>,
    pub(in crate::runtime::surface) split_pane_runtime:
        Option<crate::gui::layout_core::SplitPaneRuntimeMode>,
    pub(in crate::runtime::surface) split_pane_ratio_settled: Option<Rc<dyn Fn(f32) -> Message>>,
    pub(in crate::runtime::surface) virtual_layout:
        Option<super::super::VirtualLayoutRegistration<Message>>,
    pub(in crate::runtime::surface) scroll_message:
        Option<EventMapper<crate::runtime::ScrollUpdate, Option<Message>>>,
    pub(in crate::runtime::surface) children: Vec<SurfaceChild<Message>>,
    pub(in crate::runtime::surface) source: Option<Rc<SourceMetadata>>,
}

/// Runtime-internal named construction fields for a [`SurfaceContainer`].
pub(in crate::runtime) struct SurfaceContainerParts<Message> {
    /// Stable layout node id.
    pub id: NodeId,
    /// Container behavior policy.
    pub policy: ContainerPolicy,
    /// Optional UI-local layout capability descriptor.
    pub layout_capabilities: Option<LayoutCapabilities<Message>>,
    /// Ordered slot children.
    pub children: Vec<SurfaceChild<Message>>,
}

impl<Message> SurfaceContainer<Message> {
    /// Build a generic container node from runtime-internal named parts.
    pub(in crate::runtime) fn from_parts(parts: SurfaceContainerParts<Message>) -> Self {
        Self {
            _ui_affinity: UiAffinity::new(),
            id: parts.id,
            policy: parts.policy,
            style: None,
            hoverable: false,
            layout_capabilities: parts.layout_capabilities,
            split_pane_runtime: None,
            split_pane_ratio_settled: None,
            virtual_layout: None,
            scroll_message: None,
            children: parts.children,
            source: None,
        }
    }

    /// Build a generic container node with ordered slot children.
    pub fn new(id: NodeId, policy: ContainerPolicy, children: Vec<SurfaceChild<Message>>) -> Self {
        Self::from_parts(SurfaceContainerParts {
            id,
            policy,
            layout_capabilities: None,
            children,
        })
    }

    /// Return this container with explicit chrome styling.
    pub fn with_style(mut self, style: WidgetStyle) -> Self {
        self.style = Some(style);
        self
    }

    /// Return this container with hover chrome enabled.
    pub fn with_hoverable(mut self, hoverable: bool) -> Self {
        self.hoverable = hoverable;
        self
    }

    /// Return this container with an explicitly registered UI-local layout
    /// capability descriptor.
    ///
    /// This attaches registration, revision evidence, and normalized hit-region
    /// declaration/projection. Version-3 descriptors may receive typed pointer
    /// input through runtime-owned admission and capture; version 4 additionally
    /// supplies optional runtime-owned typed container state. Version 2 remains
    /// projection/query-only.
    pub fn with_layout_capabilities(mut self, capabilities: LayoutCapabilities<Message>) -> Self {
        self.layout_capabilities = Some(capabilities);
        self
    }

    pub(in crate::runtime) fn with_virtual_layout_registration(
        mut self,
        registration: super::super::VirtualLayoutRegistration<Message>,
    ) -> Self {
        self.virtual_layout = Some(registration);
        self
    }

    pub(crate) fn with_split_pane_ratio_settled(
        mut self,
        map: Option<Rc<dyn Fn(f32) -> Message>>,
    ) -> Self {
        self.split_pane_ratio_settled = map;
        self
    }

    /// Return this container with a scroll movement message mapper.
    pub fn with_scroll_message(
        mut self,
        message: std::sync::Arc<
            dyn Fn(crate::runtime::ScrollUpdate) -> Option<Message> + Send + Sync,
        >,
    ) -> Self
    where
        Message: 'static,
    {
        self.scroll_message = Some(EventMapper::from_arc(message));
        self
    }

    /// Return this container with a UI-local scroll movement message mapper.
    pub fn with_scroll_message_local(mut self, message: ScrollMessageMapper<Message>) -> Self {
        self.scroll_message = Some(EventMapper::from_rc(message));
        self
    }

    /// Return this container with a scroll mapper carrying optional exact
    /// equality evidence.
    pub fn with_scroll_message_mapped(
        mut self,
        message: crate::runtime::EventMapper<crate::runtime::ScrollUpdate, Option<Message>>,
    ) -> Self {
        self.scroll_message = Some(message);
        self
    }

    pub(in crate::runtime::surface) fn scroll_mapper_descriptor(
        &self,
    ) -> crate::runtime::surface::widget::MapperDescriptor {
        self.scroll_message
            .as_ref()
            .map(EventMapper::descriptor)
            .unwrap_or(crate::runtime::surface::widget::MapperDescriptor::Absent)
    }
}

/// One node in a generic declarative [`crate::runtime::UiSurface`].
///
/// Surface nodes are UI-affine, including nodes built directly through the
/// public constructors.
///
/// ```compile_fail
/// use radiant::{layout::ContainerPolicy, runtime::SurfaceNode};
///
/// let node = SurfaceNode::<()>::container(1, ContainerPolicy::default(), Vec::new());
/// std::thread::spawn(move || drop(node));
/// ```
pub enum SurfaceNode<Message> {
    /// A root scene with base content plus typed transient layers.
    Scene(SurfaceScene<Message>),
    /// A layout container that owns slot children.
    Container(SurfaceContainer<Message>),
    /// A widget leaf plus host-defined message mapping.
    Widget(SurfaceWidget<Message>),
    /// A non-interactive floating overlay painted above normal layout content.
    Overlay(SurfaceOverlay),
    /// A floating child tree that can paint out of normal layout flow.
    FloatingLayer(SurfaceFloatingLayer<Message>),
}

impl<Message> SurfaceNode<Message> {
    pub(crate) fn with_split_pane_runtime_mode(
        self,
        mode: Option<crate::gui::layout_core::SplitPaneRuntimeMode>,
    ) -> Self {
        match self {
            Self::Container(mut container) => {
                container.split_pane_runtime = mode;
                Self::Container(container)
            }
            node => node,
        }
    }

    pub(crate) fn with_split_pane_ratio_settled(
        self,
        map: Option<Rc<dyn Fn(f32) -> Message>>,
    ) -> Self {
        match self {
            Self::Container(mut container) => {
                container = container.with_split_pane_ratio_settled(map);
                Self::Container(container)
            }
            node => node,
        }
    }

    pub(in crate::runtime) fn with_virtual_layout_registration(
        self,
        registration: super::super::VirtualLayoutRegistration<Message>,
    ) -> Self {
        match self {
            Self::Container(container) => {
                Self::Container(container.with_virtual_layout_registration(registration))
            }
            node => node,
        }
    }

    /// Replace this node's root identity while retaining its descendants.
    pub(crate) fn with_id(mut self, id: NodeId) -> Self {
        match &mut self {
            Self::Scene(scene) => scene.id = id,
            Self::Container(container) => container.id = id,
            Self::Widget(widget) => widget.set_id_runtime(id),
            Self::Overlay(overlay) => overlay.id = id,
            Self::FloatingLayer(layer) => layer.container.id = id,
        }
        self
    }

    pub(crate) fn with_source_metadata(mut self, metadata: SourceMetadata) -> Self {
        let metadata = Rc::new(metadata);
        match &mut self {
            Self::Scene(scene) => scene.source = Some(Rc::clone(&metadata)),
            Self::Container(container) => container.source = Some(Rc::clone(&metadata)),
            Self::Widget(widget) => widget.source = Some(Rc::clone(&metadata)),
            Self::Overlay(overlay) => overlay.source = Some(Rc::clone(&metadata)),
            Self::FloatingLayer(layer) => layer.source = Some(Rc::clone(&metadata)),
        }
        self
    }

    pub(crate) fn source_metadata_handle(&self) -> Option<Rc<SourceMetadata>> {
        match self {
            Self::Scene(scene) => scene.source.clone(),
            Self::Container(container) => container.source.clone(),
            Self::Widget(widget) => widget.source.clone(),
            Self::Overlay(overlay) => overlay.source.clone(),
            Self::FloatingLayer(layer) => layer.source.clone(),
        }
    }

    pub(in crate::runtime) fn timed_repaint_deadline(&self) -> Option<Instant> {
        fn earlier(current: Option<Instant>, candidate: Option<Instant>) -> Option<Instant> {
            match (current, candidate) {
                (Some(current), Some(candidate)) => Some(current.min(candidate)),
                (current, candidate) => current.or(candidate),
            }
        }

        match self {
            Self::Scene(scene) => {
                scene
                    .layers
                    .iter()
                    .fold(scene.base.timed_repaint_deadline(), |deadline, layer| {
                        earlier(
                            earlier(
                                deadline,
                                layer
                                    .input
                                    .as_ref()
                                    .and_then(SurfaceNode::timed_repaint_deadline),
                            ),
                            layer.node.timed_repaint_deadline(),
                        )
                    })
            }
            Self::Container(container) => {
                container.children.iter().fold(None, |deadline, child| {
                    earlier(deadline, child.child.timed_repaint_deadline())
                })
            }
            Self::Widget(widget) => widget.widget().timed_repaint_deadline(),
            Self::Overlay(_) => None,
            Self::FloatingLayer(layer) => layer
                .container
                .children
                .iter()
                .fold(None, |deadline, child| {
                    earlier(deadline, child.child.timed_repaint_deadline())
                }),
        }
    }

    pub(in crate::runtime) fn advance_timed_repaints(&mut self, now: Instant) -> bool {
        match self {
            Self::Scene(scene) => {
                let mut changed = scene.base.advance_timed_repaints(now);
                for layer in &mut scene.layers {
                    if let Some(input) = &mut layer.input {
                        changed |= input.advance_timed_repaints(now);
                    }
                    changed |= layer.node.advance_timed_repaints(now);
                }
                changed
            }
            Self::Container(container) => {
                let mut changed = false;
                for child in &mut container.children {
                    changed |= child.child.advance_timed_repaints(now);
                }
                changed
            }
            Self::Widget(widget) => widget
                .widget_object_mut_runtime()
                .advance_timed_repaint(now),
            Self::Overlay(_) => false,
            Self::FloatingLayer(layer) => {
                let mut changed = false;
                for child in &mut layer.container.children {
                    changed |= child.child.advance_timed_repaints(now);
                }
                changed
            }
        }
    }

    /// Return the stable node id.
    pub fn id(&self) -> NodeId {
        match self {
            Self::Scene(scene) => scene.id,
            Self::Container(container) => container.id,
            Self::Widget(widget) => widget.id(),
            Self::Overlay(overlay) => overlay.id,
            Self::FloatingLayer(layer) => layer.container.id,
        }
    }

    pub(in crate::runtime) fn devtools_snapshot_node(
        &self,
        pointer_capture: Option<NodeId>,
        layout: &crate::layout::LayoutOutput,
    ) -> DevtoolsNodeSnapshot {
        let node_id = self.id();
        DevtoolsNodeSnapshot {
            node_id,
            kind: self.devtools_node_kind(),
            bounds: layout.rects.get(&node_id).copied(),
            widget: self.devtools_widget_snapshot(pointer_capture),
            layout_diagnostics: layout
                .diagnostics
                .iter()
                .filter(|diagnostic| diagnostic.node_id == node_id)
                .map(|diagnostic| DevtoolsLayoutDiagnostic {
                    code: diagnostic.code,
                    message: diagnostic.message.to_string(),
                })
                .collect(),
            children: self.devtools_children(pointer_capture, layout),
        }
    }

    pub(in crate::runtime) fn automation_snapshot_node(
        &self,
        layout: &crate::layout::LayoutOutput,
    ) -> AutomationNodeSnapshot {
        let node_id = self.id();
        let mut snapshot = AutomationNodeSnapshot::from_semantics(
            AutomationNodeId::new(node_id.to_string()),
            layout
                .rects
                .get(&node_id)
                .copied()
                .map(AutomationBounds::from_rect)
                .unwrap_or_else(AutomationBounds::zero),
            self.automation_semantics(),
        );
        if let Self::Widget(widget) = self
            && let Some(actions) = widget.widget().automation_available_actions()
        {
            snapshot.available_actions = actions;
        }
        snapshot.with_children(self.automation_children(layout))
    }

    fn automation_semantics(&self) -> AutomationNodeSemantics {
        match self {
            Self::Scene(_) => AutomationNodeSemantics::new(AutomationRole::Root),
            Self::Container(_) | Self::FloatingLayer(_) => {
                AutomationNodeSemantics::new(AutomationRole::Group)
            }
            Self::Overlay(_) => AutomationNodeSemantics::new(AutomationRole::Panel),
            Self::Widget(widget) => widget.widget().automation_semantics(),
        }
    }

    fn automation_children(
        &self,
        layout: &crate::layout::LayoutOutput,
    ) -> Vec<AutomationNodeSnapshot> {
        match self {
            Self::Scene(scene) => std::iter::once(scene.base.as_ref())
                .chain(scene.ordered_layers().flat_map(|layer| {
                    layer
                        .input
                        .as_ref()
                        .into_iter()
                        .chain(std::iter::once(&layer.node))
                }))
                .map(|child| child.automation_snapshot_node(layout))
                .collect(),
            Self::Container(container) => container
                .children
                .iter()
                .map(|child| child.child.automation_snapshot_node(layout))
                .collect(),
            Self::FloatingLayer(layer) => layer
                .container
                .children
                .iter()
                .map(|child| child.child.automation_snapshot_node(layout))
                .collect(),
            Self::Widget(_) | Self::Overlay(_) => Vec::new(),
        }
    }

    fn devtools_node_kind(&self) -> DevtoolsNodeKind {
        match self {
            Self::Scene(_) => DevtoolsNodeKind::Scene,
            Self::Container(_) => DevtoolsNodeKind::Container,
            Self::Widget(_) => DevtoolsNodeKind::Widget,
            Self::Overlay(_) => DevtoolsNodeKind::Overlay,
            Self::FloatingLayer(_) => DevtoolsNodeKind::FloatingLayer,
        }
    }

    fn devtools_widget_snapshot(
        &self,
        pointer_capture: Option<NodeId>,
    ) -> Option<DevtoolsWidgetSnapshot> {
        let Self::Widget(widget) = self else {
            return None;
        };
        let common = widget.widget().common();
        Some(DevtoolsWidgetSnapshot {
            focus: common.focus,
            focusable: widget.is_focusable(),
            keyboard_focusable: widget.is_keyboard_focusable(),
            receives_pointer_hit_testing: widget.receives_pointer_hit_testing(),
            accepts_wheel_input: widget.receives_wheel_input(),
            accepts_pointer_move: widget.accepts_pointer_move(),
            captured: pointer_capture == Some(widget.id()),
            state: common.state,
            semantics: widget.widget().automation_semantics(),
        })
    }

    fn devtools_children(
        &self,
        pointer_capture: Option<NodeId>,
        layout: &crate::layout::LayoutOutput,
    ) -> Vec<DevtoolsNodeSnapshot> {
        match self {
            Self::Scene(scene) => std::iter::once(scene.base.as_ref())
                .chain(scene.ordered_layers().flat_map(|layer| {
                    layer
                        .input
                        .as_ref()
                        .into_iter()
                        .chain(std::iter::once(&layer.node))
                }))
                .map(|child| child.devtools_snapshot_node(pointer_capture, layout))
                .collect(),
            Self::Container(container) => container
                .children
                .iter()
                .map(|child| child.child.devtools_snapshot_node(pointer_capture, layout))
                .collect(),
            Self::FloatingLayer(layer) => layer
                .container
                .children
                .iter()
                .map(|child| child.child.devtools_snapshot_node(pointer_capture, layout))
                .collect(),
            Self::Widget(_) | Self::Overlay(_) => Vec::new(),
        }
    }
}
