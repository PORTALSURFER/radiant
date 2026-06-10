use std::sync::Arc;

use crate::{
    application::{MappedWidget, ViewNode, disclosure_button, row, spacer},
    gui::{
        list::{DenseRowOutlineStyle, DenseRowPalette, TreeGuideStyle, tree_guide_indent},
        types::Rgba8,
    },
    runtime::{PaintText, WidgetMessageMapper},
    widgets::{InteractiveRowActions, WidgetId, stable_widget_id, stable_widget_id_u64},
};

use hit_target::{TreeRowHitTarget, TreeRowHitTargetParts};

mod hit_target;
#[cfg(test)]
mod tests;

const DEFAULT_TREE_ROW_HEIGHT: f32 = 22.0;
const DEFAULT_TREE_EXPANDER_WIDTH: f32 = 28.0;
const DEFAULT_TREE_DEPTH_INDENT: f32 = 12.0;
const DEFAULT_TREE_GUIDE_COLOR: Rgba8 = Rgba8 {
    r: 116,
    g: 130,
    b: 148,
    a: 128,
};
const DEFAULT_SELECTED_FILL: Rgba8 = Rgba8 {
    r: 95,
    g: 130,
    b: 170,
    a: 96,
};
const DEFAULT_INTERACTION_FILL: Rgba8 = Rgba8 {
    r: 110,
    g: 138,
    b: 170,
    a: 78,
};
const DEFAULT_ACTIVE_TARGET_FILL: Rgba8 = Rgba8 {
    r: 100,
    g: 150,
    b: 190,
    a: 190,
};
const DEFAULT_CANDIDATE_HOVER_FILL: Rgba8 = Rgba8 {
    r: 110,
    g: 150,
    b: 190,
    a: 130,
};
const DEFAULT_OUTLINE_COLOR: Rgba8 = Rgba8 {
    r: 150,
    g: 185,
    b: 220,
    a: 220,
};
const DEFAULT_HIGHLIGHTED_LABEL_COLOR: Rgba8 = Rgba8 {
    r: 245,
    g: 248,
    b: 252,
    a: 255,
};

/// Host-owned drag/drop state for a generic tree row.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct TreeRowDragDropState {
    /// A related row or external item is currently being dragged.
    pub drag_active: bool,
    /// This row is the source of the active drag.
    pub drag_source: bool,
    /// This row is a valid drop candidate.
    pub drop_candidate: bool,
    /// This row is the committed drop target.
    pub drop_target: bool,
    /// The surrounding tree currently has a committed drop target.
    pub drop_target_active: bool,
}

impl TreeRowDragDropState {
    /// Build an inactive drag/drop state.
    pub const fn new() -> Self {
        Self {
            drag_active: false,
            drag_source: false,
            drop_candidate: false,
            drop_target: false,
            drop_target_active: false,
        }
    }

    /// Return whether the row should clear stale hover when synchronized.
    pub const fn clears_hover_on_sync(self) -> bool {
        (self.drag_active || self.drop_target_active) && !self.drop_target
    }
}

/// Builder for a compact generic tree row.
pub struct TreeRowBuilder {
    label: PaintText,
    depth: usize,
    expanded: bool,
    has_children: bool,
    selected: bool,
    drag_drop: TreeRowDragDropState,
    row_height: f32,
    expander_width: f32,
    guide_style: TreeGuideStyle,
    input_id: Option<WidgetId>,
    row_key: Option<String>,
    hit_key: Option<String>,
    palette: DenseRowPalette,
    drop_target_outline: DenseRowOutlineStyle,
    normal_label_color: Option<Rgba8>,
    highlighted_label_color: Rgba8,
}

impl TreeRowBuilder {
    /// Set the visual tree depth.
    pub fn depth(mut self, depth: usize) -> Self {
        self.depth = depth;
        self
    }

    /// Set whether the row's branch is expanded.
    pub fn expanded(mut self, expanded: bool) -> Self {
        self.expanded = expanded;
        self
    }

