include!("view_node/slot.rs");

/// Application view node with generated identity and default sizing.
pub struct ViewNode<Message> {
    kind: ViewNodeKind<Message>,
    id: Option<NodeId>,
    key: Option<String>,
    sizing: Option<WidgetSizing>,
    slot: SlotBehavior,
    padding: Insets,
    align_main: Option<MainAlign>,
    align_cross: Option<CrossAlign>,
    style: Option<WidgetStyle>,
    hoverable: bool,
    input_only: bool,
    text_wrap: Option<TextWrap>,
}

enum ViewNodeKind<Message> {
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
    Scroll {
        child: Box<ViewNode<Message>>,
    },
    Stack {
        children: Vec<ViewNode<Message>>,
    },
    OverlayPanel {
        rect: crate::gui::types::Rect,
        label: Option<String>,
    },
}

impl<Message> ViewNode<Message> {
    /// Use an explicit stable id instead of the generated structural id.
    pub fn id(mut self, id: NodeId) -> Self {
        self.id = Some(id);
        self.key = None;
        self
    }

    /// Use a scoped stable key instead of a numeric id.
    ///
    /// Child keys are scoped by their keyed or explicitly identified parent, so repeated rows can
    /// use names such as `"done"` or `"delete"` without colliding with sibling rows.
    pub fn key(mut self, key: impl ToString) -> Self {
        self.key = Some(key.to_string());
        self
    }

    /// Use explicit widget sizing instead of the generated default.
    pub fn sizing(mut self, sizing: WidgetSizing) -> Self {
        self.sizing = Some(sizing);
        self
    }

    /// Use explicit fixed widget sizing instead of the generated default.
    pub fn size(self, width: f32, height: f32) -> Self {
        self.sizing(WidgetSizing::fixed(Vector2::new(width, height)))
    }

    /// Use explicit fixed widget sizing instead of the generated default.
    pub fn fixed(self, width: f32, height: f32) -> Self {
        self.size(width, height)
    }

    /// Fill remaining space on the parent main axis and stretch on the cross axis.
    pub fn fill(mut self) -> Self {
        self.slot.width = AxisSlotBehavior::Fill(1.0);
        self.slot.height = AxisSlotBehavior::Fill(1.0);
        self
    }

    /// Fill remaining horizontal space in the parent layout.
    pub fn fill_width(mut self) -> Self {
        self.slot.width = AxisSlotBehavior::Fill(1.0);
        self
    }

    /// Fill remaining vertical space in the parent layout.
    pub fn fill_height(mut self) -> Self {
        self.slot.height = AxisSlotBehavior::Fill(1.0);
        self
    }

    /// Fill remaining main-axis space with the provided weight.
    pub fn grow(mut self, weight: f32) -> Self {
        self.slot.width = AxisSlotBehavior::Fill(weight);
        self.slot.height = AxisSlotBehavior::Fill(weight);
        self
    }

    /// Use intrinsic parent slot sizing on both axes.
    pub fn intrinsic(mut self) -> Self {
        self.slot.width = AxisSlotBehavior::Intrinsic;
        self.slot.height = AxisSlotBehavior::Intrinsic;
        self
    }

    /// Use a fixed parent slot width.
    pub fn width(mut self, width: f32) -> Self {
        self.slot.width = AxisSlotBehavior::Fixed(width);
        self
    }

    /// Use a percentage of the parent width when this node is in a row.
    pub fn width_percent(mut self, ratio: f32) -> Self {
        self.slot.width = AxisSlotBehavior::Percent(ratio);
        self
    }

    /// Use a fixed parent slot height.
    pub fn height(mut self, height: f32) -> Self {
        self.slot.height = AxisSlotBehavior::Fixed(height);
        self
    }

    /// Use a percentage of the parent height when this node is in a column.
    pub fn height_percent(mut self, ratio: f32) -> Self {
        self.slot.height = AxisSlotBehavior::Percent(ratio);
        self
    }

    /// Set the minimum widget size while preserving any existing preferred size.
    pub fn min_size(mut self, width: f32, height: f32) -> Self {
        let min = Vector2::new(width, height);
        let preferred = self.sizing.map(|sizing| sizing.preferred).unwrap_or(min);
        let baseline = self.sizing.and_then(|sizing| sizing.baseline);
        self.sizing = Some(WidgetSizing::new(min, preferred).with_optional_baseline(baseline));
        self
    }

