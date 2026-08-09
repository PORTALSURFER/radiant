use crate::{
    application::{
        MappedWidget, ViewNode, default_text_input_sizing, primary_style, view_node_from_widget,
    },
    layout::Vector2,
    runtime::WidgetMessageMapper,
    widgets::{
        NumericAdjustment, NumericCodec, NumericInputConstructionError, NumericInputEditBatch,
        NumericInputWidget, TextInputChrome, WidgetProminence, WidgetSizing, WidgetStyle,
    },
};

/// Builder for a generic numeric input with retained text editing.
pub struct NumericInputBuilder<T, C, A> {
    input: NumericInputWidget<T, C, A>,
    style: Option<WidgetStyle>,
}

impl<T, C, A> NumericInputBuilder<T, C, A>
where
    T: Clone + PartialEq + 'static,
    C: NumericCodec<T> + 'static,
    A: NumericAdjustment<T> + 'static,
{
    /// Apply an explicit widget style before binding this numeric input.
    pub fn style(mut self, style: WidgetStyle) -> Self {
        self.style = Some(style);
        self
    }

    /// Use the accent tone and strong prominence.
    pub fn primary(self) -> Self {
        self.style(primary_style())
    }

    /// Use a lower-prominence treatment.
    pub fn subtle(mut self) -> Self {
        let mut style = self.style.unwrap_or_default();
        style.prominence = WidgetProminence::Subtle;
        self.style = Some(style);
        self
    }

    /// Use compact toolbar-friendly numeric-input sizing.
    pub fn compact(mut self) -> Self {
        self.input
            .set_sizing(WidgetSizing::fixed(Vector2::new(92.0, 20.0)));
        self
    }

    /// Use a minimal underline-only input chrome.
    pub fn underline(mut self) -> Self {
        self.input.set_chrome(TextInputChrome::Underline);
        self
    }

    /// Set the initial selection anchor and caret.
    pub fn selection(mut self, anchor: usize, caret: usize) -> Self {
        self.input.set_selection(anchor, caret);
        self
    }

    /// Select the full canonical editable value.
    pub fn select_all(mut self) -> Self {
        self.input.select_all();
        self
    }

    /// Emit a host message for the complete ordered numeric edit lifecycle.
    pub fn on_edit<Message: 'static>(
        self,
        map: impl Fn(NumericInputEditBatch<T>) -> Message + 'static,
    ) -> ViewNode<Message> {
        let mut node = view_node_from_widget(MappedWidget::new(
            self.input,
            WidgetMessageMapper::typed(map),
        ));
        node.style = self.style;
        node
    }
}

/// Construct a generic text-first numeric input.
#[allow(clippy::type_complexity)]
pub fn numeric_input<T, C, A>(
    value: T,
    codec: C,
    adjustment: A,
) -> Result<NumericInputBuilder<T, C, A>, NumericInputConstructionError<C::Error, A::Error>>
where
    T: Clone + PartialEq + 'static,
    C: NumericCodec<T> + 'static,
    A: NumericAdjustment<T> + 'static,
{
    NumericInputWidget::try_new(value, codec, adjustment, default_text_input_sizing())
        .map(|input| NumericInputBuilder { input, style: None })
}
