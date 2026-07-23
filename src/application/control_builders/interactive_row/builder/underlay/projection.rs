use super::super::defaults::interactive_row;
use super::InteractiveRowUnderlayBuilder;
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
        InteractiveRowWidget, Widget, WidgetInput, WidgetOutput,
    },
};

impl<Message: 'static> InteractiveRowUnderlayBuilder<Message> {
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
pub(super) struct DenseInteractiveRowUnderlayWidget {
    row: InteractiveRowWidget,
    visual_state: InteractiveRowVisualStateParts,
    chrome: DenseInteractiveRowUnderlayChrome,
}

#[derive(Clone, Copy, Debug, Default)]
pub(super) struct DenseInteractiveRowUnderlayChrome {
    pub(super) palette: Option<DenseRowPalette>,
    pub(super) leading_marker: Option<DenseRowMarkerStyle>,
    pub(super) leading_overlay_marker: Option<DenseRowMarkerStyle>,
    pub(super) pressed_leading_overlay_marker: Option<DenseRowMarkerStyle>,
    pub(super) trailing_marker: Option<DenseRowMarkerStyle>,
    pub(super) hover_trailing_marker: Option<DenseRowMarkerStyle>,
    pub(super) outline: Option<DenseRowOutlineStyle>,
    pub(super) pressed_outline: Option<DenseRowOutlineStyle>,
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
        chrome.leading_overlay_marker = if chrome.state.pressed {
            self.pressed_leading_overlay_marker
                .or(self.leading_overlay_marker)
        } else {
            self.leading_overlay_marker
        };
        chrome.trailing_marker = self.trailing_marker;
        if chrome.state.hovered && chrome.trailing_marker.is_none() {
            chrome.trailing_marker = self.hover_trailing_marker;
        }
        chrome.outline = if chrome.state.pressed {
            self.pressed_outline.or(self.outline)
        } else {
            self.outline
        };
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
