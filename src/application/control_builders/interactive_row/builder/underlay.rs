use crate::{
    application::{MappedWidget, ViewNode, input_underlay, view_node_from_widget},
    gui::{list::dense_row_palette_from_style, types::Rect},
    layout::LayoutOutput,
    runtime::{PaintPrimitive, WidgetMessageMapper},
    theme::ThemeTokens,
    widgets::{
        InteractiveRowActions, InteractiveRowMessage, InteractiveRowVisualStateParts,
        InteractiveRowWidget, Widget, WidgetId, WidgetInput, WidgetOutput, WidgetStyle,
        stable_widget_id, stable_widget_id_u64,
    },
};

use super::{InteractiveRowBuilder, defaults::interactive_row};

/// Builder for arbitrary row content backed by a generic interactive row.
pub struct InteractiveRowUnderlayBuilder<Message> {
    content: ViewNode<Message>,
    row: InteractiveRowBuilder,
    input_id: Option<WidgetId>,
    style: Option<WidgetStyle>,
    visual_state: InteractiveRowVisualStateParts,
    dense_chrome: bool,
}

impl<Message: 'static> InteractiveRowUnderlayBuilder<Message> {
    /// Configure the backing interactive row before binding messages.
    pub fn row(
        mut self,
        configure: impl FnOnce(InteractiveRowBuilder) -> InteractiveRowBuilder,
    ) -> Self {
        self.row = configure(self.row);
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
        self
    }

    /// Derive and assign a stable input widget id from a caller-owned text key.
    ///
    /// Use this for dynamic rows whose focus, hover, drag, or drop identity
    /// should survive projection changes without app-local input-id helpers.
    pub fn stable_input_id(mut self, scope: u64, key: impl AsRef<str>) -> Self {
        self.input_id = Some(stable_widget_id(scope, key));
        self
    }

    /// Derive and assign a stable input widget id from a caller-owned numeric key.
    ///
    /// Use this for dynamic rows keyed by durable numeric IDs or enum indexes
    /// without allocating temporary strings.
    pub fn stable_u64_input_id(mut self, scope: u64, key: u64) -> Self {
        self.input_id = Some(stable_widget_id_u64(scope, key));
        self
    }

    /// Apply an explicit style to the backing interactive row.
    pub fn style(mut self, style: WidgetStyle) -> Self {
        self.style = Some(style);
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
    pub fn actions(self, actions: InteractiveRowActions<Message>) -> ViewNode<Message> {
        self.filter_mapped(move |message| actions.route(message))
    }

    fn finish(self, messages: WidgetMessageMapper<Message>) -> ViewNode<Message> {
        let Self {
            content,
            row,
            input_id,
            style,
            visual_state,
            dense_chrome,
        } = self;
        let row = row.widget();
        let mut input = if dense_chrome {
            view_node_from_widget(MappedWidget::new(
                DenseInteractiveRowUnderlayWidget { row, visual_state },
                messages,
            ))
        } else {
            view_node_from_widget(MappedWidget::new(row, messages))
        };
        if let Some(id) = input_id {
            input = input.id(id);
        }
        if let Some(style) = style {
            input = input.style(style);
        }
        input_underlay(content, input)
    }
}

#[derive(Clone)]
struct DenseInteractiveRowUnderlayWidget {
    row: InteractiveRowWidget,
    visual_state: InteractiveRowVisualStateParts,
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
        let palette = dense_row_palette_from_style(theme, self.row.common.style);
        let palette = if self.row.paints_interaction_fill() {
            palette
        } else {
            palette.without_interaction_fills()
        };
        let chrome = self.row.dense_chrome_parts(self.visual_state, palette);
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
        style: None,
        visual_state: InteractiveRowVisualStateParts::default(),
        dense_chrome: false,
    }
}
