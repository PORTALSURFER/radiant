use std::sync::Arc;

use crate::{
    application::{TextContent, ViewNode},
    gui::{
        list::{DenseRowMarkerStyle, DenseRowOutlineStyle, DenseRowPalette, TreeGuideMetrics},
        svg::SvgIcon,
        types::Rgba8,
    },
    runtime::PaintText,
    widgets::{
        InteractiveRowActions, WidgetId, WidgetStyle, stable_widget_id, stable_widget_id_u64,
    },
};

use super::{
    defaults::{
        DEFAULT_HIGHLIGHTED_LABEL_COLOR, DEFAULT_TREE_EXPANDER_WIDTH, DEFAULT_TREE_ROW_HEIGHT,
        default_guide_metrics,
    },
    drag_drop::TreeRowDragDropState,
};

/// Builder for a compact generic tree row.
pub struct TreeRowBuilder {
    pub(super) label: PaintText,
    pub(super) depth: usize,
    pub(super) expanded: bool,
    pub(super) has_children: bool,
    pub(super) selected: bool,
    pub(super) focused: bool,
    pub(super) drag_drop: TreeRowDragDropState,
    pub(super) row_height: f32,
    pub(super) expander_width: f32,
    pub(super) guide_metrics: TreeGuideMetrics,
    pub(super) style: Option<WidgetStyle>,
    pub(super) input_id: Option<WidgetId>,
    pub(super) row_key: Option<String>,
    pub(super) hit_key: Option<String>,
    pub(super) palette: Option<DenseRowPalette>,
    pub(super) drop_target_outline: Option<DenseRowOutlineStyle>,
    pub(super) selected_marker: Option<DenseRowMarkerStyle>,
    pub(super) focus_marker: Option<DenseRowMarkerStyle>,
    pub(super) selected_trailing_marker: Option<DenseRowMarkerStyle>,
    pub(super) hover_trailing_marker: Option<DenseRowMarkerStyle>,
    pub(super) focus_outline: Option<DenseRowOutlineStyle>,
    pub(super) selected_hover_marker: Option<DenseRowMarkerStyle>,
    pub(super) normal_label_color: Option<Rgba8>,
    pub(super) highlighted_label_color: Rgba8,
    pub(super) label_inset_x: f32,
    pub(super) trailing_icon: Option<SvgIcon>,
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

