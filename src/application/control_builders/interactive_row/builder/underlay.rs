use crate::{
    application::{MappedWidget, ViewNode, input_underlay, view_node_from_widget},
    gui::{
        list::{
            DenseRowChromeParts, DenseRowMarkerStyle, DenseRowOutlineStyle, DenseRowPalette,
            dense_row_palette_from_style,
        },
        types::Rect,
    },
    layout::LayoutOutput,
    runtime::{PaintPrimitive, WidgetMessageMapper},
    theme::ThemeTokens,
    widgets::{
        InteractiveRowActions, InteractiveRowMessage, InteractiveRowVisualStateParts,
        InteractiveRowWidget, Widget, WidgetId, WidgetInput, WidgetOutput, WidgetStyle,
        stable_widget_id, stable_widget_id_u64,
    },
};

use super::{
    InteractiveRowBuilder,
    defaults::interactive_row,
    policy::{DenseRowDragPolicy, DenseRowDropPolicy, DenseRowPolicy},
};

/// Builder for arbitrary row content backed by a generic interactive row.
pub struct InteractiveRowUnderlayBuilder<Message> {
    content: ViewNode<Message>,
    row: InteractiveRowBuilder,
    input_id: Option<WidgetId>,
    input_key: Option<String>,
    row_key: Option<String>,
    style: Option<WidgetStyle>,
    visual_state: InteractiveRowVisualStateParts,
    chrome: DenseInteractiveRowUnderlayChrome,
    dense_chrome: bool,
}

