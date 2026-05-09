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

include!("view_node/identity.rs");

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

include!("view_node/lowering.rs");