    /// Set whether the host considers this row keyboard-focused.
    pub fn focused(mut self, focused: bool) -> Self {
        self.focused = focused;
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
    pub fn guide_style(mut self, style: impl Into<TreeGuideMetrics>) -> Self {
        self.guide_metrics = style.into();
        self
    }

    /// Resolve standard dense-row chrome from this semantic widget style.
    pub fn style(mut self, style: WidgetStyle) -> Self {
        self.style = Some(style);
        self
    }

    /// Set a stable widget id for the interactive hit target.
    pub fn input_id(mut self, id: WidgetId) -> Self {
        self.input_id = Some(id);
        self.hit_key = None;
        self
    }

    /// Derive a stable hit-target id from a text key.
    pub fn stable_input_id(mut self, scope: u64, key: impl AsRef<str>) -> Self {
        self.input_id = Some(stable_widget_id(scope, key));
        self.hit_key = None;
        self
    }

    /// Derive a stable hit-target id from a numeric key.
    pub fn stable_u64_input_id(mut self, scope: u64, key: u64) -> Self {
        self.input_id = Some(stable_widget_id_u64(scope, key));
        self.hit_key = None;
        self
    }

    /// Set a stable key for the composed row and its default child keys.
    pub fn row_key(mut self, key: impl Into<String>) -> Self {
        self.row_key = Some(key.into());
        self
    }

    /// Assign one stable row key to the composed row and its hit target.
    ///
    /// Use this for dynamic tree rows whose outer view subtree and retained
    /// hit-target widget should follow the same caller-owned row identity.
    /// Prefer [`Self::row_key`] plus [`Self::input_id`] or [`Self::hit_key`]
    /// only when those identities deliberately differ.
    pub fn stable_row_identity(mut self, input_scope: u64, row_key: impl Into<String>) -> Self {
        let row_key = row_key.into();
        self.input_id = Some(stable_widget_id(input_scope, row_key.as_str()));
        self.row_key = Some(row_key);
        self.hit_key = None;
        self
    }

    /// Override the stable key used for the interactive hit target.
    pub fn hit_key(mut self, key: impl Into<String>) -> Self {
        self.hit_key = Some(key.into());
        self.input_id = None;
        self
    }

    /// Override dense-row fill colors.
    pub fn palette(mut self, palette: DenseRowPalette) -> Self {
        self.palette = Some(palette);
        self
    }

    /// Override the outline used for committed drop targets.
    pub fn drop_target_outline(mut self, outline: DenseRowOutlineStyle) -> Self {
        self.drop_target_outline = Some(outline);
        self
    }

    /// Set a leading marker painted whenever the row is selected.
    pub fn selected_marker(mut self, marker: DenseRowMarkerStyle) -> Self {
        self.selected_marker = Some(marker);
        self
    }

    /// Set a leading marker painted for keyboard focus and pointer-down focus.
    pub fn focus_marker(mut self, marker: DenseRowMarkerStyle) -> Self {
        self.focus_marker = Some(marker);
        self
    }

    /// Set a trailing marker painted whenever the row is selected.
    pub fn selected_trailing_marker(mut self, marker: DenseRowMarkerStyle) -> Self {
        self.selected_trailing_marker = Some(marker);
        self
    }

    /// Set a trailing marker painted while the row is hovered and no selected
    /// trailing marker is active.
    pub fn hover_trailing_marker(mut self, marker: DenseRowMarkerStyle) -> Self {
        self.hover_trailing_marker = Some(marker);
        self
    }

    /// Set an outline painted whenever the host marks the row keyboard-focused.
    pub fn focus_outline(mut self, outline: DenseRowOutlineStyle) -> Self {
        self.focus_outline = Some(outline);
        self
    }

    /// Set a leading marker painted only when the row is both selected and hovered.
    pub fn selected_hover_marker(mut self, marker: DenseRowMarkerStyle) -> Self {
        self.selected_hover_marker = Some(marker);
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

    /// Set the horizontal gap between leading row chrome and the label.
    pub fn label_inset_x(mut self, inset: f32) -> Self {
        self.label_inset_x = inset.max(0.0);
        self
    }

    /// Paint a small passive icon at the trailing edge of the hit target.
    pub fn trailing_icon(mut self, icon: SvgIcon) -> Self {
        self.trailing_icon = Some(icon);
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
}

/// Builder returned after a tree row receives a toggle action.
pub struct TreeRowMessageBuilder<Message> {
    pub(super) row: TreeRowBuilder,
    pub(super) toggle: Option<Arc<dyn Fn() -> Message + Send + Sync + 'static>>,
}

/// Build a generic compact tree row.
pub fn tree_row(label: impl Into<TextContent>) -> TreeRowBuilder {
    TreeRowBuilder {
        label: label.into().into_paint_text(),
        depth: 0,
        expanded: false,
        has_children: false,
        selected: false,
        focused: false,
        drag_drop: TreeRowDragDropState::default(),
        row_height: DEFAULT_TREE_ROW_HEIGHT,
        expander_width: DEFAULT_TREE_EXPANDER_WIDTH,
        guide_metrics: default_guide_metrics(),
        style: None,
        input_id: None,
        row_key: None,
        hit_key: None,
        palette: None,
        drop_target_outline: None,
        selected_marker: None,
        focus_marker: None,
        selected_trailing_marker: None,
        hover_trailing_marker: None,
        focus_outline: None,
        selected_hover_marker: None,
        normal_label_color: None,
        highlighted_label_color: DEFAULT_HIGHLIGHTED_LABEL_COLOR,
        label_inset_x: 4.0,
        trailing_icon: None,
    }
}

impl<Message: Clone + Send + Sync + 'static> TreeRowMessageBuilder<Message> {
    /// Attach interactive row actions and build the row.
    pub fn interactive_actions(self, actions: InteractiveRowActions<Message>) -> ViewNode<Message> {
        self.row.build(self.toggle, actions)
    }
}