impl<Message: 'static> InteractiveRowUnderlayBuilder<Message> {
    /// Apply a reusable dense-row behavior policy.
    ///
    /// Policies bundle common input, visual-state, drag, and drop behavior for
    /// app-owned row content. Low-level row, chrome, identity, and message
    /// methods remain available before or after this call for custom cases.
    pub fn dense_row_policy(mut self, policy: DenseRowPolicy) -> Self {
        if policy.custom_paint_hit_target {
            self.row = self.row.custom_paint_hit_target();
        }
        if policy.activation_modifiers {
            self.row = self.row.activation_modifiers();
        }
        match policy.drag {
            DenseRowDragPolicy::None => {}
            DenseRowDragPolicy::Source {
                drag_active,
                drag_source,
                source_motion,
            } => {
                self.row = if source_motion {
                    self.row
                        .tracked_drag_source_with_motion(drag_active, drag_source)
                } else {
                    self.row.tracked_drag_source(drag_active, drag_source)
                };
            }
        }
        match policy.drop {
            DenseRowDropPolicy::None => {}
            DenseRowDropPolicy::Target {
                drag_active,
                active_target,
            } => {
                self.row = self.row.tracked_drop_target(drag_active, active_target);
            }
            DenseRowDropPolicy::Candidate {
                drag_active,
                current_target,
                candidate,
                active_target,
            } => {
                self.row = self.row.tracked_drop_candidate(
                    drag_active,
                    current_target,
                    candidate,
                    active_target,
                );
            }
        }
        if policy.drag_session_motion {
            self.row = self.row.drag_active(true).pointer_motion_active(true);
        }
        if let Some(style) = policy.style {
            self.style = Some(style);
        }
        if let Some(selected) = policy.visual_state_overrides.selected {
            self.visual_state.selected = selected;
        }
        if let Some(active_target) = policy.visual_state_overrides.active_target {
            self.visual_state.active_target = active_target;
        }
        if let Some(candidate) = policy.visual_state_overrides.candidate {
            self.visual_state.candidate = candidate;
        }
        self.dense_chrome = self.dense_chrome || policy.dense_chrome;
        self
    }

    /// Configure the backing interactive row before binding messages.
    pub fn row(
        mut self,
        configure: impl FnOnce(InteractiveRowBuilder) -> InteractiveRowBuilder,
    ) -> Self {
        self.row = configure(self.row);
        self
    }

    /// Configure the backing row as an input-only layer for app-owned row paint.
    ///
    /// Use this when the visible content or dense underlay chrome owns all row
    /// feedback, while Radiant should still route generic row input behavior.
    pub fn custom_paint_hit_target(mut self) -> Self {
        self.row = self.row.custom_paint_hit_target();
        self
    }

    /// Include primary-release modifier state in row activation messages.
    pub fn activation_modifiers(mut self) -> Self {
        self.row = self.row.activation_modifiers();
        self
    }

    /// Configure the backing row as a host-tracked drag source.
    ///
    /// Use this when arbitrary visible row content should keep its own paint
    /// tree while the underlay owns generic drag lifecycle routing.
    pub fn tracked_drag_source(mut self, drag_active: bool, drag_source: bool) -> Self {
        self.row = self.row.tracked_drag_source(drag_active, drag_source);
        self
    }

    /// Configure the backing row as a host-tracked drag source that keeps
    /// emitting pointer movement after the active source is rebuilt.
    pub fn tracked_drag_source_with_motion(mut self, drag_active: bool, drag_source: bool) -> Self {
        self.row = self
            .row
            .tracked_drag_source_with_motion(drag_active, drag_source);
        self
    }

    /// Configure the backing row as a host-tracked drop target.
    ///
    /// Use this when arbitrary visible row content should keep its own paint
    /// tree while the underlay owns generic drop and hover-drop routing.
    pub fn tracked_drop_target(mut self, drag_active: bool, active_target: bool) -> Self {
        self.row = self.row.tracked_drop_target(drag_active, active_target);
        self.visual_state.active_target = active_target;
        self
    }

    /// Configure the backing row as a host-tracked conditional drop target.
    ///
    /// Use this when arbitrary visible row content should keep its own paint
    /// tree while Radiant owns the generic candidate hover and stale-target
    /// clear lifecycle for host-validated drops.
    pub fn tracked_drop_candidate(
        mut self,
        drag_active: bool,
        current_target: bool,
        candidate: bool,
        active_target: bool,
    ) -> Self {
        self.row =
            self.row
                .tracked_drop_candidate(drag_active, current_target, candidate, active_target);
        self.visual_state.active_target = current_target;
        self.visual_state.candidate = candidate;
        self
    }

    /// Paint Radiant's standard dense-row chrome behind the visible content.
    ///
    /// Use this for list, tree, sidebar, picker, and inspector rows whose
    /// content is app-owned but whose hover, pressed, selected, and drop-target
    /// feedback should follow Radiant's generic dense-row policy.
    pub fn dense_chrome(mut self) -> Self {
        self.dense_chrome = true;
        self
    }

    /// Mark the row as selected by host-owned state and paint dense-row chrome.
    pub fn selected(mut self, selected: bool) -> Self {
        self.visual_state.selected = selected;
        self.dense_chrome = true;
        self
    }

    /// Mark the row as a committed operation target and paint dense-row chrome.
    pub fn active_target(mut self, active_target: bool) -> Self {
        self.visual_state.active_target = active_target;
        self.dense_chrome = true;
        self
    }

    /// Mark the row as a valid operation candidate and paint dense-row chrome.
    pub fn candidate(mut self, candidate: bool) -> Self {
        self.visual_state.candidate = candidate;
        self.dense_chrome = true;
        self
    }

    /// Apply host-owned visual state and paint dense-row chrome.
    pub fn visual_state(mut self, parts: InteractiveRowVisualStateParts) -> Self {
        self.visual_state = parts;
        self.dense_chrome = true;
        self
    }

    /// Assign a stable widget id to the backing interactive row.
    pub fn input_id(mut self, id: WidgetId) -> Self {
        self.input_id = Some(id);
        self.input_key = None;
        self
    }

    /// Assign a stable key to the backing interactive row.
    ///
    /// Use this when a dynamic row should refresh retained widget state based
    /// on caller-owned projection state but does not need a public numeric id.
    pub fn input_key(mut self, key: impl Into<String>) -> Self {
        self.input_key = Some(key.into());
        self.input_id = None;
        self
    }

    /// Derive and assign a stable input widget id from a caller-owned text key.
    ///
    /// Use this for dynamic rows whose focus, hover, drag, or drop identity
    /// should survive projection changes without app-local input-id helpers.
    pub fn stable_input_id(mut self, scope: u64, key: impl AsRef<str>) -> Self {
        self.input_id = Some(stable_widget_id(scope, key));
        self.input_key = None;
        self
    }

    /// Assign one stable row key to the composed row and its backing input widget.
    ///
    /// Use this for dynamic rows whose outer view subtree and retained input
    /// widget should follow the same caller-owned row identity. Prefer
    /// [`Self::input_id`] or [`Self::input_key`] plus a view-node key only when
    /// those identities deliberately differ.
    pub fn stable_row_identity(mut self, input_scope: u64, row_key: impl Into<String>) -> Self {
        let row_key = row_key.into();
        self.input_id = Some(stable_widget_id(input_scope, row_key.as_str()));
        self.input_key = None;
        self.row_key = Some(row_key);
        self
    }

    /// Derive and assign a stable input widget id from a caller-owned numeric key.
    ///
    /// Use this for dynamic rows keyed by durable numeric IDs or enum indexes
    /// without allocating temporary strings.
    pub fn stable_u64_input_id(mut self, scope: u64, key: u64) -> Self {
        self.input_id = Some(stable_widget_id_u64(scope, key));
        self.input_key = None;
        self
    }

    /// Apply an explicit style to the backing interactive row.
    pub fn style(mut self, style: WidgetStyle) -> Self {
        self.style = Some(style);
        self
    }

    /// Override dense-row fill colors for this underlay.
    pub fn dense_chrome_palette(mut self, palette: DenseRowPalette) -> Self {
        self.chrome.palette = Some(palette);
        self.dense_chrome = true;
        self
    }

    /// Add a leading-edge marker to the dense-row underlay chrome.
    pub fn leading_marker(mut self, marker: DenseRowMarkerStyle) -> Self {
        self.chrome.leading_marker = Some(marker);
        self.dense_chrome = true;
        self
    }

    /// Add a leading-edge marker when `condition` is true.
    pub fn leading_marker_if(mut self, condition: bool, marker: DenseRowMarkerStyle) -> Self {
        if condition {
            self.chrome.leading_marker = Some(marker);
            self.dense_chrome = true;
        }
        self
    }

    /// Add a trailing-edge marker to the dense-row underlay chrome.
    pub fn trailing_marker(mut self, marker: DenseRowMarkerStyle) -> Self {
        self.chrome.trailing_marker = Some(marker);
        self.dense_chrome = true;
        self
    }

    /// Add a trailing-edge marker when `condition` is true.
    pub fn trailing_marker_if(mut self, condition: bool, marker: DenseRowMarkerStyle) -> Self {
        if condition {
            self.chrome.trailing_marker = Some(marker);
            self.dense_chrome = true;
        }
        self
    }

    /// Add an inset outline to the dense-row underlay chrome.
    pub fn outline(mut self, outline: DenseRowOutlineStyle) -> Self {
        self.chrome.outline = Some(outline);
        self.dense_chrome = true;
        self
    }

    /// Add an inset outline when `condition` is true.
    pub fn outline_if(mut self, condition: bool, outline: DenseRowOutlineStyle) -> Self {
        if condition {
            self.chrome.outline = Some(outline);
            self.dense_chrome = true;
        }
        self
    }

    /// Emit mapped host messages for row interactions.
    pub fn mapped(
        self,
        map: impl Fn(InteractiveRowMessage) -> Message + Send + Sync + 'static,
    ) -> ViewNode<Message> {
        self.finish(WidgetMessageMapper::interactive_row(map))
    }

    /// Emit host messages for selected row interactions.
    pub fn filter_mapped(
        self,
        map: impl Fn(InteractiveRowMessage) -> Option<Message> + Send + Sync + 'static,
    ) -> ViewNode<Message> {
        self.finish(WidgetMessageMapper::interactive_row_filtered(map))
    }

    /// Emit host messages for common row actions.
    pub fn actions(mut self, actions: InteractiveRowActions<Message>) -> ViewNode<Message> {
        if actions.routes_hover() {
            self.row = self.row.hover_messages(true);
        }
        self.filter_mapped(move |message| actions.route(message))
    }

    fn finish(self, messages: WidgetMessageMapper<Message>) -> ViewNode<Message> {
        let Self {
            content,
            row,
            input_id,
            input_key,
            row_key,
            style,
            visual_state,
            chrome,
            dense_chrome,
        } = self;
        let row = row.widget();
        let mut input = if dense_chrome {
            view_node_from_widget(MappedWidget::new(
                DenseInteractiveRowUnderlayWidget {
                    row,
                    visual_state,
                    chrome,
                },
                messages,
            ))
        } else {
            view_node_from_widget(MappedWidget::new(row, messages))
        };
        if let Some(id) = input_id {
            input = input.id(id);
        } else if let Some(key) = input_key {
            input = input.key(key);
        }
        if let Some(style) = style {
            input = input.style(style);
        }
        let mut row = input_underlay(content, input);
        if let Some(row_key) = row_key {
            row = row.key(row_key);
        }
        row
    }
}

