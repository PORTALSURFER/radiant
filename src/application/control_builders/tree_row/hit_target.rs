use crate::{
    gui::{
        list::{
            DenseRowChromeParts, DenseRowLabelParts, DenseRowMarkerStyle, DenseRowOutlineStyle,
            DenseRowPalette,
        },
        svg::SvgIcon,
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

use super::{defaults::DEFAULT_TREE_ROW_HEIGHT, drag_drop::TreeRowDragDropState};

#[derive(Clone)]
pub(super) struct TreeRowHitTarget<Message> {
    row: InteractiveRowWidget,
    actions: InteractiveRowActions<Message>,
    label: PaintText,
    selected: bool,
    focused: bool,
    drag_drop: TreeRowDragDropState,
    palette: DenseRowPalette,
    drop_target_outline: DenseRowOutlineStyle,
    selected_hover_marker: Option<DenseRowMarkerStyle>,
    normal_label_color: Option<Rgba8>,
    highlighted_label_color: Rgba8,
    trailing_icon: Option<SvgIcon>,
}

pub(super) struct TreeRowHitTargetParts<Message> {
    pub(super) label: PaintText,
    pub(super) selected: bool,
    pub(super) focused: bool,
    pub(super) drag_drop: TreeRowDragDropState,
    pub(super) palette: DenseRowPalette,
    pub(super) drop_target_outline: DenseRowOutlineStyle,
    pub(super) selected_hover_marker: Option<DenseRowMarkerStyle>,
    pub(super) normal_label_color: Option<Rgba8>,
    pub(super) highlighted_label_color: Rgba8,
    pub(super) trailing_icon: Option<SvgIcon>,
    pub(super) actions: InteractiveRowActions<Message>,
}

const TREE_ROW_TRAILING_ICON_SIZE: f32 = 11.0;
const TREE_ROW_TRAILING_ICON_INSET: f32 = 5.0;

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
            .activation_modifiers()
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
            focused: parts.focused,
            drag_drop: parts.drag_drop,
            palette: parts.palette,
            drop_target_outline: parts.drop_target_outline,
            selected_hover_marker: parts.selected_hover_marker,
            normal_label_color: parts.normal_label_color,
            highlighted_label_color: parts.highlighted_label_color,
            trailing_icon: parts.trailing_icon,
        }
    }

    fn visual_state_parts(&self) -> InteractiveRowVisualStateParts {
        InteractiveRowVisualStateParts {
            selected: self.selected,
            active_target: self.drag_drop.drop_target,
            candidate: self.drag_drop.drop_candidate,
        }
    }

    fn visual_state(&self) -> crate::gui::list::DenseRowVisualState {
        let mut state = self.row.dense_visual_state(self.visual_state_parts());
        state.selected |= self.focused;
        state
    }

    fn chrome_parts(&self) -> DenseRowChromeParts {
        let state = self.visual_state();
        let mut chrome = DenseRowChromeParts::new(state, self.palette)
            .outline_if(self.drag_drop.drop_target, self.drop_target_outline);
        if state.selected
            && state.hovered
            && let Some(marker) = self.selected_hover_marker
        {
            chrome = chrome.leading_marker(marker);
        }
        chrome
    }

    fn label_color(&self, theme: &ThemeTokens) -> Rgba8 {
        if self.label_visual_state().emphasizes_label() {
            self.highlighted_label_color
        } else {
            self.normal_label_color.unwrap_or(theme.text_primary)
        }
    }

    fn label_visual_state(&self) -> crate::gui::list::DenseRowVisualState {
        let mut state = self.row.dense_visual_state(self.visual_state_parts());
        if state.hovered && self.focused {
            state.selected = true;
        } else if !state.hovered {
            state.selected = false;
        }
        state
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
        if let Some(icon) = &self.trailing_icon {
            icon.append_paint(primitives, self.row.id(), trailing_icon_rect(bounds));
        }
    }
}

fn trailing_icon_rect(bounds: Rect) -> Rect {
    let size = TREE_ROW_TRAILING_ICON_SIZE.min(bounds.height()).max(0.0);
    let x = (bounds.max.x - TREE_ROW_TRAILING_ICON_INSET - size).max(bounds.min.x);
    let y = bounds.min.y + ((bounds.height() - size) * 0.5).max(0.0);
    Rect::from_xy_size(x, y, size, size)
}