    /// Set the preferred widget size while preserving any existing minimum size.
    pub fn preferred_size(mut self, width: f32, height: f32) -> Self {
        let preferred = Vector2::new(width, height);
        let min = self.sizing.map(|sizing| sizing.min).unwrap_or(preferred);
        let baseline = self.sizing.and_then(|sizing| sizing.baseline);
        self.sizing = Some(WidgetSizing::new(min, preferred).with_optional_baseline(baseline));
        self
    }

    /// Set the widget text baseline.
    pub fn baseline(mut self, baseline: f32) -> Self {
        let sizing = self.sizing.unwrap_or_else(|| match &self.kind {
            ViewNodeKind::Widget(widget) => widget.default_sizing(),
            _ => WidgetSizing::fixed(Vector2::new(0.0, 0.0)),
        });
        self.sizing = Some(sizing.with_baseline(baseline));
        self
    }

    /// Apply equal content padding when this node is a container.
    pub fn padding(mut self, padding: f32) -> Self {
        self.padding = Insets::all(padding.max(0.0));
        self
    }

    /// Apply horizontal content padding when this node is a container.
    pub fn padding_x(mut self, padding: f32) -> Self {
        let padding = padding.max(0.0);
        self.padding.left = padding;
        self.padding.right = padding;
        self
    }

    /// Apply vertical content padding when this node is a container.
    pub fn padding_y(mut self, padding: f32) -> Self {
        let padding = padding.max(0.0);
        self.padding.top = padding;
        self.padding.bottom = padding;
        self
    }

    /// Align this container's children along the main axis.
    pub fn align_main(mut self, align: MainAlign) -> Self {
        self.align_main = Some(align);
        self
    }

    /// Align this container's children along the cross axis.
    pub fn align_cross(mut self, align: CrossAlign) -> Self {
        self.align_cross = Some(align);
        self
    }

    /// Apply an explicit widget style.
    pub fn style(mut self, style: WidgetStyle) -> Self {
        self.style = Some(style);
        self
    }

    /// Allow this styled container to show hover chrome.
    pub fn hoverable(mut self) -> Self {
        self.hoverable = true;
        self
    }

    /// Keep an interactive widget in hit testing without painting its own chrome.
    pub fn input_only(mut self) -> Self {
        self.input_only = true;
        self
    }

    /// Use the accent tone and strong prominence.
    pub fn primary(self) -> Self {
        self.style(primary_style())
    }

    /// Use the danger tone for destructive actions.
    pub fn danger(self) -> Self {
        self.style(danger_style())
    }

    /// Use a lower-prominence treatment.
    pub fn subtle(mut self) -> Self {
        let mut style = self.style.unwrap_or_default();
        style.prominence = WidgetProminence::Subtle;
        self.style = Some(style);
        self
    }

    /// Allow text to wrap by words inside its assigned rectangle.
    pub fn wrap(mut self) -> Self {
        self.text_wrap = Some(TextWrap::Word);
        self
    }

    /// Keep text on one line and clip overflow.
    pub fn truncate(mut self) -> Self {
        self.text_wrap = Some(TextWrap::None);
        self
    }

    /// Set row or column spacing when this node is a container.
    pub fn spacing(mut self, spacing: f32) -> Self {
        self.set_spacing(spacing);
        self
    }

    fn set_spacing(&mut self, spacing: f32) {
        match &mut self.kind {
            ViewNodeKind::Row {
                spacing: current, ..
            }
            | ViewNodeKind::Column {
                spacing: current, ..
            } => *current = spacing.max(0.0),
            ViewNodeKind::Scroll { child } => child.set_spacing(spacing),
            _ => {}
        }
    }

    fn collect_reserved_ids(&self, scope: u64, ids: &mut HashSet<NodeId>) {
        if let Some(id) = self.resolved_id(scope) {
            ids.insert(id);
        }
        let child_scope = self.child_scope(scope);
        match &self.kind {
            ViewNodeKind::Row { children, .. } | ViewNodeKind::Column { children, .. } => {
                for child in children {
                    child.collect_reserved_ids(child_scope, ids);
                }
            }
            ViewNodeKind::Stack { children } => {
                for child in children {
                    child.collect_reserved_ids(child_scope, ids);
                }
            }
            ViewNodeKind::Scroll { child } => child.collect_reserved_ids(child_scope, ids),
            ViewNodeKind::Runtime(node) => {
                ids.insert(node.id());
            }
            _ => {}
        }
    }

    fn resolved_id(&self, scope: u64) -> Option<NodeId> {
        self.id
            .or_else(|| self.key.as_ref().map(|key| scoped_key_id(scope, key)))
    }

    fn child_scope(&self, parent_scope: u64) -> u64 {
        self.resolved_id(parent_scope).unwrap_or(parent_scope)
    }
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
        }
    }
}