#[derive(Clone)]
struct DenseInteractiveRowUnderlayWidget {
    row: InteractiveRowWidget,
    visual_state: InteractiveRowVisualStateParts,
    chrome: DenseInteractiveRowUnderlayChrome,
}

#[derive(Clone, Copy, Debug, Default)]
struct DenseInteractiveRowUnderlayChrome {
    palette: Option<DenseRowPalette>,
    leading_marker: Option<DenseRowMarkerStyle>,
    trailing_marker: Option<DenseRowMarkerStyle>,
    outline: Option<DenseRowOutlineStyle>,
}

impl DenseInteractiveRowUnderlayChrome {
    fn palette(self, row: &InteractiveRowWidget, theme: &ThemeTokens) -> DenseRowPalette {
        let palette = self
            .palette
            .unwrap_or_else(|| dense_row_palette_from_style(theme, row.common.style));
        if row.paints_interaction_fill() {
            palette
        } else {
            palette.without_interaction_fills()
        }
    }

    fn apply_to(self, mut chrome: DenseRowChromeParts) -> DenseRowChromeParts {
        chrome.leading_marker = self.leading_marker;
        chrome.trailing_marker = self.trailing_marker;
        chrome.outline = self.outline;
        chrome
    }
}

impl Widget for DenseInteractiveRowUnderlayWidget {
    fn common(&self) -> &crate::widgets::WidgetCommon {
        self.row.common()
    }

