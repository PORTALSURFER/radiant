use crate::{
    application::{MappedWidget, ViewNode, default_text_input_sizing, view_node_from_widget},
    runtime::WidgetMessageMapper,
    widgets::{
        TextEditorEdit, TextEditorSnapshot, TextEditorWidget, TextEditorWidgetParts, WidgetId,
    },
};

/// Builder for a controlled multi-line editor backed by an application document snapshot.
pub struct TextEditorBuilder {
    snapshot: TextEditorSnapshot,
    id: WidgetId,
    wrap: bool,
    font_size: f32,
}

impl TextEditorBuilder {
    /// Set a stable widget identity for geometry receipts and input routing.
    pub fn id(mut self, id: WidgetId) -> Self {
        self.id = id;
        self
    }

    /// Toggle soft wrapping inside the assigned view-node bounds.
    pub fn wrap(mut self, wrap: bool) -> Self {
        self.wrap = wrap;
        self
    }

    /// Set the unscaled editor font size when it is finite and within the safe UI range.
    pub fn font_size(mut self, font_size: f32) -> Self {
        if font_size.is_finite() && (1.0..=512.0).contains(&font_size) {
            self.font_size = font_size;
        }
        self
    }

    /// Map exact application-owned document edits into ordinary host messages.
    pub fn message<Message: 'static>(
        self,
        map: impl Fn(TextEditorEdit) -> Message + 'static,
    ) -> ViewNode<Message> {
        let mut widget = TextEditorWidget::from_parts(TextEditorWidgetParts {
            id: self.id,
            snapshot: self.snapshot,
            sizing: default_text_input_sizing(),
        });
        widget.wrap = self.wrap;
        widget.font_size = self.font_size;
        view_node_from_widget(MappedWidget::new(widget, WidgetMessageMapper::typed(map)))
    }
}

/// Start a controlled multi-line editor from the application's exact document snapshot.
pub fn text_editor(snapshot: TextEditorSnapshot) -> TextEditorBuilder {
    TextEditorBuilder {
        snapshot,
        id: 0,
        wrap: true,
        font_size: 14.0,
    }
}