    /// Set whether the row has expandable children.
    pub fn has_children(mut self, has_children: bool) -> Self {
        self.has_children = has_children;
        self
    }

    /// Set whether the row is selected.
    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    /// Set host-owned drag/drop state for the row.
    pub fn drag_drop_state(mut self, state: TreeRowDragDropState) -> Self {
        self.drag_drop = state;
        self
    }

    /// Set the fixed row height used by the expander and hit target.
    pub fn row_height(mut self, height: f32) -> Self {
        self.row_height = height.max(0.0);
        self
    }

    /// Set the fixed width of the disclosure/expander slot.
    pub fn expander_width(mut self, width: f32) -> Self {
        self.expander_width = width.max(0.0);
        self
    }

    /// Set the tree-guide style used for the leading indent.
    pub fn guide_style(mut self, style: TreeGuideStyle) -> Self {
        self.guide_style = style;
        self
    }

    /// Set a stable widget id for the interactive hit target.
    pub fn input_id(mut self, id: WidgetId) -> Self {
        self.input_id = Some(id);
        self
    }

    /// Derive a stable hit-target id from a text key.
    pub fn stable_input_id(mut self, scope: u64, key: impl AsRef<str>) -> Self {
        self.input_id = Some(stable_widget_id(scope, key));
        self
    }

    /// Derive a stable hit-target id from a numeric key.
    pub fn stable_u64_input_id(mut self, scope: u64, key: u64) -> Self {
        self.input_id = Some(stable_widget_id_u64(scope, key));
        self
    }

    /// Set a stable key for the composed row and its default child keys.
    pub fn row_key(mut self, key: impl Into<String>) -> Self {
        self.row_key = Some(key.into());
        self
    }

    /// Override the stable key used for the interactive hit target.
    pub fn hit_key(mut self, key: impl Into<String>) -> Self {
        self.hit_key = Some(key.into());
        self
    }

    /// Override dense-row fill colors.
    pub fn palette(mut self, palette: DenseRowPalette) -> Self {
        self.palette = palette;
        self
    }

    /// Override the outline used for committed drop targets.
    pub fn drop_target_outline(mut self, outline: DenseRowOutlineStyle) -> Self {
        self.drop_target_outline = outline;
        self
    }

    /// Override the normal label color.
    pub fn label_color(mut self, color: Rgba8) -> Self {
        self.normal_label_color = Some(color);
        self
    }

    /// Override the highlighted label color.
    pub fn highlighted_label_color(mut self, color: Rgba8) -> Self {
        self.highlighted_label_color = color;
        self
    }

    /// Attach a toggle action for the disclosure/expander slot.
    pub fn on_toggle<Message>(
        self,
        message: impl Fn() -> Message + Send + Sync + 'static,
    ) -> TreeRowMessageBuilder<Message> {
        TreeRowMessageBuilder {
            row: self,
            toggle: Some(Arc::new(message)),
        }
    }