    fn common_mut(&mut self) -> &mut crate::widgets::WidgetCommon {
        self.row.common_mut()
    }

    fn handle_input(&mut self, bounds: Rect, input: WidgetInput) -> Option<WidgetOutput> {
        self.row
            .handle_input(bounds, input)
            .map(WidgetOutput::typed)
    }

    fn accepts_pointer_move(&self) -> bool {
        self.row.accepts_pointer_move()
    }

    fn synchronize_from_previous(&mut self, previous: &dyn Widget) {
        if let Some(previous) = previous.as_any().downcast_ref::<Self>() {
            self.row.synchronize_from_previous(&previous.row);
        } else if let Some(previous) = previous.as_any().downcast_ref::<InteractiveRowWidget>() {
            self.row.synchronize_from_previous(previous);
        }
    }

    fn append_paint(
        &self,
        primitives: &mut Vec<PaintPrimitive>,
        bounds: Rect,
        _layout: &LayoutOutput,
        theme: &ThemeTokens,
    ) {
        let palette = self.chrome.palette(&self.row, theme);
        let chrome = self.row.dense_chrome_parts(self.visual_state, palette);
        let chrome = self.chrome.apply_to(chrome);
        self.row.push_dense_chrome(primitives, bounds, chrome);
    }
}

/// Build arbitrary visible content backed by an interactive row underlay.
///
/// The content remains visible above the row, while the backing row owns
/// activation, secondary activation, drag, drop, focus, and row feedback paint.
pub fn interactive_row_underlay<Message: 'static>(
    content: ViewNode<Message>,
) -> InteractiveRowUnderlayBuilder<Message> {
    InteractiveRowUnderlayBuilder {
        content,
        row: interactive_row(),
        input_id: None,
        input_key: None,
        row_key: None,
        style: None,
        visual_state: InteractiveRowVisualStateParts::default(),
        chrome: DenseInteractiveRowUnderlayChrome::default(),
        dense_chrome: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{application::text, widgets::Widget};

    #[test]
    fn dense_row_policy_preserves_drag_session_motion_after_drop_candidate_policy() {
        let builder = interactive_row_underlay::<()>(text("Sample")).dense_row_policy(
            DenseRowPolicy::new()
                .drag_session_motion(true)
                .tracked_drop_candidate(true, false, false, false),
        );
        let row = builder.row.widget();

        assert!(row.props.drag_active);
        assert!(row.props.droppable);
        assert!(!row.props.drop_hover);
        assert!(!row.props.clear_drop_on_hover);
        assert!(row.props.pointer_motion_active);
        assert!(row.accepts_pointer_move());
    }
}