impl<Message> IntoView<Message> for ViewNode<Message>
where
    Message: 'static,
{
    fn into_node(self) -> SurfaceNode<Message> {
        let mut reserved = HashSet::new();
        self.collect_reserved_ids(ROOT_KEY_SCOPE, &mut reserved);
        let mut ids = IdGenerator::new(reserved);
        self.lower(&mut ids, ROOT_KEY_SCOPE)
    }
}

impl<Message> ViewNode<Message>
where
    Message: 'static,
{
    fn lower(self, ids: &mut IdGenerator, scope: u64) -> SurfaceNode<Message> {
        let id = self.resolved_id(scope).unwrap_or_else(|| ids.next());
        let child_scope = id;
        match self.kind {
            ViewNodeKind::Runtime(node) => node,
            ViewNodeKind::Widget(widget) => widget.into_surface_node(WidgetViewContext {
                id,
                sizing: self.sizing,
                style: self.style,
                input_only: self.input_only,
                text_wrap: self.text_wrap,
            }),
            ViewNodeKind::Row { spacing, children } => {
                let policy = ContainerPolicy {
                    kind: ContainerKind::Row,
                    spacing,
                    padding: self.padding,
                    align_main: self.align_main.unwrap_or(MainAlign::Start),
                    align_cross: self.align_cross.unwrap_or(CrossAlign::Stretch),
                    ..ContainerPolicy::default()
                };
                let children = children
                    .into_iter()
                    .map(|child| child.lower_child(ids, child_scope, true))
                    .collect();
                if let Some(style) = self.style {
                    SurfaceNode::styled_container(id, policy, style, children)
                        .with_container_hoverable(self.hoverable)
                } else {
                    SurfaceNode::container(id, policy, children)
                }
            }
            ViewNodeKind::Column { spacing, children } => {
                let policy = ContainerPolicy {
                    kind: ContainerKind::Column,
                    spacing,
                    padding: self.padding,
                    align_main: self.align_main.unwrap_or(MainAlign::Start),
                    align_cross: self.align_cross.unwrap_or(CrossAlign::Stretch),
                    ..ContainerPolicy::default()
                };
                let children = children
                    .into_iter()
                    .map(|child| child.lower_child(ids, child_scope, false))
                    .collect();
                if let Some(style) = self.style {
                    SurfaceNode::styled_container(id, policy, style, children)
                        .with_container_hoverable(self.hoverable)
                } else {
                    SurfaceNode::container(id, policy, children)
                }
            }
            ViewNodeKind::Scroll { child } => {
                let policy = ContainerPolicy {
                    kind: ContainerKind::ScrollView,
                    overflow: crate::layout::OverflowPolicy::Scroll,
                    padding: self.padding,
                    align_main: self.align_main.unwrap_or(MainAlign::Start),
                    align_cross: self.align_cross.unwrap_or(CrossAlign::Stretch),
                    ..ContainerPolicy::default()
                };
                let children = vec![SurfaceChild::fill(child.lower(ids, child_scope))];
                if let Some(style) = self.style {
                    SurfaceNode::styled_container(id, policy, style, children)
                        .with_container_hoverable(self.hoverable)
                } else {
                    SurfaceNode::container(id, policy, children)
                }
            }
            ViewNodeKind::Stack { children } => {
                let policy = ContainerPolicy {
                    kind: ContainerKind::Stack,
                    padding: self.padding,
                    align_main: self.align_main.unwrap_or(MainAlign::Start),
                    align_cross: self.align_cross.unwrap_or(CrossAlign::Stretch),
                    ..ContainerPolicy::default()
                };
                let children = children
                    .into_iter()
                    .map(|child| SurfaceChild::fill(child.lower(ids, child_scope)))
                    .collect();
                if let Some(style) = self.style {
                    SurfaceNode::styled_container(id, policy, style, children)
                        .with_container_hoverable(self.hoverable)
                } else {
                    SurfaceNode::container(id, policy, children)
                }
            }
            ViewNodeKind::OverlayPanel { rect, label } => {
                if let Some(label) = label {
                    SurfaceNode::overlay_panel(id, rect, label, self.style.unwrap_or_default())
                } else {
                    SurfaceNode::overlay_marker(id, rect, self.style.unwrap_or_default())
                }
            }
        }
    }

    fn lower_child(
        self,
        ids: &mut IdGenerator,
        scope: u64,
        parent_horizontal: bool,
    ) -> SurfaceChild<Message> {
        let slot = self.slot.to_slot_params(parent_horizontal);
        SurfaceChild::new(slot, self.lower(ids, scope))
    }
}