    /// Attach interactive row actions and build the row.
    pub fn interactive_actions<Message: Clone + Send + Sync + 'static>(
        self,
        actions: InteractiveRowActions<Message>,
    ) -> ViewNode<Message> {
        self.build(None, actions)
    }

    fn build<Message: Clone + Send + Sync + 'static>(
        self,
        toggle: Option<Arc<dyn Fn() -> Message + Send + Sync + 'static>>,
        actions: InteractiveRowActions<Message>,
    ) -> ViewNode<Message> {
        let row_height = self.row_height;
        let depth = self.depth;
        let guide_style = self.guide_style;
        let row_key = self.row_key.clone();
        let hit_key = self
            .hit_key
            .clone()
            .or_else(|| row_key.as_ref().map(|key| format!("{key}-hit")));
        let expander = self.expander(toggle);
        let mut hit_target = self.hit_target(actions).fill_width().height(row_height);
        if let Some(hit_key) = hit_key {
            hit_target = hit_target.key(hit_key);
        }

        let mut row = row([tree_guide_indent(depth, guide_style), expander, hit_target])
            .spacing(1.0)
            .fill_width()
            .height(row_height);
        if let Some(row_key) = row_key {
            row = row.key(row_key);
        }
        row
    }

    fn expander<Message: Clone + Send + Sync + 'static>(
        &self,
        toggle: Option<Arc<dyn Fn() -> Message + Send + Sync + 'static>>,
    ) -> ViewNode<Message> {
        let key = self.row_key.as_ref().map(|key| format!("{key}-expander"));
        let mut expander = if self.has_children {
            if let Some(toggle) = toggle {
                disclosure_button(self.expanded)
                    .mapped(move |_| toggle())
                    .subtle()
                    .size(self.expander_width, self.row_height)
            } else {
                disclosure_button(self.expanded)
                    .enabled(false)
                    .passive()
                    .subtle()
                    .size(self.expander_width, self.row_height)
            }
        } else {
            spacer().size(self.expander_width, self.row_height)
        };
        if let Some(key) = key {
            expander = expander.key(key);
        }
        expander
    }

    fn hit_target<Message: Clone + Send + Sync + 'static>(
        self,
        actions: InteractiveRowActions<Message>,
    ) -> ViewNode<Message> {
        let input_id = self.input_id;
        let widget = TreeRowHitTarget::new(TreeRowHitTargetParts {
            label: self.label,
            selected: self.selected,
            drag_drop: self.drag_drop,
            palette: self.palette,
            drop_target_outline: self.drop_target_outline,
            normal_label_color: self.normal_label_color,
            highlighted_label_color: self.highlighted_label_color,
            actions,
        });
        let mut view = crate::application::view_node_from_widget(MappedWidget::new(
            widget,
            WidgetMessageMapper::typed(|message: Message| message),
        ));
        if let Some(input_id) = input_id {
            view = view.id(input_id);
        }
        view
    }
}

/// Builder returned after a tree row receives a toggle action.
pub struct TreeRowMessageBuilder<Message> {
    row: TreeRowBuilder,
    toggle: Option<Arc<dyn Fn() -> Message + Send + Sync + 'static>>,
}

impl<Message: Clone + Send + Sync + 'static> TreeRowMessageBuilder<Message> {
    /// Attach interactive row actions and build the row.
    pub fn interactive_actions(self, actions: InteractiveRowActions<Message>) -> ViewNode<Message> {
        self.row.build(self.toggle, actions)
    }
}

/// Build a generic compact tree row.
pub fn tree_row(label: impl Into<PaintText>) -> TreeRowBuilder {
    TreeRowBuilder {
        label: label.into(),
        depth: 0,
        expanded: false,
        has_children: false,
        selected: false,
        drag_drop: TreeRowDragDropState::default(),
        row_height: DEFAULT_TREE_ROW_HEIGHT,
        expander_width: DEFAULT_TREE_EXPANDER_WIDTH,
        guide_style: TreeGuideStyle::new(
            DEFAULT_TREE_DEPTH_INDENT,
            DEFAULT_TREE_ROW_HEIGHT,
            DEFAULT_TREE_GUIDE_COLOR,
        ),
        input_id: None,
        row_key: None,
        hit_key: None,
        palette: DenseRowPalette::new()
            .selected(DEFAULT_SELECTED_FILL)
            .interaction_fills(DEFAULT_INTERACTION_FILL, DEFAULT_INTERACTION_FILL)
            .active_target(DEFAULT_ACTIVE_TARGET_FILL)
            .candidate_hovered(DEFAULT_CANDIDATE_HOVER_FILL),
        drop_target_outline: DenseRowOutlineStyle::new(0.5, DEFAULT_OUTLINE_COLOR, 1.5),
        normal_label_color: None,
        highlighted_label_color: DEFAULT_HIGHLIGHTED_LABEL_COLOR,
    }
}
