use crate::{
    gui::automation::{AutomationLiveRegion, AutomationNodeSemantics, AutomationRole},
    gui::{
        list::{
            DenseRowChromeParts, DenseRowLabelParts, DenseRowMarkerStyle, DenseRowOutlineStyle,
            DenseRowPalette, dense_row_drop_outline_from_style, dense_row_palette_from_style,
        },
        svg::SvgIcon,
        types::{Rect, Rgba8},
    },
    layout::{LayoutOutput, Vector2},
    runtime::{PaintPrimitive, PaintText, ResolvedEnvironment},
    theme::ThemeTokens,
    widgets::{
        DeclaredTextMetrics, EmbeddedInteractiveRowWidget, FocusBehavior, InteractiveRowMessage,
        InteractiveRowVisualStateParts, InteractiveRowWidget, WidgetSemantics, WidgetSizing,
        WidgetStyle,
    },
};

use super::{
    defaults::{default_drop_target_outline, default_palette},
    drag_drop::TreeRowDragDropState,
};

#[derive(Clone)]
pub(super) struct TreeRowHitTarget {
    row: InteractiveRowWidget,
    declared_text_metrics: DeclaredTextMetrics,
    label: PaintText,
    selected: bool,
    focused: bool,
    drag_drop: TreeRowDragDropState,
    style: Option<WidgetStyle>,
    palette: Option<DenseRowPalette>,
    drop_target_outline: Option<DenseRowOutlineStyle>,
    selected_marker: Option<DenseRowMarkerStyle>,
    focus_marker: Option<DenseRowMarkerStyle>,
    pressed_focus_marker: Option<DenseRowMarkerStyle>,
    selected_trailing_marker: Option<DenseRowMarkerStyle>,
    hover_trailing_marker: Option<DenseRowMarkerStyle>,
    focus_outline: Option<DenseRowOutlineStyle>,
    selected_hover_marker: Option<DenseRowMarkerStyle>,
    normal_label_color: Option<Rgba8>,
    highlighted_label_color: Rgba8,
    trailing_icon: Option<SvgIcon>,
}

pub(super) struct TreeRowHitTargetParts {
    pub(super) label: PaintText,
    pub(super) row_height: f32,
    pub(super) selected: bool,
    pub(super) focused: bool,
    pub(super) drag_drop: TreeRowDragDropState,
    pub(super) style: Option<WidgetStyle>,
    pub(super) palette: Option<DenseRowPalette>,
    pub(super) drop_target_outline: Option<DenseRowOutlineStyle>,
    pub(super) selected_marker: Option<DenseRowMarkerStyle>,
    pub(super) focus_marker: Option<DenseRowMarkerStyle>,
    pub(super) pressed_focus_marker: Option<DenseRowMarkerStyle>,
    pub(super) selected_trailing_marker: Option<DenseRowMarkerStyle>,
    pub(super) hover_trailing_marker: Option<DenseRowMarkerStyle>,
    pub(super) focus_outline: Option<DenseRowOutlineStyle>,
    pub(super) selected_hover_marker: Option<DenseRowMarkerStyle>,
    pub(super) normal_label_color: Option<Rgba8>,
    pub(super) highlighted_label_color: Rgba8,
    pub(super) label_inset_x: f32,
    pub(super) trailing_icon: Option<SvgIcon>,
}

const TREE_ROW_TRAILING_ICON_SIZE: f32 = 11.0;
const TREE_ROW_TRAILING_ICON_INSET: f32 = 5.0;

