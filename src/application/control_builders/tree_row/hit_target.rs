use crate::{
    gui::{
        list::{DenseRowChromeParts, DenseRowLabelParts, DenseRowOutlineStyle, DenseRowPalette},
        types::{Rect, Rgba8},
    },
    layout::{LayoutOutput, Vector2},
    runtime::{PaintPrimitive, PaintText},
    theme::ThemeTokens,
    widgets::{
        EmbeddedInteractiveRowWidget, InteractiveRowActions, InteractiveRowVisualStateParts,
        InteractiveRowWidget, WidgetSizing,
    },
};

use super::{DEFAULT_TREE_ROW_HEIGHT, TreeRowDragDropState};

#[derive(Clone)]
pub(super) struct TreeRowHitTarget<Message> {
    row: InteractiveRowWidget,
    actions: InteractiveRowActions<Message>,
    label: PaintText,
    selected: bool,
    drag_drop: TreeRowDragDropState,
    palette: DenseRowPalette,
    drop_target_outline: DenseRowOutlineStyle,
    normal_label_color: Option<Rgba8>,
    highlighted_label_color: Rgba8,
}

pub(super) struct TreeRowHitTargetParts<Message> {
    pub(super) label: PaintText,
    pub(super) selected: bool,
    pub(super) drag_drop: TreeRowDragDropState,
    pub(super) palette: DenseRowPalette,
    pub(super) drop_target_outline: DenseRowOutlineStyle,
    pub(super) normal_label_color: Option<Rgba8>,
    pub(super) highlighted_label_color: Rgba8,
    pub(super) actions: InteractiveRowActions<Message>,
}

impl<Message> TreeRowHitTarget<Message> {
    pub(super) fn new(parts: TreeRowHitTargetParts<Message>) -> Self {
        let mut row = crate::application::interactive_row()
            .tracked_drag_source_with_motion(
                parts.drag_drop.drag_active,
                parts.drag_drop.drag_source,
            )
            .tracked_drop_candidate(
                parts.drag_drop.drag_active && !parts.drag_drop.drag_source,
                parts.drag_drop.drop_target,
                parts.drag_drop.drop_candidate,
                parts.drag_drop.drop_target_active,
            )
            .custom_paint_hit_target();
        if parts.drag_drop.clears_hover_on_sync() {
            row = row.clear_hover_on_sync();
        }
        let mut row = row.widget();
        row.common.sizing = WidgetSizing::fixed(Vector2::new(0.0, DEFAULT_TREE_ROW_HEIGHT));
        Self {
            row,
            actions: parts.actions,
            label: parts.label,
            selected: parts.selected,
            drag_drop: parts.drag_drop,
            palette: parts.palette,
            drop_target_outline: parts.drop_target_outline,
            normal_label_color: parts.normal_label_color,
            highlighted_label_color: parts.highlighted_label_color,
        }
    }

    fn visual_state_parts(&self) -> InteractiveRowVisualStateParts {
        InteractiveRowVisualStateParts {
            selected: self.selected,
            active_target: self.drag_drop.drop_target,
            candidate: self.drag_drop.drop_candidate,
        }
    }

    fn chrome_parts(&self) -> DenseRowChromeParts {
        self.row
            .dense_chrome_parts(self.visual_state_parts(), self.palette)
            .outline_if(self.drag_drop.drop_target, self.drop_target_outline)
    }

    fn label_color(&self, theme: &ThemeTokens) -> Rgba8 {
        if self
            .row
            .dense_visual_state(self.visual_state_parts())
            .emphasizes_label()
        {
            self.highlighted_label_color
        } else {
            self.normal_label_color.unwrap_or(theme.text_primary)
        }
    }
}

impl<Message> EmbeddedInteractiveRowWidget for TreeRowHitTarget<Message>
where
    Message: Clone + Send + Sync + 'static,
{
    type Message = Message;

    fn interactive_row(&self) -> &InteractiveRowWidget {
        &self.row
    }

    fn interactive_row_mut(&mut self) -> &mut InteractiveRowWidget {
        &mut self.row
    }

    fn interactive_row_actions(&self) -> Option<&InteractiveRowActions<Self::Message>> {
        Some(&self.actions)
    }

    fn append_interactive_row_paint(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        bounds: Rect,
        _layout: &LayoutOutput,
        theme: &ThemeTokens,
    ) {
        self.row.push_dense_labeled_chrome(
            primitives,
            bounds,
            self.chrome_parts(),
            DenseRowLabelParts::new(self.label.clone(), self.label_color(theme)),
        );
    }
}