impl TreeRowHitTarget {
    pub(super) fn new(parts: TreeRowHitTargetParts) -> Self {
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
        row.common.sizing = WidgetSizing::fixed(Vector2::new(0.0, parts.row_height));
        let declared_text_metrics = DeclaredTextMetrics::new(
            WidgetSizing::fixed(Vector2::new(0.0, parts.row_height)),
            crate::gui::list::dense_row_label_font_size(parts.row_height),
            Vector2::new(parts.label_inset_x.max(0.0), 0.0),
        );
        if let Some(style) = parts.style {
            row.common.style = style;
        }
        Self {
            row,
            declared_text_metrics,
            label: parts.label,
            selected: parts.selected,
            focused: parts.focused,
            drag_drop: parts.drag_drop,
            style: parts.style,
            palette: parts.palette,
            drop_target_outline: parts.drop_target_outline,
            selected_marker: parts.selected_marker,
            focus_marker: parts.focus_marker,
            pressed_focus_marker: parts.pressed_focus_marker,
            selected_trailing_marker: parts.selected_trailing_marker,
            hover_trailing_marker: parts.hover_trailing_marker,
            focus_outline: parts.focus_outline,
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
        self.row.dense_visual_state(self.visual_state_parts())
    }

    fn palette(&self, theme: &ThemeTokens) -> DenseRowPalette {
        self.palette.unwrap_or_else(|| {
            self.style
                .map(|style| dense_row_palette_from_style(theme, style))
                .unwrap_or_else(default_palette)
        })
    }

    fn drop_target_outline(&self, theme: &ThemeTokens) -> DenseRowOutlineStyle {
        self.drop_target_outline.unwrap_or_else(|| {
            self.style
                .map(|style| dense_row_drop_outline_from_style(theme, style))
                .unwrap_or_else(default_drop_target_outline)
        })
    }

    fn chrome_parts(&self, theme: &ThemeTokens) -> DenseRowChromeParts {
        let state = self.visual_state();
        let mut chrome = DenseRowChromeParts::new(state, self.palette(theme))
            .outline_if(self.drag_drop.drop_target, self.drop_target_outline(theme));
        if self.selected
            && let Some(marker) = self.selected_marker
        {
            chrome = chrome.leading_marker(marker);
        }
        let focus_marker = if state.pressed {
            self.pressed_focus_marker.or(self.focus_marker)
        } else if self.focused {
            self.focus_marker
        } else {
            None
        };
        if let Some(marker) = focus_marker {
            chrome = chrome.leading_overlay_marker(marker);
        }
        if self.selected
            && let Some(marker) = self.selected_trailing_marker
        {
            chrome = chrome.trailing_marker(marker);
        } else if state.hovered
            && let Some(marker) = self.hover_trailing_marker
        {
            chrome = chrome.trailing_marker(marker);
        }
        if state.selected
            && state.hovered
            && let Some(marker) = self.selected_hover_marker
        {
            chrome = chrome.leading_marker(marker);
        }
        if self.focused
            && let Some(outline) = self.focus_outline
        {
            chrome = chrome.outline(outline);
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

impl EmbeddedInteractiveRowWidget for TreeRowHitTarget {
    type Message = InteractiveRowMessage;

    fn interactive_row(&self) -> &InteractiveRowWidget {
        &self.row
    }

    fn interactive_row_mut(&mut self) -> &mut InteractiveRowWidget {
        &mut self.row
    }

    fn map_interactive_row_message(&self, message: InteractiveRowMessage) -> Option<Self::Message> {
        Some(message)
    }

    fn declared_interactive_row_text_metrics(&self) -> Option<DeclaredTextMetrics> {
        Some(self.declared_text_metrics)
    }

    fn interactive_row_semantics(&self) -> Option<&dyn WidgetSemantics> {
        Some(self)
    }

    fn append_interactive_row_paint(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        bounds: Rect,
        _layout: &LayoutOutput,
        theme: &ThemeTokens,
    ) {
        self.append_paint_for_environment(
            primitives,
            bounds,
            theme,
            &ResolvedEnvironment::default(),
        );
    }

    fn append_interactive_row_paint_with_context(
        &self,
        context: &mut crate::widgets::WidgetPaintContext<'_>,
    ) {
        let bounds = context.bounds();
        let theme = context.theme();
        let environment = context.environment();
        let primitives = context.primitives();
        self.append_paint_for_environment(primitives, bounds, theme, environment);
    }
}

impl WidgetSemantics for TreeRowHitTarget {
    fn automation_role(&self) -> crate::gui::automation::AutomationRole {
        AutomationRole::Row
    }

    fn automation_label(&self) -> Option<String> {
        Some(self.label.as_str().to_owned())
    }

    fn resolve_automation_semantics(
        &self,
        common: &crate::widgets::WidgetCommon,
    ) -> AutomationNodeSemantics {
        let focusable = common.focus != FocusBehavior::None && !common.state.disabled;
        AutomationNodeSemantics {
            role: AutomationRole::Row,
            label: self.automation_label(),
            description: None,
            value_text: None,
            checked: None,
            selected: self.selected,
            disabled: common.state.disabled,
            read_only: common.state.read_only,
            focusable,
            focused: common.state.focused,
            tab_index: (common.focus == FocusBehavior::Keyboard && !common.state.disabled)
                .then_some(0),
            focus_hints: Default::default(),
            live_region: AutomationLiveRegion::None,
            metadata: Default::default(),
        }
    }
}

impl TreeRowHitTarget {
    fn append_paint_for_environment(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        bounds: Rect,
        theme: &ThemeTokens,
        environment: &ResolvedEnvironment,
    ) {
        self.row.push_dense_labeled_chrome_with_environment(
            primitives,
            bounds,
            self.chrome_parts(theme),
            DenseRowLabelParts::new(self.label.clone(), self.label_color(theme)),
            self.declared_text_metrics,
            environment,
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
